extern crate elfkit;
extern crate tempfile;

use std::fs::File;
use elfkit::Elf;
use std::io::{Write};
use std::process::Command;

fn build_host_code(code: &[u8]) -> String {
    let mut fo = tempfile::NamedTempFile::new().unwrap();
    fo.write(code);

    let path = fo.path().to_string_lossy().into_owned();

    assert!(Command::new("gcc").args(&["-c", "-x", "c", &path.clone(), "-o", &(path.clone() + ".o")])
            .status().unwrap().success());

    path + ".o"
}

#[test]
fn simple_sections() {
    let ofile = build_host_code(b"int main() {return 42;}");
    let mut file = File::open(&ofile).unwrap();
    std::fs::remove_file(ofile);
    let elf = Elf::from_reader(&mut file).unwrap();

    let secnames: Vec<&str> = elf.sections.iter().map(|s| s.name.as_ref()).collect();
    assert!(secnames.contains(&".text"));
    assert!(secnames.contains(&".symtab"));

    assert!(true);
}
