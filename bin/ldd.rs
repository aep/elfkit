extern crate glob;
extern crate clap;
extern crate elfkit;

use std::fs::{File};
use elfkit::Elf;
use std::path::Path;
use std::collections::HashSet;
use std::io::{self};
use std::io::BufReader;
use std::io::BufRead;
use glob::glob;

struct Ldd {
    sysroot:    String,
    lpaths:     Vec<String>,
    visited:    HashSet<String>,
}

fn join_paths(a: &str, b: &str) -> String {
    if b.len() < 1 {
        return String::from(a);
    }
    let mut a = String::from(a);
    if a.len() < 1 {
        return a;
    }
    if a.chars().last().unwrap() != '/' {
        a.push('/');
    }

    if b.chars().nth(0).unwrap() == '/' {
        return a + &b[1..];
    }
    return a + b;
}

impl Ldd {
    fn recurse(&mut self, path: &str) {
        let mut f = File::open(path).unwrap();
        let mut elf = Elf::from_reader(&mut f).unwrap();

        let mut deps = Vec::new();
        for shndx in 0..elf.sections.len() {
            if elf.sections[shndx].header.shtype == elfkit::types::SectionType::DYNAMIC {
                elf.load(shndx, &mut f).unwrap();
                let dynamic = elf.sections[shndx].content.as_dynamic().unwrap();

                for dyn in dynamic.iter() {
                    if dyn.dhtype == elfkit::types::DynamicType::RPATH{
                        if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                            self.lpaths.push(join_paths(
                                    &self.sysroot, &String::from_utf8_lossy(&name.0).into_owned()))
                        }
                    }
                    if dyn.dhtype == elfkit::types::DynamicType::NEEDED {
                        if let elfkit::dynamic::DynamicContent::String(ref name) = dyn.content {
                            deps.push(String::from_utf8_lossy(&name.0).into_owned());
                        }
                    }
                }
            }
        }

        for dep in &mut deps {
            let mut found = false;
            for lpath in self.lpaths.iter() {
                let joined = join_paths(&lpath, &dep);
                let joined = Path::new(&joined);
                if joined.exists() {
                    *dep = joined.to_string_lossy().into_owned();
                    found = true;
                    break;
                }
            }
            if found {
                if self.visited.insert(dep.clone()) {
                    println!("{}", dep);
                    self.recurse(dep);
                }
            } else {
                panic!("unable to find dependcy {} in {:?}", dep, self.lpaths);
            }
        }
    }
}

fn parse_ld_so_conf(path: &str) -> io::Result<Vec<String>> {
    let mut paths = Vec::new();

    let f = File::open(path)?;
    let f = BufReader::new(&f);
    for line in f.lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with("#") {
            continue;
        }
        if line == "" {
            continue;
        }

        if line.contains(" ") {
            if line.starts_with("include ") {
                for entry in glob(line.split(" ").last().unwrap()).expect("Failed to read glob pattern") {
                    paths.extend(parse_ld_so_conf(&entry.unwrap().to_string_lossy().into_owned())?);
                }

            }
        } else {
            paths.push(line.to_owned());
        }
    }
    Ok(paths)
}

fn main() {
    let matches = clap::App::new("elfkit-ldd")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .setting(clap::AppSettings::UnifiedHelpMessage)
        .setting(clap::AppSettings::DisableHelpSubcommand)
        .version("0.5")
        .arg(clap::Arg::with_name("file")
             .required(true)
             .help("path to binary to inspect")
             .takes_value(true)
             .index(1)
            )
        .arg(clap::Arg::with_name("library-path")
             .short("L")
             .long("library-path")
             .takes_value(true)
             .multiple(true)
             .help("library lookup path, ignores $SYSROOT/etc/ld.so.conf")
             )
        .arg(clap::Arg::with_name("sysroot")
             .short("R")
             .long("sysroot")
             .takes_value(true)
             .help("specify sysroot to look up dependencies in, instead of /")
             )
        .get_matches();



    let sysroot = matches.value_of("sysroot").unwrap_or("/").to_owned();

    let lpaths = match matches.values_of("library-path") {
        None => {
            match parse_ld_so_conf(&(sysroot.clone() + "/etc/ld.so.conf")) {
                Ok(l) => l.into_iter().map(|l| join_paths(&sysroot, &l)).collect(),
                Err(_) => vec![join_paths(&sysroot, "/lib"), join_paths(&sysroot, "/usr/lib")],
            }
        },
        Some(vals) => {
            vals.map(|v|v.to_owned()).collect()
        }
    };


    let mut ldd = Ldd{
        sysroot:    sysroot,
        lpaths:     lpaths,
        visited:    HashSet::new(),
    };

    let path  = matches.value_of("file").unwrap();
    ldd.recurse(&path);
}


