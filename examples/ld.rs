#[macro_use] extern crate elfkit;
#[macro_use] extern crate itertools;
extern crate byteorder;

use itertools::Itertools;

use std::env;
use std::fs::OpenOptions;
use elfkit::{
    Elf, types, SegmentHeader, Section, SectionContent,
    SectionHeader, Dynamic, Symbol, Relocation, Strtab};

use elfkit::linker::Linker;

use elfkit::dynamic::DynamicContent;
use elfkit::relocation::RelocationType;
use std::collections::HashMap;



#[derive(Default)]
struct Unit {
    pub symbols: Vec<Symbol>,
    pub dynrel: Vec<Relocation>,
}

impl Unit {
    pub fn load(in_elf: &Elf, out_elf: &mut Elf) -> Result<Unit, elfkit::Error> {

        let mut symbols = Vec::new();
        let mut dynrel  = Vec::new();
        let mut sectionmap = HashMap::new();

        for (i, sec) in in_elf.sections.iter().enumerate() {
            if sec.header.flags.contains(types::SectionFlags::ALLOC) {
                let mut sec = sec.clone();
                //need writeble for reloc.
                //TODO we should probably use GNU_RELRO or something
                sec.header.flags.insert(types::SectionFlags::WRITE);

                sectionmap.insert(i as u16, out_elf.sections.len() as u16);
                out_elf.sections.push(sec);
            }
            match (sec.name.as_ref(), &sec.content) {
                (".symtab", &SectionContent::Symbols(ref v)) => {symbols.extend(v.iter().cloned());},
                _ => {},
            };
        }
        out_elf.sync_all().unwrap();
        Linker::relayout(out_elf, 0x300).unwrap();

        for sym in &mut symbols {
            if sym.shndx > 0 && sym.shndx < 65521 {
                sym.shndx = match sectionmap.get(&sym.shndx) {
                    Some(v) => *v,
                    None => 0,
                };
            }
        }

        for sec in in_elf.sections.iter() {
            if sec.header.shtype == types::SectionType::RELA {
                let (mut reloc_target,reloc_target_shndx) = match sectionmap.get(&(sec.header.info as u16)) {
                    Some(v) => (
                        std::mem::replace(&mut out_elf.sections[*v as usize], Section::default()),
                        *v as usize),
                    None => panic!(format!("error loading unit: section {} is not allocated, \
                                           referenced in reloc section {}", sec.header.info, sec.name)),
                };

                let symtab = match in_elf.sections[sec.header.link as usize].content {
                    SectionContent::Symbols(ref vv) => vv,
                    _ => return Err(elfkit::Error::LinkedSectionIsNotSymtab),
                };

                for reloc in sec.content.as_relocations().unwrap() {
                    match reloc.rtype {
                        RelocationType::R_X86_64_NONE => {},

                        // Symbol + Addend
                        RelocationType::R_X86_64_64   => {
                            let symbol  = &symtab[reloc.sym as usize];
                            let absaddr = symbol.value + out_elf.sections[
                                sectionmap[&symbol.shndx] as usize].header.addr;

                            let value = (
                                absaddr as i64 +
                                reloc.addend as i64)
                                as u64;

                            println!("delaying {:?} value {:x}", reloc, value);
                            dynrel.push(Relocation {
                                rtype:  RelocationType::R_X86_64_RELATIVE,
                                sym:    0,
                                addr:   reloc_target.header.addr + reloc.addr,
                                addend: value as i64,
                            });
                        },

                        //Symbol + Addend - Load address of the Global Offset Table
                        RelocationType::R_X86_64_GOTOFF64 => {
                            let symbol  = &symtab[reloc.sym as usize];
                            println!("relocating {:?} symbol {:?}", reloc, symbol);
                            let absaddr = symbol.value + out_elf.sections[
                                sectionmap[&symbol.shndx] as usize].header.addr;

                            let value = (
                                absaddr as i64 +
                                reloc.addend as i64)
                                as u64;

                            println!("relocating {:?} value {:x}", reloc, value);
                        },

                        //Symbol + Addend - relocation target section load address
                        RelocationType::R_X86_64_PC32   => {
                            let symbol  = &symtab[reloc.sym as usize];
                            let absaddr = symbol.value + out_elf.sections[
                                sectionmap[&symbol.shndx] as usize].header.addr;

                            let value = (
                                absaddr as i64 +
                                reloc.addend as i64 -
                                reloc_target.header.addr as i64 )
                                as u32;

                            println!("relocating {:?} value {:x}", reloc, value);
                            //let mut io = &mut reloc_target.content.as_raw_mut().unwrap()[reloc.addr..];
                            //elf_write_u32!(&out_elf.header, io, value);

                        },
                        _ => panic!(format!("relocation {:?} not implemented",reloc)),
                    }
                }

                out_elf.sections[reloc_target_shndx as usize] = reloc_target;

            }
        }

        Ok(Unit{
            symbols: symbols,
            dynrel:  dynrel,
        })
    }
}



fn main() {
    let out_filename = "/tmp/e";

    let in_filename  = env::args().nth(1).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();
    in_elf.load_all().unwrap();

    let mut out_elf = Elf::default();
    out_elf.header.ident_class  = in_elf.header.ident_class.clone();
    out_elf.header.ident_abi    = in_elf.header.ident_abi.clone();

    //NOTE PIEs must be set to DYN, since ld behaves differently.
    //elfkit is not tested for EXEC and is unlikely going to produce a working exe
    out_elf.header.etype        = types::ElfType::DYN;
    out_elf.header.machine      = in_elf.header.machine.clone();

    let mut sc_interp  : Vec<u8> = b"/lib64/ld-linux-x86-64.so.2\0".to_vec();
    //let mut sc_interp  : Vec<u8> = b"/usr/local/musl/lib/libc.so\0".to_vec();
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


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(".interp", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    let mut unit = Unit::load(&in_elf, &mut out_elf).unwrap();
    sc_dynsym.extend(unit.symbols.drain(..));
    sc_rela.extend(unit.dynrel.drain(..));


    let sh_index_dynstr = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynstr", types::SectionType::STRTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Strtab(Strtab::default()), 0,0));

    //TODO should i maybe just make all symbols global? a dynlinker will probably not use local
    //syms anyway
    sc_dynsym.sort_unstable_by(|a,b| a.bind.cmp(&b.bind));
    let first_global_dynsym = sc_dynsym.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    let sh_index_dynsym = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynsym", types::SectionType::SYMTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Symbols(sc_dynsym),
                                       sh_index_dynstr as u32, first_global_dynsym as u32));


    out_elf.sections.push(Section::new(".rela.dyn", types::SectionType::RELA,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Relocations(sc_rela),
                                       sh_index_dynsym as u32, 0));



    out_elf.sections.push(Section::new(".shstrtab", types::SectionType::STRTAB,
                                       types::SectionFlags::from_bits_truncate(0),
                                       SectionContent::Strtab(Strtab::default()),
                                       0,0));


    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    sc_dynamic.extend(Linker::dynamic(&out_elf).unwrap());
    let sh_index_dynamic = out_elf.sections.len() -1;
    out_elf.sections.insert(sh_index_dynamic, Section::new(".dynamic", types::SectionType::DYNAMIC,
                                                           types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                                           SectionContent::Dynamic(sc_dynamic), sh_index_dynstr as u32,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();


    //find the start sym
    for sec in &out_elf.sections {
        match (sec.name.as_ref(), &sec.content) {
            (".dynsym", &SectionContent::Symbols(ref v)) => {
                for sym in v{
                    if sym.name == "_start" {
                        out_elf.header.entry = out_elf.sections[sym.shndx as usize].header.addr + sym.value;
                    }
                }
            },
            _ => {},
        }
    }

    if out_elf.header.entry == 0 {
        println!("warning: _start not found, entry address is set to 0x0");
    }

    out_elf.segments = Linker::segments(&out_elf).unwrap();


    out_elf.store_all().unwrap();
    out_elf.to_writer(&mut out_file).unwrap();
}
