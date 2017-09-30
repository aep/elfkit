extern crate elfkit;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf,types,SectionContent, Section};
use elfkit::linker::Linker;

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();

    in_elf.load_all().unwrap();

    let mut out_elf = Elf::default();

    out_elf.header.ident_class  = in_elf.header.ident_class;
    out_elf.header.ident_abi    = in_elf.header.ident_abi;
    out_elf.header.etype        = in_elf.header.etype;
    out_elf.header.machine      = in_elf.header.machine;
    out_elf.header.entry        = in_elf.header.entry;
    out_elf.header.shstrndx     = in_elf.header.shstrndx;

    // sections which do not have an ALLOC flag aren't needed by the dynamic linker
    // but also keep the first NULL section
    out_elf.sections = in_elf.sections;

    let mut i = 0;
    loop {
        let keep = {
            let sec = &out_elf.sections[i];
            (sec.header.flags.contains(types::SectionFlags::ALLOC) ||
             sec.header.shtype == types::SectionType::NULL) && match (sec.name.as_ref() as &str, &sec.header.shtype) {
                (_, &types::SectionType::NOTE) => false,
                (_, &types::SectionType::GNU_HASH) => false,
                //(_, &types::SectionType::GNU_VERSYM) => false,
                //(_, &types::SectionType::GNU_VERNEED) => false,
                (".eh_frame_hdr", &types::SectionType::PROGBITS) => false,
                (".eh_frame", &types::SectionType::PROGBITS) => false,
                (".gcc_except_table", &types::SectionType::PROGBITS) => false,
                _ => true,
            }
        };
        if keep {
            i += 1;
        } else {
            out_elf.remove_section(i).unwrap();
        }
        if i >= out_elf.sections.len() {
            break
        }
    }

    for sec in &mut out_elf.sections {
        match sec.content {
            SectionContent::Dynamic(ref mut dyn) => {
                dyn.retain(|dyn| {
                    match dyn.dhtype {
                        //types::DynamicType::VERNEED |
                        //    types::DynamicType::VERNEEDNUM |
                        //    types::DynamicType::VERSYM |
                            types::DynamicType::GNU_HASH => false,
                        _ => true,
                    }
                });
            },
            _ => {},
        }
    }

    out_elf.store_all().unwrap();

    //move the first sections out of the way to make room for a larger program header
    for _ in 0..2 {
        let to = out_elf.sections.len() - 1;
        let f  = out_elf.move_section(1, to).unwrap();
    }

    let off = out_elf.sections[1].header.offset;
    out_elf.relayout(off).unwrap();


    out_elf.segments = Linker::segments(&out_elf).unwrap();


    out_elf.to_writer(&mut out_file).unwrap();
}

