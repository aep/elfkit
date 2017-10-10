#[macro_use] extern crate elfkit;
#[macro_use] extern crate itertools;
extern crate byteorder;

use itertools::Itertools;

use std::env;
use std::fs::OpenOptions;
use elfkit::{
    Elf, Header, types, SegmentHeader, Section, SectionContent,
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
                                None => 0,
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
                                    sym:    reloc.sym,
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


                        //TODO we're not emitting a PLT or GOT. see NOTES.md


                        RelocationType::R_X86_64_PLT32  |
                        RelocationType::R_X86_64_REX_GOTPCRELX  |
                            RelocationType:: R_X86_64_GOTPCREL |
                            RelocationType::R_X86_64_GOTPCRELX => {
                            rela.push(Relocation {
                                rtype:  RelocationType::R_X86_64_PC32,
                                sym:    reloc.sym,
                                addr:   reloc_target.header.addr + reloc.addr,
                                addend: reloc.addend,
                            });
                            println!("delaying {:?}", reloc);
                        }



                        //Symbol + Addend - relocation target section load address
                        RelocationType::R_X86_64_PC32   => {
                            rela.push(Relocation {
                                rtype:  RelocationType::R_X86_64_PC32,
                                sym:    reloc.sym,
                                addr:   reloc_target.header.addr + reloc.addr,
                                addend: reloc.addend,
                            });
                            println!("delaying undefined {:?}", reloc);
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
    ];


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(".interp", types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));

    out_elf.sync_all().unwrap();
    Linker::relayout(&mut out_elf, 0x300).unwrap();


    let mut link_lookup = Symtab::default();
    let mut units = Vec::new();

    for in_path in env::args().skip(1) {
        let mut in_file  = OpenOptions::new().read(true).open(&in_path).unwrap();
        let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();
        let in_name = Path::new(&in_path).file_name().unwrap().to_string_lossy().into_owned();
        match in_elf.header.etype {
            types::ElfType::REL => {
                println!("loading unit {}", in_path);
                in_elf.load_all().unwrap();

                let mut unit = Unit::load(&in_elf, &mut out_elf, &in_name).unwrap();

                //for debugging
                sc_symtab.extend(unit.symbols.iter().cloned());
                units.push(unit);
            },
            types::ElfType::DYN => {
                println!("loading so {}", in_path);
                sc_dynamic.push(Dynamic{
                    dhtype: types::DynamicType::NEEDED,
                    content: DynamicContent::String(in_name),
                });
                in_elf.load_all().unwrap();
                for sec in &in_elf.sections {
                    if sec.header.shtype == types::SectionType::DYNSYM {
                        for mut sym in sec.content.as_symbols().unwrap().iter().cloned() {
                            sym.shndx = 0;
                            sym.value = 0;
                            sym.size  = 0;
                            link_lookup.insert(sym);
                        }
                    }
                }
            },
            other => {
                panic!(format!("{}: unable to link elf objects of type {:?}", in_path, other));
            }
        }
    }

    for unit in &mut units {
        for mut sym in unit.symbols.iter().cloned() {
            if sym.shndx > 0 {
                sym.bind = types::SymbolBind::LOCAL;
                link_lookup.insert(sym);
            }
        }
    }

    for unit in &mut units {
        for mut reloc in unit.relocations.drain(..) {

            if reloc.rtype == RelocationType::R_X86_64_RELATIVE {
                sc_rela.push(reloc);
                continue;
            }


            let symbol = match unit.symbols.get(reloc.sym as usize) {
                Some(v) => v,
                None => panic!(format!("reloc with missing symbol {:?}", reloc)),
            };

            let symbol = match symbol.stype {
                types::SymbolType::SECTION => {
                    //FIXME this leads to LOCAL symbols beeing emited after GLOBAL
                    symbol
                },
                types::SymbolType::FUNC |
                    types::SymbolType::OBJECT |
                    types::SymbolType::NOTYPE
                    if symbol.name.len() > 0 => {
                        match link_lookup.get_by_name(&symbol.name) {
                            None => {
                                if symbol.bind == types::SymbolBind::WEAK {
                                    symbol
                                } else {
                                    panic!(format!("undefined reference to {:?}", symbol))
                                }
                            },
                            Some(s) => s,
                        }
                    },
                _ => {
                    panic!(format!("reloc to unsupported symbol {:?}->{:?}", reloc, symbol));
                },
            };

            if symbol.shndx == 0 && symbol.bind == types::SymbolBind::LOCAL {
                panic!(format!("undefined reference to {:?}", symbol));
            }

            match reloc.rtype {
                RelocationType::R_X86_64_NONE => {},
                RelocationType::R_X86_64_RELATIVE => unreachable!(),
                RelocationType::R_X86_64_64 => {
                    let value = (
                        symbol.value as i64 +
                        reloc.addend as i64)
                        as i64;

                    println!("emitting RELA for {:?} value {:x}", reloc, value);
                    sc_rela.push(Relocation {
                        rtype:  RelocationType::R_X86_64_RELATIVE,
                        sym:    0,
                        addr:   reloc.addr,
                        addend: value as i64,
                    });
                },
                RelocationType::R_X86_64_PC32 => {
                    if symbol.bind == types::SymbolBind::WEAK {
                        println!("NOT emitting reloc to weak symbol {:?} -> {:?}", reloc, symbol);
                        continue;
                    }
                    println!("emitting TEXTREL for {:?} -> {:?}", reloc, symbol);

                    let sym = sc_dynsym.insert(symbol.clone());


                    sc_rela.push(Relocation {
                        rtype:  RelocationType::R_X86_64_PC32,
                        sym:    sym as u32,
                        addr:   reloc.addr,
                        addend: reloc.addend,
                    });
                }
                _ => panic!(format!("linking relocation {:?} not implemented",reloc)),
            };
        }
    }




    //store on .dynamic may add strings to dynsym, which will change all the offsets.
    //this is why dynstr is last. this index needs to be changed everytime something is added
    //between here and dynstr
    let sh_index_dynstr = out_elf.sections.len() + 3;

    let sh_index_dynsym = out_elf.sections.len();
    let symhash = sc_dynsym.make_symhash(&out_elf.header, sh_index_dynsym as u32);
    let sc_dynsym = sc_dynsym.into_vec();
    let first_global_dynsym = sc_dynsym.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    out_elf.sections.push(Section::new(".dynsym", types::SectionType::DYNSYM,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Symbols(sc_dynsym),
                                       sh_index_dynstr as u32, first_global_dynsym as u32));

    out_elf.sections.push(symhash);


    sc_rela.sort_unstable_by(|a,b| if a.rtype == RelocationType::R_X86_64_RELATIVE { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater} );

    out_elf.sections.push(Section::new(".rela.dyn", types::SectionType::RELA,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Relocations(sc_rela),
                                       sh_index_dynsym as u32, 0));


    out_elf.sections.push(Section::new(".dynstr", types::SectionType::STRTAB,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Strtab(Strtab::default()), 0,0));


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
                for sym in v {
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

fn sysv_hash(s: &String) -> u64 {

    let mut h : u64 = 0;
    let mut g : u64 = 0;

    for byte in s.bytes() {
        h = (h << 4) + byte as u64;
        g = h & 0xf0000000;
        if g >  0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    return h;
}



#[derive(Default)]
struct Symtab {
    pub ordered:    Vec<Symbol>,
    pub by_name:    HashMap<String, usize>,
    pub by_section: HashMap<u16, usize>,
}

impl Symtab {
    fn from_vec(mut v: Vec<Symbol>) -> Symtab {
        let mut r = Symtab::default();
        for sym in v.drain(..) {
            r.insert(sym);
        }
        r
    }

    fn into_vec(self) -> Vec<Symbol> {
        self.ordered
    }


    fn make_symhash(&self, eh: &Header, link: u32) -> Section {
        //TODO i'm too lazy to do this correctly now, so we'll just emit a hashtable with nbuckets  == 1
        let mut b = Vec::new();
        {
            let mut io = &mut b;
            elf_write_uclass!(eh, io, 1); //nbuckets
            elf_write_uclass!(eh, io, self.ordered.len() as u64); //nchains

            elf_write_uclass!(eh, io, 1); //the bucket. pointing at symbol 1

            elf_write_uclass!(eh, io, 0); //symbol 0

            //the chains. every symbol just points at the next, because nbuckets == 1
            for i in 1..self.ordered.len() - 1 {
                elf_write_uclass!(eh, io, i as u64 + 1);
            }

            //except the last one
            elf_write_uclass!(eh, io, 0);
        }

        Section {
            name: String::from(".hash"),
            header: SectionHeader {
                name: 0,
                shtype: types::SectionType::HASH,
                flags:  types::SectionFlags::ALLOC,
                addr:   0,
                offset: 0,
                size:   b.len() as u64,
                link:   link,
                info:       0,
                addralign:  0,
                entsize:    8, // or 4 for CLass32
            },
            content: SectionContent::Raw(b),
        }
    }

    fn get_by_name(&self, name: &String) -> Option<&Symbol> {
        match self.by_name.get(name) {
            Some(v) => self.ordered.get(*v),
            None => None,
        }
    }

    fn insert(&mut self, mut sym: Symbol) -> usize {
        match sym.stype {
            types::SymbolType::SECTION => {
                match self.by_section.entry(sym.shndx) {
                    Entry::Vacant(o) => {
                        let i = self.ordered.len();
                        o.insert(i);
                        self.ordered.push(sym);
                        i
                    },
                    Entry::Occupied(o) => {
                        let sym2 = &mut self.ordered[*o.get()];
                        std::mem::replace(sym2, sym);
                        *o.get()
                    }
                }
            },
            types::SymbolType::FILE | types::SymbolType::NOTYPE | types::SymbolType::OBJECT | types::SymbolType::FUNC => {
                match self.by_name.entry(sym.name.clone()) {
                    Entry::Vacant(o) => {
                        let i = self.ordered.len();
                        o.insert(i);
                        self.ordered.push(sym);
                        i
                    },
                    Entry::Occupied(o) => {
                        let sym2 = &mut self.ordered[*o.get()];
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
            },
            _ => {
                panic!(format!("unsupported symbol type for Symtab: {:?}", sym));
            }

        }
    }
}

