extern crate elfkit;
extern crate colored;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf,Section};
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

    out_elf.write_start(&mut out_file);

    for section in &in_elf.sections {
        match section.name.as_ref() {
            ".text" => {
                in_file.seek(SeekFrom::Start(section.offset)).unwrap();
                let mut off = out_file.seek(SeekFrom::Current(0)).unwrap();

                out_elf.sections.push(Section{
                    name:       String::from(".text"),
                    shtype:     section.shtype.clone(),
                    flags:      section.flags,
                    addr:       section.addr,
                    offset:     off,
                    size:       section.size,
                    link:       section.link,
                    info:       section.info,
                    addralign:  section.addralign,
                    entsize:    section.entsize,

                    _name: 0,
                });

                copy(&mut (&in_file).take(section.size), &mut out_file).unwrap();
            },
            &_ => {},
        }
    }

    out_elf.write_end(&mut out_file);
}

