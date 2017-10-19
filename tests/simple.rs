extern crate elfkit;

use std::env;
use std::fs::File;
use elfkit::Elf;
use elfkit::symbol::Symbol;
use std::io::{Read, Seek, SeekFrom};


#[test]
fn amd64_exe_sections() {
    let mut file = File::open("./tests/samples/amd64_pie_asm").unwrap();
    let elf = Elf::from_reader(&mut file).unwrap();

    let secnames: Vec<&str> = elf.sections.iter().map(|s| s.name.as_ref()).collect();
    assert!(secnames.contains(&".text"));
    assert!(secnames.contains(&".data"));
    assert!(secnames.contains(&".shstrtab"));



    assert!(true);
}
