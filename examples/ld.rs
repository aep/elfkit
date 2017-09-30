extern crate elfkit;
#[macro_use] extern crate itertools;
use itertools::Itertools;

use std::env;
use std::fs::OpenOptions;
use elfkit::{
    Elf, types, SegmentHeader, Section, SectionContent,
    SectionHeader, Dynamic, Symbol, Relocation, Strtab};

use elfkit::linker::Linker;

use elfkit::dynamic::DynamicContent;
use std::collections::HashMap;




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

    //NOTE PIEs must be set to DYN, since ld behaves differently.
    //elfkit is not tested for EXEC and is unlikely going to produce a working exe
    out_elf.header.etype        = types::ElfType::DYN;
    out_elf.header.machine      = in_elf.header.machine;
    out_elf.header.entry        = 0x310;

    let mut sc_interp  : Vec<u8> = b"/lib64/ld-linux-x86-64.so.2\0".to_vec();
    //let mut sc_interp  : Vec<u8> = b"/usr/local/musl/lib/libc.so\0".to_vec();
    let mut sc_text    : Vec<u8> = Vec::new();
    let mut sc_data    : Vec<u8> = Vec::new();
    let mut sc_dynsym  : Vec<Symbol>  = Vec::new();
    let mut sc_rela    : Vec<Relocation>  = Vec::new();
    let mut sc_dynamic : Vec<Dynamic> = vec![
        Dynamic{
            dhtype: types::DynamicType::FLAGS_1,
            content: DynamicContent::Flags1(types::DynamicFlags1::PIE),
        },
        //Dynamic{
        //    dhtype: types::DynamicType::NEEDED,
        //    content: DynamicContent::String(String::from("libc.so.6")),
        //},
    ];

    for mut sec in &in_elf.sections {
        if sec.header.shtype == types::SectionType::PROGBITS && sec.name == ".text" {
            match sec.content {
                SectionContent::Raw(ref v) => sc_text.extend(v),
                _ => unreachable!(),
            }
        } else if sec.header.shtype == types::SectionType::PROGBITS && sec.name == ".data" {
            match sec.content {
                SectionContent::Raw(ref v) => sc_data.extend(v),
                _ => unreachable!(),
            }
        } else if sec.header.shtype == types::SectionType::RELA && sec.name == ".rela.text" {
            match sec.content {
                SectionContent::Relocations(ref v) => {
                    let v = v.clone();
                    for mut rel in v {
                        sc_rela.push(rel);
                    }
                }
                _ => unreachable!(),
            }
        } else if sec.header.shtype == types::SectionType::SYMTAB {
            match sec.content {
                SectionContent::Symbols(ref v) => {
                    let v = v.clone();
                    for mut sym in v {
                        if sym.shndx > 0 && (sym.shndx as usize) < in_elf.sections.len() {
                            match in_elf.sections[sym.shndx as usize].name.as_ref() {
                                ".text" => {
                                    sym.shndx = 2;
                                    sc_dynsym.push(sym);
                                },
                                ".data" => {
                                    sym.shndx = 3;
                                    sc_dynsym.push(sym);
                                },
                                _ => {},
                            }
                        }
                    }
                },
                _ => unreachable!(),
            }
        }
    }
    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(".interp", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));

    out_elf.sections.push(Section::new(".text", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC | types::SectionFlags::EXECINSTR,
                                       SectionContent::Raw(sc_text), 0,0));

    let sh_index_dynstr = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynstr", types::SectionType::STRTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Strtab(Strtab::default()), 0,0));

    //TODO should i maybe just make all symbols global? a dynlinker will probably not use local
    //syms anyway
    sc_dynsym.sort_unstable_by(|a,b| a.bind.cmp(&b.bind));
    let (first_global_dynsym,_) = sc_dynsym.iter().enumerate().find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).unwrap();;
    let sh_index_dynsym = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynsym", types::SectionType::SYMTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Symbols(sc_dynsym),
                                       sh_index_dynstr as u32, first_global_dynsym as u32));


    out_elf.sections.push(Section::new(".rela.dyn", types::SectionType::RELA,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Relocations(sc_rela),
                                       sh_index_dynsym as u32, 0));


    out_elf.sections.push(Section::new(".data", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                       SectionContent::Raw(sc_data), 0,0));

    out_elf.sections.push(Section::new(".shstrtab", types::SectionType::STRTAB,
                                       types::SectionFlags::from_bits_truncate(0),
                                       SectionContent::Strtab(Strtab::default()),
                                       0,0));


    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    let nd = Linker::dynamic(&out_elf).unwrap();
    //out_elf.sections[sh_index_dynamic].content.as_dynamic_mut().unwrap().extend(nd);
    sc_dynamic.extend(nd);
    let sh_index_dynamic = out_elf.sections.len() -1;
    out_elf.sections.insert(sh_index_dynamic, Section::new(".dynamic", types::SectionType::DYNAMIC,
                                       types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                       SectionContent::Dynamic(sc_dynamic), sh_index_dynstr as u32,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf,0x300).unwrap();
    out_elf.segments = Linker::segments(&out_elf).unwrap();


    out_elf.store_all().unwrap();
    out_elf.to_writer(&mut out_file).unwrap();
}

