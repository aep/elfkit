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
    pub data: Vec<u8>,
    pub text: Vec<u8>,
    pub symbols: Vec<Symbol>,
    pub dynrel: Vec<Relocation>,
}

impl Unit {
    pub fn load(in_elf: &Elf) -> Result<Unit, elfkit::Error> {

        let mut r = Unit::default();

        let mut shndx_text = 0;
        let mut shndx_data = 0;

        for (i, sec) in in_elf.sections.iter().enumerate() {
            match (sec.name.as_ref(), &sec.content) {
                (".text", &SectionContent::Raw(ref v)) => {shndx_text = i; r.text.extend(v);},
                (".data", &SectionContent::Raw(ref v)) => {shndx_data = i; r.data.extend(v);},
                (".symtab", &SectionContent::Symbols(ref v)) => {r.symbols.extend(v.iter().cloned());},
                _ => {},
            };
        }

        for sec in in_elf.sections.iter() {
            if sec.header.shtype == types::SectionType::RELA && sec.header.info == shndx_text as u32 {
                let symtab = in_elf.sections[sec.header.link as usize].content.as_symbols().unwrap();
                for reloc in  sec.content.as_relocations().unwrap() {
                    match reloc.rtype {
                        RelocationType::R_X86_64_NONE => {},
                        RelocationType::R_X86_64_64   => {
                            let symbol = &symtab[reloc.sym as usize];
                            if symbol.shndx == shndx_data as u16 {
                                let value = (
                                    r.data.len() as i64 +
                                    symbol.value as i64 +
                                    reloc.addend as i64)
                                    as u64;

                                let (_,mut xd) = r.text.split_at_mut(reloc.addr as usize);
                                elf_write_u64!(&in_elf.header, xd, value)?;

                                r.dynrel.push(Relocation{
                                    rtype:  RelocationType::R_X86_64_RELATIVE,
                                    sym:    0,
                                    addr:   reloc.addr,
                                    addend: value as i64,
                                })
                            } else {
                                panic!(format!("relocation {:?} on symbol {:?} in unknown section",
                                               reloc, symbol));
                            }

                        },
                        _ => panic!(format!("relocation {:?} not implemented",reloc)),
                    }
                }
            }
        }


        Ok(r)
    }
}

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
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


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(".interp", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));



    let in_unit = Unit::load(&in_elf).unwrap();
    sc_text = in_unit.text;
    sc_data = in_unit.data;

    let sh_index_mo_ae = out_elf.sections.len();
    out_elf.sections.push(Section::new(".mo1.read", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC | types::SectionFlags::WRITE | types::SectionFlags::EXECINSTR,
                                       SectionContent::Raw(sc_text), 0,0));

    out_elf.sections.push(Section::new(".mo1.write", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                       SectionContent::Raw(sc_data), 0,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    for mut rel in in_unit.dynrel {
        rel.addr   += out_elf.sections[sh_index_mo_ae].header.addr;

        //TODO: this only works for X86_64_RELATIVE
        rel.addend += out_elf.sections[sh_index_mo_ae].header.addr as i64;
        sc_rela.push(rel);
    }



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
    for sym in in_unit.symbols {
        if sym.name == "_start" {
            out_elf.header.entry = out_elf.sections[sh_index_mo_ae].header.addr +  sym.value;
        }
    }


    out_elf.segments = Linker::segments(&out_elf).unwrap();


    out_elf.store_all().unwrap();
    out_elf.to_writer(&mut out_file).unwrap();
}

