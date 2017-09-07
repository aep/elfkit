extern crate elfkit;
extern crate colored;

use std::env;
use std::fs::OpenOptions;
use elfkit::Elf;
use elfkit::relocation::{Relocation};
use elfkit::symbol::{Symbol};
use elfkit::types;
use std::io::{Read, Seek, SeekFrom};
use colored::*;

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut elf  = Elf::from_reader(&mut in_file).unwrap();
    elf.to_writer(&mut out_file).unwrap();
}

