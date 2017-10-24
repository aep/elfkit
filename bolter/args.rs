use std::path::Path;
use std::env;
use ::fail;
use elfkit::*;
use std::fs::OpenOptions;
use ::goblin;
use std::io::{Read, Cursor};
use colored::*;


#[derive(Default)]
pub struct LdOptions {
    pub dynamic_linker: String,
    pub object_paths:   Vec<String>,
    pub output_path:    String,
}

fn search_lib(search_paths: &Vec<String>, needle: &String) -> String{
    let so = String::from("lib") + needle + ".a";
    for p in search_paths {
        let pc = Path::new(p).join(&so);
        if pc.exists() {
            return pc.into_os_string().into_string().unwrap();
        }
    }
    fail(format!("ld.elfkit: cannot find: {} in {:?}", so, search_paths));
}

fn ldarg(arg: &String, argname: &str, argc: &mut usize) -> Option<String> {
    if arg.starts_with(argname) {
        Some(if arg.len() < argname.len() + 1 {
            *argc += 1;
            env::args().nth(*argc).unwrap()
        } else {
            String::from(&arg[2..])
        })
    } else {
        None
    }
}

pub fn parse_ld_options() -> LdOptions{
    let mut options         = LdOptions::default();
    options.output_path     = String::from("a.out");
    let mut search_paths    = Vec::new();

    let mut argc = 1;
    loop {
        if argc >= env::args().len() {
            break;
        }

        let arg = env::args().nth(argc).unwrap();
        if let Some(val) = ldarg(&arg, "-L", &mut argc) {
            search_paths.push(val);
        } else if let Some(val) = ldarg(&arg, "-z", &mut argc) {
            argc += 1;
            let arg2 = env::args().nth(argc).unwrap();
            println!("{}", format!("argument ignored: {} {}", arg,arg2).yellow());

        } else if let Some(val) = ldarg(&arg, "-l", &mut argc) {
            options.object_paths.push(search_lib(&search_paths, &val));
        } else if let Some(val) = ldarg(&arg, "-m", &mut argc) {
            if val != "elf_x86_64" {
                fail(format!("machine not supported: {}", val));
            }
        } else if let Some(val) = ldarg(&arg, "-o", &mut argc) {
            options.output_path = val;
        } else if arg == "-pie" {
        } else if arg == "-dynamic-linker" {
            argc += 1;
            options.dynamic_linker = env::args().nth(argc).unwrap()
        } else if arg.starts_with("-") {
            println!("{}", format!("argument ignored: {}",arg).yellow());
        } else {
            options.object_paths.push(arg);
        }
        argc +=1;
    }

    println!("linking {:?}", options.object_paths);

    options
}

pub fn load_elfs(paths: Vec<String>) -> Vec<(String,Elf)> {
    let mut elfs = Vec::new();
    for in_path in paths {
        let mut in_file  = match OpenOptions::new().read(true).open(&in_path) {
            Ok(f) => f,
            Err(e) => {
                fail(format!("while loading '{}' : {:?}", in_path, e));
            }
        };
        let in_name = Path::new(&in_path).file_name().unwrap().to_string_lossy().into_owned();
        match filetype::filetype(&in_file).unwrap() {
            filetype::FileType::Elf => {
                elfs.push((in_name, match Elf::from_reader(&mut in_file) {
                    Ok(e) => e,
                    Err(e) => {
                        fail(format!("error loading {} : {:?}",
                                               in_path, e));
                    },

                }));
            },
            filetype::FileType::Archive => {
                let mut buffer = Vec::new();
                in_file.read_to_end(&mut buffer).unwrap();
                match goblin::Object::parse(&buffer).unwrap() {
                    goblin::Object::Archive(archive) => {
                        for (name,member,_) in archive.summarize() {
                            let mut io = Cursor::new(&buffer[
                                                     member.offset as usize ..
                                                     member.offset as usize + member.header.size]);

                            match Elf::from_reader(&mut io) {
                                Ok(e)  => elfs.push((String::from(name), e)),
                                Err(e) => {
                                    println!("{}", format!("skipping {} in {}: {:?}",
                                                     name, in_path, e).yellow());
                                },
                            }
                        }
                    },
                    _ => unreachable!(),
                }
            },
            _ => {
                fail(format!("{}: unknown file type", in_name));
            }
        }
    }
    elfs
}
