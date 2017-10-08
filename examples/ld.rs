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
use std::collections::hash_map::Entry;
use std::path::Path;


#[derive(Default)]
struct Unit {
    pub symbols:        Vec<Symbol>,
    pub relocations:    Vec<Relocation>,
}

impl Unit {
    pub fn load(in_elf: &Elf, out_elf: &mut Elf, name: &str)
        -> Result<Unit, elfkit::Error> {

        let mut symbols     = Vec::new();
        let mut rela        = Vec::new();
        let mut sectionmap  = HashMap::new();

        // 1. PROGBITS

        for (i, sec) in in_elf.sections.iter().enumerate() {
            if sec.header.flags.contains(types::SectionFlags::ALLOC) {
                let mut sec = sec.clone();
                //need writeble for reloc.
                //TODO we should probably use GNU_RELRO or something
                sec.header.flags.insert(types::SectionFlags::WRITE);

                sectionmap.insert(i as u16, out_elf.sections.len() as u16);
                sec.name.insert_str(0, name);
                out_elf.sections.push(sec);
            }
        }

        out_elf.sync_all().unwrap();
        Linker::relayout(out_elf, 0x300).unwrap();

        // 2. SYMTAB

        let mut symtab_shndx = 0;
        for (i, sec) in in_elf.sections.iter().enumerate() {
            match sec.header.shtype {
                types::SectionType::SYMTAB => {
                    if symtab_shndx > 0 {
                        panic!(format!("error loading '{}': found SYMTAB section at {} but there already was one at {}\n
                                 it's theoretically possible to link using multiple SYMTAB sections\n
                                 if you really need that, open a bug report",
                                 name, i, symtab_shndx));
                        continue;
                    }
                    symtab_shndx = i;

                    for sym in sec.content.as_symbols().unwrap() {
                        let mut sym = sym.clone();
                        if sym.shndx > 0 && sym.shndx < 65521 {
                            sym.shndx = match sectionmap.get(&sym.shndx) {
                                None => continue,
                                Some(v) => *v,
                            };
                            sym.value += out_elf.sections[sym.shndx as usize].header.addr;
                        }
                        symbols.push(sym);
                    }
                },
                _ => {},
            }
        }

        // 3. RELA

        for sec in in_elf.sections.iter() {
            if sec.header.shtype == types::SectionType::RELA {
                let (mut reloc_target,reloc_target_shndx) = match sectionmap.get(&(sec.header.info as u16)) {
                    None => {println!("warning: section {} is not allocated, \
                                     referenced in reloc section {}", sec.header.info, sec.name); continue},
                    Some(v) => (
                        std::mem::replace(&mut out_elf.sections[*v as usize], Section::default()),
                        *v as usize),
                };

                if symtab_shndx != sec.header.link as usize {
                    panic!(format!("error loading '{}' section '{}' sec.header.link({}) != symtab_shndx({})", 
                                   name, sec.name, sec.header.link, symtab_shndx));
                }

                for reloc in sec.content.as_relocations().unwrap() {
                    match reloc.rtype {
                        RelocationType::R_X86_64_NONE => {},

                        // Symbol + Addend
                        RelocationType::R_X86_64_64   => {
                            let symbol  = &symbols[reloc.sym as usize];
                            if symbol.shndx == 0 {
                                rela.push(Relocation {
                                    rtype:  RelocationType::R_X86_64_64,
                                    sym:    reloc.sym, //FIXME: this offset will change with multiple units,
                                    addr:   reloc_target.header.addr + reloc.addr,
                                    addend: reloc.addend,
                                });
                                println!("delaying undefined {:?}", reloc);
                            } else {
                                let value = (
                                    symbol.value as i64 +
                                    reloc.addend as i64)
                                    as u64;

                                println!("linking local {:?} -> {:?} value {:x}", reloc, symbol, value);
                                rela.push(Relocation {
                                    rtype:  RelocationType::R_X86_64_RELATIVE,
                                    sym:    0,
                                    addr:   reloc_target.header.addr + reloc.addr,
                                    addend: value as i64,
                                });
                            }
                        },

                        //Symbol + Addend - Load address of the Global Offset Table
                        RelocationType::R_X86_64_GOTOFF64 => {
                            let symbol  = &symbols[reloc.sym as usize];
                            let value = (
                                symbol.value as i64 +
                                reloc.addend as i64)
                                as u64;

                            println!("relocating {:?} value {:x}", reloc, value);
                        },

                        //Symbol + Addend - relocation target section load address
                        RelocationType::R_X86_64_PC32   => {
                            let symbol  = &symbols[reloc.sym as usize];
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
                        _ => panic!(format!("loading relocation {:?} not implemented",reloc)),
                    }
                }

                out_elf.sections[reloc_target_shndx as usize] = reloc_target;

            }
        }

        Ok(Unit{
            symbols:    symbols,
            relocations: rela,
        })
    }
}


fn main() {
    let out_filename = "/tmp/e";

    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut out_elf = Elf::default();
    out_elf.header.ident_class      = types::Class::Class64;
    out_elf.header.ident_endianness = types::Endianness::LittleEndian;
    out_elf.header.ident_abi        = types::Abi::SYSV;
    out_elf.header.etype            = types::ElfType::DYN;
    out_elf.header.machine          = types::Machine::X86_64;

    let mut sc_interp  : Vec<u8> = b"/lib64/ld-linux-x86-64.so.2\0".to_vec();
    let mut sc_symtab  : Vec<Symbol>  = vec![Symbol::default()];
    let mut sc_dynsym  = Symtab::from_vec(vec![Symbol::default()]);
    let mut sc_rela    : Vec<Relocation>  = Vec::new();
    let mut sc_dynamic : Vec<Dynamic> = vec![
        Dynamic{
            dhtype: types::DynamicType::FLAGS_1,
            content: DynamicContent::Flags1(types::DynamicFlags1::PIE),
        },
        //        Dynamic{
        //            dhtype: types::DynamicType::NEEDED,
        //            content: DynamicContent::String(String::from("libdxdata.so")),
//        },
    ];


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(".interp", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    let mut units = Vec::new();
    for in_path in env::args().skip(1) {
        println!("loading unit {}", in_path);
        let mut in_file  = OpenOptions::new().read(true).open(&in_path).unwrap();
        let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();
        in_elf.load_all().unwrap();

        let in_name = Path::new(&in_path).file_name().unwrap().to_string_lossy().into_owned();
        let mut unit = Unit::load(&in_elf, &mut out_elf, &in_name).unwrap();

        //for debugging
        sc_symtab.extend(unit.symbols.iter().cloned());
        units.push(unit);
    }


    let mut link_lookup = Symtab::default();
    for unit in &mut units {
        for sym in &unit.symbols {
            link_lookup.insert(sym.clone());
        }
    }


    for unit in &mut units {
        for mut reloc in unit.relocations.drain(..) {
            let symbol = match unit.symbols.get(reloc.sym as usize) {
                Some(v) => v,
                None => panic!(format!("reloc with missing symbol {:?}", reloc)),
            };

            let symbol = match link_lookup.get(&symbol.name) {
                None => panic!(format!("undefined reference to {}", symbol.name)),
                Some(s) => s,
            };

            match reloc.rtype {
                RelocationType::R_X86_64_NONE => {},
                RelocationType::R_X86_64_RELATIVE => {
                    sc_rela.push(reloc);
                },
                RelocationType::R_X86_64_64 => {
                    let value = (
                        symbol.value as i64 +
                        reloc.addend as i64)
                        as i64;

                    println!("linking {:?} value {:x}", reloc, value);
                    sc_rela.push(Relocation {
                        rtype:  RelocationType::R_X86_64_RELATIVE,
                        sym:    0,
                        addr:   reloc.addr,
                        addend: value as i64,
                    });

                },
                _ => panic!(format!("linking relocation {:?} not implemented",reloc)),
            };
        }
    }










    let sh_index_dynstr = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynstr", types::SectionType::STRTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Strtab(Strtab::default()), 0,0));

    let sc_dynsym = sc_dynsym.into_vec();
    let first_global_dynsym = sc_dynsym.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    let sh_index_dynsym = out_elf.sections.len();
    out_elf.sections.push(Section::new(".dynsym", types::SectionType::DYNSYM,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Symbols(sc_dynsym),
                                       sh_index_dynstr as u32, first_global_dynsym as u32));

    out_elf.sections.push(Section::new(".rela.dyn", types::SectionType::RELA,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Relocations(sc_rela),
                                       sh_index_dynsym as u32, 0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();

    sc_dynamic.extend(Linker::dynamic(&out_elf).unwrap());
    out_elf.sections.push(Section::new(".dynamic", types::SectionType::DYNAMIC,
                                                           types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                                           SectionContent::Dynamic(sc_dynamic), sh_index_dynstr as u32,0));


    let sh_index_strtab = out_elf.sections.len();
    out_elf.sections.push(Section::new(".strtab", types::SectionType::STRTAB,
                                       types::SectionFlags::empty(),
                                       SectionContent::Strtab(Strtab::default()), 0,0));

    //sc_symtab.sort_unstable_by(|a,b| a.bind.cmp(&b.bind));
    let first_global_symtab = sc_symtab.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    out_elf.sections.push(Section::new(".symtab", types::SectionType::SYMTAB,
                                       types::SectionFlags::empty(),
                                       SectionContent::Symbols(sc_symtab),
                                       sh_index_strtab as u32, first_global_symtab as u32));

    out_elf.sections.push(Section::new(".shstrtab", types::SectionType::STRTAB,
                                       types::SectionFlags::from_bits_truncate(0),
                                       SectionContent::Strtab(Strtab::default()),
                                       0,0));


    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();


    //find the start sym
    for sec in &out_elf.sections {
        match &sec.content {
            &SectionContent::Symbols(ref v) => {
                for sym in v{
                    if sym.name == "_start" {
                        out_elf.header.entry = /*out_elf.sections[sym.shndx as usize].header.addr + */sym.value;
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

#[derive(Default)]
struct Symtab {
    pub v: Vec<Symbol>,
    pub h: HashMap<String, usize>,
}

impl Symtab {
    fn from_vec(v: Vec<Symbol>) -> Symtab {
        let mut h = HashMap::new();
        for (i,sym) in v.iter().enumerate() {
            h.insert(sym.name.clone(), i);
        }
        Symtab{v,h}
    }

    fn into_vec(self) -> Vec<Symbol> {
        self.v
    }

    fn get(&self, name: &String) -> Option<&Symbol> {
        match self.h.get(name) {
            Some(v) => self.v.get(*v),
            None => None,
        }
    }

    fn insert(&mut self, mut sym: Symbol) -> usize {
        match self.h.entry(sym.name.clone()) {
            Entry::Vacant(o) => {
                let i = self.v.len();
                o.insert(i);
                self.v.push(sym);
                i
            },
            Entry::Occupied(o) => {
                let sym2 = &mut self.v[*o.get()];
                if sym.bind == types::SymbolBind::GLOBAL {
                    if sym2.bind == types::SymbolBind::GLOBAL && sym2.shndx != 0{
                        panic!(println!("re-export of globally defined symbol {:?} <> {:?}",
                                 sym, sym2));

                    }
                    std::mem::replace(sym2, sym);
                }
                *o.get()
            }
        }
    }
}

