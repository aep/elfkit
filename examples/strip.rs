extern crate elfkit;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf,types};

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();

    // de/serialize all known section types to detailed representation
    // this isn't nessesary for strip, because it never touches the section content
    // we just do this for demonstration purposes
    in_elf.load_all().unwrap();

    let mut out_elf = Elf::default();

    out_elf.header.ident_class  = in_elf.header.ident_class;
    out_elf.header.ident_abi    = in_elf.header.ident_abi;
    out_elf.header.etype        = in_elf.header.etype;
    out_elf.header.machine      = in_elf.header.machine;
    out_elf.header.entry        = in_elf.header.entry;
    out_elf.header.shstrndx     = in_elf.header.shstrndx;

    out_elf.segments = in_elf.segments.clone();

    // sections which do not have an ALLOC flag aren't needed by the dynamic linker
    // but also keep the first NULL section
    out_elf.sections = in_elf.sections.drain(..).filter(|sec|{
        sec.header.flags.contains(types::SectionFlags::ALLOC) ||
        sec.header.shtype == types::SectionType::NULL
    }).collect();
    //out_elf.sections = in_elf.sections;

    out_elf.store_all().unwrap();

    out_elf.to_writer(&mut out_file).unwrap();
}

