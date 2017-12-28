#[macro_use] extern crate log;
#[macro_use] extern crate elfkit;
extern crate ordermap;
extern crate byteorder;
extern crate env_logger;

use std::env;
use elfkit::{Elf, Header, types, symbol, relocation, section, Error, loader, dynamic};
use elfkit::symbolic_linker::{SymbolicLinker};
use self::ordermap::{OrderMap};
use std::collections::hash_map::{self,HashMap};
use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::process;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();


    let args = parse_ld_options();
    let mut loader: Vec<loader::State> = args.object_paths.into_iter().map(|s|{
        loader::State::Path{name:s}}).collect();

    let mut elf = Elf::default();
    elf.header.ident_class      = types::Class::Class64;
    elf.header.ident_endianness = types::Endianness::LittleEndian;
    elf.header.ident_abi        = types::Abi::SYSV;
    elf.header.etype            = args.etype;
    elf.header.machine          = types::Machine::X86_64;

    let mut elf = match elf.header.etype {
        types::ElfType::DYN => {
            loader.push(loader::State::Object{
                name:     String::from("___linker_entry"),
                hash:     String::from("___linker_entry"),
                symbols:  vec![symbol::Symbol{
                    stype: types::SymbolType::FUNC,
                    size:  0,
                    value: 0,
                    bind:  types::SymbolBind::GLOBAL,
                    vis:   types::SymbolVis::DEFAULT,
                    shndx: symbol::SymbolSectionIndex::Undefined,
                    name:  b"_start".to_vec(),
                    _name: 0,
                }],
                header:   Header::default(),
                sections: Vec::new(),
            });

            let mut linker = SymbolicLinker::default();

            match linker.link(loader) {
                Ok(_)   => {},
                Err(Error::ConflictingSymbol{sym, obj1_name, obj1_hash, obj2_name, obj2_hash}) => {
                    error!("conflicting symbol '{}'\n       in {} [LHAS {}]\n       in {} [LHAS {}]",
                           sym, obj1_name, obj1_hash, obj2_name, obj2_hash);
                    process::abort();
                },
                Err(e)  => panic!(e),
            };

            info!("lookup complete: {} nodes in link tree", linker.objects.len());
            linker.gc();
            info!("  after gc: {}", linker.objects.len());

            elf.sections.push(section::Section::default());
            let mut dl = args.dynamic_linker.into_bytes();
            if dl.len() > 0 {
                dl.push(0);
                elf.sections.push(section::Section::new(b".interp".to_vec(), types::SectionType::PROGBITS,
                types::SectionFlags::ALLOC,
                section::SectionContent::Raw(dl), 0, 0));
            }

            let mut collected = SimpleCollector::new(elf).collect(linker).into_collected();
            DynamicRelocator::relocate(&mut collected).unwrap();
            let mut elf = collected.into_elf();
            elf.make_symtab_gnuld_compat().unwrap();
            elf.layout().unwrap();
            elf
        },
        types::ElfType::REL => {
            let mut linker = SymbolicLinker::default();
            linker.link_all(loader).unwrap();
            info!("lookup complete: {} nodes in link tree", linker.objects.len());
            elf.sections.push(section::Section::default());
            let mut collected = SimpleCollector::new(elf).collect(linker).into_collected();
            DynamicRelocator::relocate(&mut collected).unwrap();
            let mut elf = collected.into_elf();
            elf.make_symtab_gnuld_compat().unwrap();
            elf.layout().unwrap();
            elf
        },
        _ => unreachable!(),
    };



    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(args.output_path).unwrap();
    elf.to_writer(&mut out_file).unwrap();

    let mut perms = out_file.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    out_file.set_permissions(perms).unwrap();

}


struct DynamicRelocator {
}
impl DynamicRelocator {
    pub fn relocate (collected: &mut Collected) -> Result<(), Error>  {

        let shndx_com = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".com".to_vec(),
        types::SectionType::NOBITS, types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
        section::SectionContent::None, 0, 0));
        collected.elf.sections[shndx_com].header.addralign = 16;

        let shndx_got = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".got".to_vec(),
        //musl's dalias basically said got must be writeable. i disagree and gnuld seems to do
        //relocs before protection, so lots more stuff can do read only, but whatever
        types::SectionType::PROGBITS, types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
        section::SectionContent::None, 0, 0));
        collected.elf.sections[shndx_got].header.addralign = 16;

        let mut dynrel      = Vec::new();
        let mut dynrel_addr = Vec::new();

        let mut hrel        = Vec::new();
        let mut got         = Vec::new();
        let mut sym2got     = HashMap::new();
        let mut sym2tls     = HashMap::new();

        for (shndx, relocs) in collected.relocs.drain() {
            for mut reloc in relocs {
                let mut sym = collected.symtab.get(reloc.sym as usize).unwrap().clone();
                match sym.shndx {
                    symbol::SymbolSectionIndex::Section(_) => {},
                    symbol::SymbolSectionIndex::Common => {
                        let com_slot = collected.elf.sections[shndx_com].header.size;
                        collected.elf.sections[shndx_com].header.size += sym.size;
                        sym.value = com_slot;
                        sym.shndx = symbol::SymbolSectionIndex::Section(shndx_com as u16);
                        collected.symtab[reloc.sym as usize] = sym.clone();
                    },
                    symbol::SymbolSectionIndex::Undefined => {
                        assert_eq!(sym.value, 0);
                    },
                    _ => panic!("garbage relocation {:?} against {:?}", reloc, sym),
                };

                match reloc.rtype {
                    relocation::RelocationType::R_X86_64_64 => {
                        reloc.rtype   = relocation::RelocationType::R_X86_64_RELATIVE;
                        dynrel.push(reloc);
                        dynrel_addr.push(shndx);
                    },
                    relocation::RelocationType::R_X86_64_PC32 |
                    relocation::RelocationType::R_X86_64_PLT32 => {
                        reloc.rtype = relocation::RelocationType::R_X86_64_PC32;
                        hrel.push((shndx,reloc));
                    },
                    relocation::RelocationType::R_X86_64_GOTPCREL |
                        relocation::RelocationType::R_X86_64_GOTPCRELX |
                        relocation::RelocationType::R_X86_64_REX_GOTPCRELX => {
                        let got_sym = match sym2got.entry(reloc.sym) {
                            hash_map::Entry::Occupied(e) => *e.get(),
                            hash_map::Entry::Vacant(e) => {
                                let got_slot = got.len();
                                got.append(&mut vec![0;8]);
                                let got_sym = collected.symtab.len();
                                collected.symtab.push(symbol::Symbol{
                                    shndx:  symbol::SymbolSectionIndex::Section(shndx_got as u16),
                                    value:  got_slot as u64,
                                    size:   8,
                                    name:   [&sym.name[..], b"__GOT"].concat(),
                                    stype:  types::SymbolType::OBJECT,
                                    bind:   types::SymbolBind::GLOBAL,
                                    vis:    types::SymbolVis::DEFAULT,
                                    _name:  0,
                                });

                                if let symbol::SymbolSectionIndex::Undefined = sym.shndx {
                                    warn!("{:?} to undefined symbol {} will be relocated to zeroed out GOT",
                                          reloc.rtype, String::from_utf8_lossy(&sym.name));
                                } else {
                                    dynrel.push(relocation::Relocation{
                                        addr:   got_slot as u64,
                                        sym:    reloc.sym,
                                        rtype:  relocation::RelocationType::R_X86_64_RELATIVE,
                                        addend: 0,
                                    });
                                    dynrel_addr.push(shndx_got);
                                }

                                e.insert(got_sym);
                                got_sym
                            },
                        };

                        reloc.sym = got_sym as u32;
                        reloc.rtype = relocation::RelocationType::R_X86_64_PC32;
                        hrel.push((shndx, reloc));
                    },
                    relocation::RelocationType::R_X86_64_32 | relocation::RelocationType::R_X86_64_32S => {
                        panic!("unsupported relocation. maybe missing -fPIC ? {:?} -> {:?}", reloc, sym);
                    },

                    relocation::RelocationType::R_X86_64_GOTTPOFF |
                    relocation::RelocationType::R_X86_64_TLSLD |
                    relocation::RelocationType::R_X86_64_TLSGD => {
                        if sym.stype != types::SymbolType::TLS {
                            //says drepper's paper at least. but then again i have no idea wtf the
                            //TLS symbol is for anyway. this check could be removed i guess.
                            panic!("relocation {:?} cannot be used against non-tls symbol {:?}", reloc, sym);
                        }
                        let tls_sym = match sym2tls.entry(reloc.sym) {
                            hash_map::Entry::Occupied(e) => *e.get(),
                            hash_map::Entry::Vacant(e) => {
                                let got_slot = got.len();

                                let gotsize = match reloc.rtype {
                                    relocation::RelocationType::R_X86_64_TLSGD => {
                                        elf_write_u64!(&collected.elf.header, got, 1)?;
                                        elf_write_u64!(&collected.elf.header, got, sym.value)?;
                                        16
                                    },
                                    relocation::RelocationType::R_X86_64_TLSLD => {
                                        elf_write_u64!(&collected.elf.header, got, 1)?;
                                        //The offset is already added by the compiler,
                                        //so the offset in the tls entry must be 0.
                                        //TODO but what's the point of all of this if it's always 0?
                                        //i must still be missing something.
                                        elf_write_u64!(&collected.elf.header, got, 0)?;
                                        16
                                    },
                                    relocation::RelocationType::R_X86_64_GOTTPOFF => {
                                        //TODO: this is entirely guesswork since it's not documented clearly
                                        //tdata comes exactly before %fs on x86_64, like so:
                                        //
                                        //     -200 [ module 2 .tdata ]
                                        //     -100 [ module 1 .tdata ]
                                        //     0    $fs
                                        //
                                        // so a symbol value 10 in module 1 is 10 - 100 = -90.
                                        // (this example ignores alignment)
                                        //
                                        let shndx_tdata = match sym.shndx {
                                            symbol::SymbolSectionIndex::Section(shndx) => shndx,
                                            _ => panic!("relocation against garbage symbol {:?} -> {:?}",
                                                        reloc, sym),
                                        };

                                        let mut tlx = collected.elf.sections[shndx_tdata as usize].header.size;
                                        let align = collected.elf.sections[shndx_tdata as usize].header.addralign;
                                        if tlx % align > 0 {
                                            tlx += align - (tlx % align);
                                        }
                                        tlx -= sym.value;
                                        let tlx = -(tlx as i64);

                                        elf_write_u64!(&collected.elf.header, got, tlx as u64)?;
                                        8
                                    }
                                    _ => unreachable!(),
                                };


                                let tls_sym = collected.symtab.len();
                                collected.symtab.push(symbol::Symbol{
                                    shndx:  symbol::SymbolSectionIndex::Section(shndx_got as u16),
                                    value:  got_slot as u64,
                                    size:   gotsize,
                                    name:   [&sym.name[..], b"__TLS_GOT"].concat(),
                                    stype:  types::SymbolType::OBJECT,
                                    bind:   types::SymbolBind::GLOBAL,
                                    vis:    types::SymbolVis::DEFAULT,
                                    _name:  0,
                                });

                                e.insert(tls_sym);
                                tls_sym
                            },
                        };


                        reloc.sym = tls_sym as u32;
                        reloc.rtype = relocation::RelocationType::R_X86_64_PC32;
                        hrel.push((shndx, reloc));
                    },
                    relocation::RelocationType::R_X86_64_DTPOFF32 => {
                        reloc.rtype = relocation::RelocationType::R_X86_64_32;
                        hrel.push((shndx, reloc));
                    },
                    _ => {
                        panic!("relocating {:?} of '{}' in '{}' not implemented",
                               reloc.rtype,
                               String::from_utf8_lossy(&sym.name),
                               String::from_utf8_lossy(&collected.elf.sections[shndx].name),
                              );
                    }
                }
            }
        }

        collected.elf.sections[shndx_got].content = section::SectionContent::Raw(got);

        let shndx_dynstr = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".dynstr".to_vec(),
        types::SectionType::STRTAB, types::SectionFlags::ALLOC,
        section::SectionContent::Strtab(elfkit::strtab::Strtab::default()), 0, 0));

        let mut shndx_dynsym = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".dynsym".to_vec(),
        types::SectionType::DYNSYM, types::SectionFlags::ALLOC,
        section::SectionContent::Symbols(vec![symbol::Symbol::default()]), shndx_dynstr as u32, 0));

        let mut shndx_hash = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".hash".to_vec(),
        types::SectionType::HASH, types::SectionFlags::ALLOC,
        section::SectionContent::None, shndx_dynsym  as u32, 0));

        let mut shndx_reladyn = collected.elf.sections.len();
        collected.elf.sections.push(section::Section::new(b".rela.dyn".to_vec(),
        types::SectionType::RELA, types::SectionFlags::ALLOC,
        section::SectionContent::Relocations(dynrel),
        shndx_dynsym as u32, 0));
        collected.elf.sections[shndx_reladyn].header.addralign = 8;

        let mut shndx_dynamic = collected.elf.sections.len();
        let dynamic = DynamicRelocator::dynamic(&collected.elf)?;
        collected.elf.sections.push(section::Section::new(b".dynamic".to_vec(), types::SectionType::DYNAMIC,
        types::SectionFlags::ALLOC | types::SectionFlags::WRITE, // TODO why writeable?
        //section::SectionContent::Dynamic(vec![dynamic::Dynamic::default()]),
        section::SectionContent::Dynamic(dynamic),
        shndx_dynstr as u32,0));
        collected.elf.sections[shndx_dynamic].header.addralign = 8;


        let remap = collected.reorder()?;

        collected.elf.layout().unwrap();

        if let Some(v) = remap.get(&shndx_hash) {
            shndx_hash = *v;
        }
        if let Some(v) = remap.get(&shndx_dynsym) {
            shndx_dynsym = *v;
        }
        if let Some(v) = remap.get(&shndx_reladyn) {
            shndx_reladyn = *v;
        }
        if let Some(v) = remap.get(&shndx_dynamic) {
            shndx_dynamic = *v;
        }

        collected.elf.sections[shndx_hash   ].addrlock = true;
        collected.elf.sections[shndx_dynsym ].addrlock = true;
        collected.elf.sections[shndx_reladyn].addrlock = true;
        collected.elf.sections[shndx_dynamic].content = section::SectionContent::Dynamic(
            DynamicRelocator::dynamic(&collected.elf)?);



        let mut shndx_init_array = None;
        let mut shndx_fini_array = None;
        for (i, section) in collected.elf.sections.iter().enumerate() {
            match section.header.shtype {
                types::SectionType::INIT_ARRAY => {
                    shndx_init_array = Some(i);
                },
                types::SectionType::FINI_ARRAY => {
                    shndx_fini_array = Some(i);
                },
                _ => {}
            }
        }

        for sym in collected.symtab.iter_mut() {
            if let symbol::SymbolSectionIndex::Section(so) = sym.shndx {
                let addr = collected.elf.sections[so as usize].header.addr;

                if sym.stype != types::SymbolType::TLS {
                    sym.value += addr;
                }
                if sym.name == b"_start" && sym.bind == types::SymbolBind::GLOBAL {
                    collected.elf.header.entry = sym.value;
                }
            }
            match sym.name.as_slice() {
                b"_DYNAMIC" => {
                    sym.stype   = types::SymbolType::OBJECT;
                    sym.bind    = types::SymbolBind::LOCAL;
                    sym.vis     = types::SymbolVis::DEFAULT;
                    sym.value   = collected.elf.sections[shndx_dynamic].header.addr;
                    sym.shndx   = symbol::SymbolSectionIndex::Section(shndx_dynamic as u16);
                },
                b"__init_array_start" => {
                    if let Some(shndx_init_array) = shndx_init_array {
                        sym.stype   = types::SymbolType::NOTYPE;
                        sym.bind    = types::SymbolBind::LOCAL;
                        sym.vis     = types::SymbolVis::DEFAULT;
                        sym.value   = collected.elf.sections[shndx_init_array].header.addr;
                        sym.shndx   = symbol::SymbolSectionIndex::Section(shndx_init_array as u16);
                    }
                },
                b"__init_array_end" => {
                    if let Some(shndx_init_array) = shndx_init_array {
                        sym.stype   = types::SymbolType::NOTYPE;
                        sym.bind    = types::SymbolBind::LOCAL;
                        sym.vis     = types::SymbolVis::DEFAULT;
                        sym.value   = collected.elf.sections[shndx_init_array].header.addr +
                            collected.elf.sections[shndx_init_array].header.size;
                        sym.shndx   = symbol::SymbolSectionIndex::Section(shndx_init_array as u16);
                    }
                },
                b"__fini_array_start" => {
                    if let Some(shndx_fini_array) = shndx_fini_array {
                        sym.stype   = types::SymbolType::NOTYPE;
                        sym.bind    = types::SymbolBind::LOCAL;
                        sym.vis     = types::SymbolVis::DEFAULT;
                        sym.value   = collected.elf.sections[shndx_fini_array].header.addr;
                        sym.shndx   = symbol::SymbolSectionIndex::Section(shndx_fini_array as u16);
                    }
                },
                b"__fini_array_end" => {
                    if let Some(shndx_fini_array) = shndx_fini_array {
                        sym.stype   = types::SymbolType::NOTYPE;
                        sym.bind    = types::SymbolBind::LOCAL;
                        sym.vis     = types::SymbolVis::DEFAULT;
                        sym.value   = collected.elf.sections[shndx_fini_array].header.addr +
                            collected.elf.sections[shndx_fini_array].header.size;
                        sym.shndx   = symbol::SymbolSectionIndex::Section(shndx_fini_array as u16);
                    }
                },
                _ => {},
            }
        }


        let mut reladyn = std::mem::replace(collected.elf.sections[shndx_reladyn].content
                                        .as_relocations_mut().unwrap(), Vec::new());
        for (i, rel) in reladyn.iter_mut().enumerate() {
            let mut shndx = dynrel_addr[i];
            if let Some(v) = remap.get(&shndx) {
                shndx = *v;
            }
            rel.addend  += collected.symtab[rel.sym as usize].value as i64;
            rel.sym     =  0;
            rel.addr    += collected.elf.sections[shndx].header.addr;
            if rel.addend == 0 {
                error!("BUG emitting R_X86_64_RELATIVE with addend == 0 wont work.");
            }
        }
        collected.elf.sections[shndx_reladyn].content = section::SectionContent::Relocations(reladyn);




        for (mut shndx, reloc) in hrel.into_iter() {
            if let Some(v) = remap.get(&shndx) {
                shndx = *v;
            }
            let sym = collected.symtab.get(reloc.sym as usize).unwrap();
            match reloc.rtype {
                relocation::RelocationType::R_X86_64_PC32 => {
                    let vaddr = collected.elf.sections[shndx].header.addr + reloc.addr;
                    let rv = ((sym.value as i64) + (reloc.addend as i64) - (vaddr as i64)) as i32;

                    if sym.value == 0 {
                        warn!("{:?} to undefined symbol  {}",  reloc.rtype,
                              String::from_utf8_lossy(&sym.name));
                    }

                    let w = match collected.elf.sections[shndx].content.as_raw_mut() {
                        Some(v) => v.as_mut_slice(),
                        None => {
                            panic!("relocation {:?} against non-raw section {} makes no sense",
                                   reloc, shndx);
                        }
                    };

                    if reloc.addr > w.len() as u64 {
                        panic!("relocation {:?} against section {} would exceed its size of {} bytes",
                               reloc, shndx, w.len());
                    }

                    let mut w = &mut w[reloc.addr as usize ..];
                    elf_write_u32!(&collected.elf.header, w, rv as u32)?;
                },
                relocation::RelocationType::R_X86_64_32 => {
                    let w = match collected.elf.sections[shndx].content.as_raw_mut() {
                        Some(v) => v.as_mut_slice(),
                        None => {
                            panic!("relocation {:?} against non-raw section {} makes no sense",
                                   reloc, shndx);
                        }
                    };
                    let value = sym.value as i64 + reloc.addend;
                    let mut w = &mut w[reloc.addr as usize ..];
                    elf_write_u32!(&collected.elf.header, w, value as u32)?;
                },
                _ => unreachable!(),
            }
        }

        Ok(())

    }

    pub fn dynamic(elf: &Elf) -> Result<Vec<dynamic::Dynamic>, Error> {
        let mut padding = Vec::new();
        let mut r = vec![
            dynamic::Dynamic{
                dhtype: types::DynamicType::FLAGS_1,
                content: dynamic::DynamicContent::Flags1(types::DynamicFlags1::PIE),
            },
        ];

        for sec in &elf.sections {
            match sec.name.as_slice() {
                b".hash" => {
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::HASH,
                        content: dynamic::DynamicContent::Address(sec.header.addr),
                    });
                }
                b".dynstr" => {
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::STRTAB,
                        content: dynamic::DynamicContent::Address(sec.header.addr),
                    });

                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::STRSZ,
                        content: dynamic::DynamicContent::Address(sec.header.size),
                    });
                }
                b".dynsym" => {
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::SYMTAB,
                        content: dynamic::DynamicContent::Address(sec.header.addr),
                    });
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::SYMENT,
                        content: dynamic::DynamicContent::Address(sec.header.entsize),
                    });
                }
                b".rela.dyn" => {
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::RELA,
                        content:dynamic:: DynamicContent::Address(sec.header.addr),
                    });
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::RELASZ,
                        content: dynamic::DynamicContent::Address(sec.header.size),
                    });
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::RELAENT,
                        content: dynamic::DynamicContent::Address(sec.header.entsize),
                    });

                    let first_non_rela = match sec.content.as_relocations() {
                        None => return Err(Error::UnexpectedSectionContent),
                        Some(v) => v.iter()
                            .position(|ref r| {
                                r.rtype != relocation::RelocationType::R_X86_64_RELATIVE
                                    && r.rtype != relocation::RelocationType::R_X86_64_JUMP_SLOT
                            })
                        .unwrap_or(v.len()),
                    } as u64;


                    if first_non_rela > 0 {
                        r.push(dynamic::Dynamic {
                            dhtype: types::DynamicType::RELACOUNT,
                            content: dynamic::DynamicContent::Address(first_non_rela),
                        });
                    } else {
                        padding.push(dynamic::Dynamic::default());
                    }

                    //TODO
                    r.push(dynamic::Dynamic {
                        dhtype: types::DynamicType::TEXTREL,
                        content: dynamic::DynamicContent::Address(0),
                    });
                }
                _ => {}
            }
        }

        r.append(&mut padding);

        r.push(dynamic::Dynamic {
            dhtype: types::DynamicType::NULL,
            content: dynamic::DynamicContent::Address(0),
        });


        Ok(r)
    }
}


pub struct Collected {
    pub elf:        Elf,
    pub symtab:     Vec<symbol::Symbol>,
    pub relocs:     HashMap<usize, Vec<relocation::Relocation>>,
}

impl Collected {
    pub fn into_elf(mut self) -> Elf {
        let sh_index_strtab = self.elf.sections.len();

        self.elf.sections.push(section::Section::new(b".strtab".to_vec(), types::SectionType::STRTAB,
        types::SectionFlags::empty(),
        section::SectionContent::Strtab(elfkit::strtab::Strtab::default()), 0,0));

        let sh_index_symtab = self.elf.sections.len();
        let first_global_symtab = self.symtab.iter().enumerate()
            .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
        self.elf.sections.push(section::Section::new(b".symtab".to_vec(), types::SectionType::SYMTAB,
        types::SectionFlags::empty(),
        section::SectionContent::Symbols(self.symtab),
        sh_index_strtab as u32, first_global_symtab as u32));

        for (shndx, relocs) in self.relocs {
            let mut name = b".rela".to_vec();
            name.append(&mut self.elf.sections[shndx].name.clone());

            self.elf.sections.push(section::Section::new(name, types::SectionType::RELA,
                                                              types::SectionFlags::empty(),
                                                              section::SectionContent::Relocations(relocs), sh_index_symtab as u32, shndx as u32));
        }

        self.elf.sections.push(section::Section::new(b".shstrtab".to_vec(), types::SectionType::STRTAB,
        types::SectionFlags::from_bits_truncate(0),
        section::SectionContent::Strtab(elfkit::strtab::Strtab::default()),
        0,0));

        self.elf
    }
    pub fn reorder(&mut self) -> Result<HashMap<usize,usize>, Error> {
        let reorder = self.elf.reorder()?;
        for sym in &mut self.symtab {
            if let symbol::SymbolSectionIndex::Section(shndx) = sym.shndx {
                if let Some(nu) = reorder.get(&(shndx as usize)) {
                    sym.shndx = symbol::SymbolSectionIndex::Section(*nu as u16);
                }
            }
        }

        for (mut i, reloc) in std::mem::replace(&mut self.relocs, HashMap::new()) {
            if let Some(nu) = reorder.get(&i) {
                i = *nu;
            }
            self.relocs.insert(i, reloc);
        }

        Ok(reorder)
    }

}

pub trait Collector {
    fn into_collected(self) -> Collected;
}


/// a dummy implementation of Collector which works for testing
pub struct SimpleCollector {
    pub collected:  Collected,
    sections: OrderMap<Vec<u8>, section::Section>,
}

impl Collector for SimpleCollector {
    fn into_collected(mut self) -> Collected {
        let mut collected = self.collected;
        collected.elf.sections = self.sections.drain(..).map(|v|v.1).collect();

        collected
    }
}



impl SimpleCollector {

    pub fn new(mut elf: Elf) -> SimpleCollector {

        let mut sections = OrderMap::new();
        if elf.sections.len() < 1 {
            sections.insert(Vec::new(), section::Section::default());
        } else {
            for sec in elf.sections.drain(..) {
                sections.insert(sec.name.clone(), sec);
            }
        }

        let collected = Collected {
            elf: elf,
            relocs:     HashMap::new(),
            symtab:     Vec::new(),
        };


        Self{
            collected:  collected,
            sections:   sections,
        }
    }

    fn collect(mut self, mut linker: SymbolicLinker) -> Self {

        let mut input_map = HashMap::new();

        for (_, object) in linker.objects {
            let (nu_shndx, nu_off) = self.merge(object.section, object.relocs, object.name);
            input_map.insert(object.lid, (nu_shndx, nu_off));
        }

        for loc in &mut linker.symtab {
            match loc.sym.shndx {
                symbol::SymbolSectionIndex::Section(_) => {
                    match input_map.get(&loc.obj) {
                        None => {
                            panic!("linker emitted dangling link {} -> {:?}", loc.obj, loc.sym);
                        },
                        Some(&(nu_shndx, nu_off)) =>  {
                            if let symbol::SymbolSectionIndex::Section(_) = loc.sym.shndx {
                                loc.sym.shndx = symbol::SymbolSectionIndex::Section(nu_shndx as u16);
                                loc.sym.value += nu_off as u64;
                            }
                            self.collected.symtab.push(loc.sym.clone());
                        },
                    };
                },
                symbol::SymbolSectionIndex::Undefined => {
                    self.collected.symtab.push(loc.sym.clone());
                },
                symbol::SymbolSectionIndex::Absolute |
                    symbol::SymbolSectionIndex::Common => {
                    self.collected.symtab.push(loc.sym.clone());
                },
            }
        }

        self
    }

    fn merge(&mut self, mut sec: section::Section, rela: Vec<relocation::Relocation>, objname: String) -> (usize, usize) {

        let mut name = sec.name.clone();
        if name.len() > 3 && &name[0..4] == b".bss" {
            name = b".bss".to_vec();
        }
        if name.len() > 6 && &name[0..7] == b".rodata" {
            name = b".rodata".to_vec();
        }
        if name.len() > 4 && &name[0..5] == b".data" {
            name = b".data".to_vec();
        }
        if name.len() > 4 && &name[0..5] == b".text" {
            name = b".text".to_vec();
        }
        if name.len() > 5 && &name[0..6] == b".tdata" {
            name = b".tdata".to_vec();
        }
        if name.len() > 6 && &name[0..7] == b".debug_" {
            name = b".dbgFIXME".to_vec();
        }

        sec.header.flags.remove(types::SectionFlags::GROUP);

        let (nu_shndx, nu_off) = match self.sections.entry(name.clone()) {
            ordermap::Entry::Occupied(mut e) => {
                let i  = e.index();
                let ov = match sec.content {
                    section::SectionContent::Raw(mut r) => {
                        let align = std::cmp::max(e.get().header.addralign, sec.header.addralign);
                        e.get_mut().header.addralign = align;

                        let cc = e.get_mut().content.as_raw_mut().unwrap();
                        if  cc.len() % align as usize != 0 {
                            let mut al = vec![0;align as usize - (cc.len() % align as usize)];
                            cc.append(&mut al);
                        }
                        let ov = cc.len();
                        cc.append(&mut r);
                        ov
                    },
                    section::SectionContent::None => {
                        let ov = e.get().header.size;
                        e.get_mut().header.size += sec.header.size as u64;
                        ov as usize
                    },
                    _ => unreachable!(),
                };
                (i, ov)
            },
            ordermap::Entry::Vacant(e) => {
                let i = e.index();
                sec.name = name.clone();
                sec.addrlock = false;
                e.insert(sec);
                (i, 0)
            },
        };

        let relav = self.collected.relocs.entry(nu_shndx).or_insert_with(||Vec::new());
        for mut rel in rela {
            match rel.rtype {
                relocation::RelocationType::R_X86_64_32 | relocation::RelocationType::R_X86_64_32S => {
                    error!("unsupported relocation. maybe missing -fPIC ? {:?} in {}",
                           rel, objname);
                    process::abort();
                },
                _ => {},
            };
            rel.addr += nu_off as u64;
            relav.push(rel);
        }



        (nu_shndx, nu_off)
    }

}





use std::path::Path;

pub struct LdOptions {
    pub dynamic_linker: String,
    pub object_paths:   Vec<String>,
    pub output_path:    String,
    pub etype:          types::ElfType,
}

impl Default for LdOptions {
    fn default() -> Self {
        Self {
            dynamic_linker: String::default(),
            object_paths:   Vec::new(),
            output_path:    String::from("a.out"),
            etype:          types::ElfType::DYN,
        }
    }
}



fn search_lib(search_paths: &Vec<String>, needle: &String) -> String{
    let so = String::from("lib") + needle + ".a";
    for p in search_paths {
        let pc = Path::new(p).join(&so);
        if pc.exists() {
            return pc.into_os_string().into_string().unwrap();
        }
    }
    panic!("ld.elfkit: cannot find: {} in {:?}", so, search_paths);
}

fn ldarg(arg: &String, argname: &str, argc: &mut usize) -> Option<String> {
    if arg.starts_with(argname) {
        Some(if arg.len() < argname.len() + 1 {
            *argc += 1;
            env::args().nth(*argc).unwrap()
        } else {
            String::from(&arg[2..])
        })
    } else {
        None
    }
}


pub fn parse_ld_options() -> LdOptions{
    let mut options         = LdOptions::default();
    let mut search_paths    = Vec::new();

    debug!("arguments to ld.elfkit: {:?}", env::args());

    let mut argc = 1;
    loop {
        if argc >= env::args().len() {
            break;
        }

        let arg = env::args().nth(argc).unwrap();
        if let Some(val) = ldarg(&arg, "-L", &mut argc) {
            search_paths.push(val);
        } else if let Some(val) = ldarg(&arg, "-l", &mut argc) {
            let path = search_lib(&search_paths, &val);
            if !options.object_paths.contains(&path) {
                options.object_paths.push(path);
            }
        } else if let Some(val) = ldarg(&arg, "-m", &mut argc) {
            if val != "elf_x86_64" {
                panic!("machine not supported: {}", val);
            }
        } else if let Some(val) = ldarg(&arg, "-o", &mut argc) {
            options.output_path = val;
        } else if arg == "-pie" {
        } else if arg == "-r" {
            options.etype = types::ElfType::REL;
        } else if arg == "-dynamic-linker" {
            argc += 1;
            options.dynamic_linker = env::args().nth(argc).unwrap()
        } else if let Some(val) = ldarg(&arg, "-z", &mut argc) {
            warn!("argument ignored: -z {}" ,val);

        } else if arg == "-plugin" {
            argc += 1;
            warn!("argument ignored: -plugin {}" , env::args().nth(argc).unwrap());
        } else if arg.starts_with("-") {
            warn!("argument ignored: {}",arg);
        } else {
            options.object_paths.push(arg);
        }
        argc +=1;
    }

    info!("linking {:?}", options.object_paths);

    options
}


