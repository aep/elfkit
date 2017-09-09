extern crate elfkit;
extern crate colored;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf,Section,SectionHeader,SectionContent};
use elfkit::relocation::{Relocation};
use elfkit::symbol::{Symbol};
use elfkit::types;
use std::io::{Read, Seek, SeekFrom, copy};
use colored::*;

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();
    let mut out_elf = Elf::default();

    out_elf.header.ident_class  = in_elf.header.ident_class;
    out_elf.header.ident_abi    = in_elf.header.ident_abi;
    out_elf.header.etype        = in_elf.header.etype;
    out_elf.header.machine      = in_elf.header.machine;
    out_elf.header.entry        = in_elf.header.entry;

    out_elf.segments = in_elf.segments.clone();
    out_elf.sections = in_elf.sections.drain(..).filter(|sec|{
        !sec.name.starts_with(".debug")
    }).collect();

    out_elf.to_writer(&mut out_file).unwrap();
}

