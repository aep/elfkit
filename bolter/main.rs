extern crate colored;
#[macro_use] extern crate elfkit;
extern crate byteorder;
extern crate goblin;
extern crate sha2;

use elfkit::{
    Elf, Header, types, SegmentHeader, Section, SectionContent, Error,
    SectionHeader, Dynamic, Symbol, Relocation, Strtab, SymbolSectionIndex};

use elfkit::filetype;
use elfkit::linker;

use std::fs::OpenOptions;
use elfkit::dynamic::DynamicContent;
use elfkit::relocation::RelocationType;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use colored::*;
use sha2::Digest;
use std::io::Write;

mod ld;
use ld::*;
mod args;
use args::*;
mod relocations;
use relocations::*;

pub fn fail(msg: String) -> ! {
    println!("{}", msg.red());
    panic!("abort");
}

fn main() {
    let ldoptions  = parse_ld_options();
    let mut elfs   = load_elfs(ldoptions.object_paths);
    let mut lookup = Lookup::default();

    let mut start  = Symbol::default();
    start.name     = String::from("_start");
    start.bind     = types::SymbolBind::GLOBAL;
    let mut got    = Symbol::default();
    got.name       = String::from("_GLOBAL_OFFSET_TABLE_"); //TODO
    got.shndx      = SymbolSectionIndex::Section(1);
    got.bind       = types::SymbolBind::GLOBAL;
    lookup.insert_unit(Unit::fake(String::from("exe"), LinkBehaviour::Static, vec![start, got]));

    lookup.link(elfs);
    // TODO garbage collect unused units

    println!("linking {} units into exe", lookup.units.len());

    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(ldoptions.output_path).unwrap();
    let mut out_elf = Elf::default();
    out_elf.header.ident_class      = types::Class::Class64;
    out_elf.header.ident_endianness = types::Endianness::LittleEndian;
    out_elf.header.ident_abi        = types::Abi::SYSV;
    out_elf.header.etype            = types::ElfType::DYN;
    out_elf.header.machine          = types::Machine::X86_64;

    let mut sc_interp  : Vec<u8> = ldoptions.dynamic_linker.trim().bytes().collect();
    sc_interp.push(0);
    let mut sc_rela    : Vec<Relocation>        = Vec::new();
    let mut sc_dynsym  : Vec<Symbol>            = vec![Symbol::default()];
    let mut sc_dynamic : Vec<Dynamic>           = vec![
        Dynamic{
            dhtype: types::DynamicType::FLAGS_1,
            content: DynamicContent::Flags1(types::DynamicFlags1::PIE),
        },
    ];
    let mut sc_symtab : Vec<Symbol> = vec![Symbol::default()];


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(String::from(".interp"), types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC,
                                       SectionContent::Raw(sc_interp), 0,0));

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    //sort units by segment
    lookup.units.sort_unstable_by(|a,b| {
        a.segment.cmp(&b.segment)
    });

    //caculate space for bootstrap
    let boostrap_len = 1 + 4 + lookup.units.iter().fold(0, |acc, ref u| {
        acc + u.relocations.iter().fold(0, |acc, ref reloc|{
            acc + match reloc.rtype {
                RelocationType::R_X86_64_64 => 3 + 4 + 3 + 4,
                RelocationType::R_X86_64_GOTPCREL |
                    RelocationType::R_X86_64_GOTPCRELX |
                    RelocationType::R_X86_64_REX_GOTPCRELX => 3 + 4 + 3 + 4 + 2 + 4 + 4,
                RelocationType::R_X86_64_PC32 |
                    RelocationType::R_X86_64_PLT32 => 2 + 4 + 4,
                _ => 0,
            }
        })
    });
    let mut bootstrap = vec![0;boostrap_len];
    let sh_index_bootstrap = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".bootstrap"), types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC |
                                       types::SectionFlags::EXECINSTR,
                                       SectionContent::Raw(bootstrap),
                                       0,0));

    //layout all the units
    let mut global2section = HashMap::new();
    for unit in &mut lookup.units {

        if unit.code.len() == 0 {
            println!("{}",format!(
                    "trying to link unit '{}' with size==0. this might be a bug in llvm. padding unit to 8"
                    , unit.name).yellow()
                    );
            unit.code = vec![0;8];
        }

        let mut sec = match unit.segment {
            UnitSegment::Executable | UnitSegment::Data => {
                let mut flags = types::SectionFlags::ALLOC | types::SectionFlags::WRITE;
                if unit.segment == UnitSegment::Executable {
                    flags.insert(types::SectionFlags::EXECINSTR);
                }
                Section::new(unit.name.clone(),
                             types::SectionType::PROGBITS, flags,
                SectionContent::Raw(std::mem::replace(&mut unit.code, Vec::new())), 0, 0)
            },
            UnitSegment::Bss => {
                let mut sec = Section::new(unit.name.clone(), types::SectionType::NOBITS,
                    types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                    SectionContent::None, 0,0);
                sec.header.size = unit.code.len() as u64;
                sec
            }
        };

        global2section.insert(unit.global_id, out_elf.sections.len());
        out_elf.sections.push(sec);
    }

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    // symbols for debugging
    let blt_bootstrap_sym = Symbol{
        shndx:  SymbolSectionIndex::Section(sh_index_bootstrap as u16),
        value:  out_elf.sections[sh_index_bootstrap].header.addr,
        size:   out_elf.sections[sh_index_bootstrap].header.size,
        name:   String::from("__blt_bootstrap"),
        stype:  types::SymbolType::FUNC,
        bind:   types::SymbolBind::LOCAL,
        vis:    types::SymbolVis::DEFAULT,
    };
    sc_symtab.push(blt_bootstrap_sym.clone());


    // map all the relocs
    let mut count_got = 0;
    for mut unit in std::mem::replace(&mut lookup.units, Vec::new()) {
        for mut reloc in unit.relocations {
            let mut sym = &unit.symbols[reloc.sym as usize];
            if sym.shndx == SymbolSectionIndex::Undefined {
                sym = match lookup.get_by_name(&sym.name) {
                    Some(s) => s,
                    None => {
                        fail(format!(
                                "bug in elfkit linker: symbol no longer in lookup table while relocating: {:?} < {:?} < {}",
                                sym, reloc, unit.name));
                    }
                };
                assert!(sym.name.len() > 0);
            }
            let mut sym = sym.clone();
            if let SymbolSectionIndex::Global(id) = sym.shndx {
                sym.shndx  = SymbolSectionIndex::Section(global2section[&id] as u16);
                sym.value += out_elf.sections[global2section[&id]].header.addr;
            }

            reloc.addr += out_elf.sections[global2section[&unit.global_id]].header.addr;
            reloc.sym = sc_dynsym.len() as u32;
            sc_dynsym.push(sym);
            match reloc.rtype {
                RelocationType::R_X86_64_GOTPCREL |
                    RelocationType::R_X86_64_GOTPCRELX |
                    RelocationType::R_X86_64_REX_GOTPCRELX => {
                        count_got += 1;
                    },
                    _ => {},
            }
            sc_rela.push(reloc);
        }
    }


    let sh_index_got = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".got"), types::SectionType::NOBITS,
                                       types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                                       SectionContent::None,
                                       0,0));
    out_elf.sections[sh_index_got].header.size = count_got * 8;

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    let dynsym_index_got = sc_dynsym.len();
    sc_dynsym.push(Symbol{
        shndx:  SymbolSectionIndex::Section(sh_index_got as u16),
        value:  out_elf.sections[sh_index_got].header.addr,
        size:   0,
        name:   String::from("__got"),
        stype:  types::SymbolType::NOTYPE,
        bind:   types::SymbolBind::LOCAL,
        vis:    types::SymbolVis::DEFAULT,
    });

    //built bootstrap relocations
    let mut bootstrap = Vec::new();
    let mut got_used : u64 = 0;
    let relocs = std::mem::replace(&mut sc_rela, Vec::new());
    for mut reloc in relocs {
        match reloc.rtype {
            RelocationType::R_X86_64_64 => {
                write_bootstrap_abs64(&out_elf.header,
                                      out_elf.sections[sh_index_bootstrap].header.addr,
                                      &mut bootstrap,
                                      (sc_dynsym[reloc.sym as usize].value as i64 + reloc.addend) as u64,
                                      reloc.addr,
                                      );
            },

            RelocationType::R_X86_64_GOTPCREL |
                RelocationType::R_X86_64_GOTPCRELX |
                RelocationType::R_X86_64_REX_GOTPCRELX => {
                    let got_slot = got_used;
                    got_used += 8;
                    write_bootstrap_rel32(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          (sc_dynsym[dynsym_index_got].value as i64 +
                                           got_slot as i64 + reloc.addend) as u64,
                                          reloc.addr,
                                          );

                    //this is is only really used for debugging, hence symtab only
                    sc_symtab.push(Symbol{
                        shndx:  SymbolSectionIndex::Section(sh_index_got as u16),
                        value:  sc_dynsym[dynsym_index_got].value + got_slot,
                        size:   8,
                        name:   sc_dynsym[reloc.sym as usize].name.clone() + "@GOT",
                        stype:  types::SymbolType::OBJECT,
                        bind:   types::SymbolBind::LOCAL,
                        vis:    types::SymbolVis::DEFAULT,
                    });

                    if sc_dynsym[reloc.sym as usize].shndx == SymbolSectionIndex::Undefined {
                        assert!(sc_dynsym[reloc.sym as usize].bind == types::SymbolBind::WEAK);
                        println!("GOT slot at 0x{:x} remains 0 for undefined weak symbol {}",
                                 sc_dynsym[dynsym_index_got].value + got_slot,
                                 sc_dynsym[reloc.sym as usize].name
                                 );
                    } else {
                        write_bootstrap_abs64(&out_elf.header,
                                      out_elf.sections[sh_index_bootstrap].header.addr,
                                      &mut bootstrap,
                                      sc_dynsym[reloc.sym as usize].value,
                                      sc_dynsym[dynsym_index_got].value + got_slot,
                                  );
                    }
                },

            RelocationType::R_X86_64_PC32 => {
                write_bootstrap_rel32(&out_elf.header,
                                      out_elf.sections[sh_index_bootstrap].header.addr,
                                      &mut bootstrap,
                                      (sc_dynsym[reloc.sym as usize].value as i64 + reloc.addend) as u64,
                                      reloc.addr,
                                      );
            },
            RelocationType::R_X86_64_PLT32  => {
                write_bootstrap_rel32(&out_elf.header,
                              out_elf.sections[sh_index_bootstrap].header.addr,
                              &mut bootstrap,
                              (sc_dynsym[reloc.sym as usize].value as i64 + reloc.addend) as u64,
                              reloc.addr,
                             );
            },
            RelocationType::R_X86_64_32 | RelocationType::R_X86_64_32S => {
                println!("unsupported relocation. maybe missing -fPIC ? {:?}", reloc);
            },
            _ => {
                println!("unsupported relocation {:?}", reloc);
            },
        }
    }


    //reposition all the symbols for symtab
    for sym in &mut lookup.symbols {
        if let SymbolSectionIndex::Global(id) = sym.shndx {
            sym.value += out_elf.sections[global2section[&id]].header.addr;
            sym.shndx = SymbolSectionIndex::Section(global2section[&id] as u16);
            sym.bind  = types::SymbolBind::LOCAL;
        }
    }
    out_elf.header.entry = lookup.get_by_name("_start").unwrap().value;
    sc_symtab.extend(lookup.symbols);

    //TODO clearing out dynsym for now after having implemented relocations
    //this will have to be refactored when dynamic linking is supported
    sc_dynsym = vec![Symbol::default()];


    //indirect _start via __blt_bootstrap
    write_reljumpto(&out_elf.header,
                    out_elf.sections[sh_index_bootstrap].header.addr,
                    &mut bootstrap,
                    out_elf.header.entry,
                    );

    out_elf.header.entry = out_elf.sections[sh_index_bootstrap].header.addr;

    if bootstrap.len() < out_elf.sections[sh_index_bootstrap].header.size as usize {
        let more = out_elf.sections[sh_index_bootstrap].header.size as usize - bootstrap.len();
        bootstrap.extend(vec![0;more]);
    }
    assert_eq!(bootstrap.len(), out_elf.sections[sh_index_bootstrap].header.size as usize);
    out_elf.sections[sh_index_bootstrap].content = SectionContent::Raw(bootstrap);


    let mut sc_relink = Vec::new();
    for sec in &out_elf.sections {
        let io = &mut sc_relink;
        elf_write_u32!(&out_elf.header, io, sec.header.offset as u32);
    }

    // merge segments
    let mut text_off     = None;
    let mut text_content = Vec::new();
    let mut bss_off      = None;
    let mut bss_size     = 0;

    //TODO this only works because we layed it out continously earlier
    for mut sec in out_elf.sections.drain(sh_index_bootstrap+1..) {
        match sec.header.shtype {
            types::SectionType::PROGBITS => {
                if text_off == None {
                    text_off = Some(sec.header.offset);
                }
                text_content.extend(sec.content.as_raw_mut().unwrap().drain(..));
            },
            types::SectionType::NOBITS => {
                if bss_off == None {
                    bss_off = Some(sec.header.offset);
                }
                bss_size += sec.header.size;
            },
            _ => unreachable!(),
        };
    }
    out_elf.sections.push(Section::new(String::from(".text"),
    types::SectionType::PROGBITS,
    types::SectionFlags::ALLOC | types::SectionFlags::WRITE | types::SectionFlags::EXECINSTR,
    SectionContent::Raw(text_content), 0, 0));

    let mut bss = Section::new(String::from(".bss"),
    types::SectionType::NOBITS,
    types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
    SectionContent::None, 0, 0);
    bss.header.size = bss_size;
    out_elf.sections.push(bss);

    //store on .dynamic may add strings to dynsym, which will change all the offsets.
    //this is why dynstr is last. this index needs to be changed everytime something is added
    //between here and dynstr
    let sh_index_dynstr = out_elf.sections.len() + 3;

    let sh_index_dynsym = out_elf.sections.len();
    let symhash = elfkit::symbol::symhash(&out_elf.header, &sc_dynsym, sh_index_dynsym as u32);
    let first_global_dynsym = sc_dynsym.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    out_elf.sections.push(Section::new(String::from(".dynsym"), types::SectionType::DYNSYM,
    types::SectionFlags::ALLOC,
    SectionContent::Symbols(sc_dynsym),
    sh_index_dynstr as u32, first_global_dynsym as u32));

    out_elf.sections.push(symhash.unwrap());

    sc_rela.sort_unstable_by(|a,b| if a.rtype == RelocationType::R_X86_64_RELATIVE { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater} );
    out_elf.sections.push(Section::new(String::from(".rela.dyn"), types::SectionType::RELA,
    types::SectionFlags::ALLOC,
    SectionContent::Relocations(sc_rela),
    sh_index_dynsym as u32, 0));

    out_elf.sections.push(Section::new(String::from(".dynstr"), types::SectionType::STRTAB,
    types::SectionFlags::ALLOC,
    SectionContent::Strtab(Strtab::default()), 0,0));


    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    sc_dynamic.extend(linker::dynamic(&out_elf).unwrap());
    out_elf.sections.push(Section::new(String::from(".dynamic"), types::SectionType::DYNAMIC,
    types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
    SectionContent::Dynamic(sc_dynamic), sh_index_dynstr as u32,0));

    let sh_index_strtab = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".strtab"), types::SectionType::STRTAB,
    types::SectionFlags::empty(),
    SectionContent::Strtab(Strtab::default()), 0,0));

    //sc_symtab.sort_unstable_by(|a,b| a.bind.cmp(&b.bind));
    let first_global_symtab = sc_symtab.iter().enumerate()
        .find(|&(_,s)|s.bind == types::SymbolBind::GLOBAL).map(|(i,_)|i).unwrap_or(0);;
    out_elf.sections.push(Section::new(String::from(".symtab"), types::SectionType::SYMTAB,
    types::SectionFlags::empty(),
    SectionContent::Symbols(sc_symtab),
    sh_index_strtab as u32, first_global_symtab as u32));

    out_elf.sections.push(Section::new(String::from(".shstrtab"), types::SectionType::STRTAB,
    types::SectionFlags::from_bits_truncate(0),
    SectionContent::Strtab(Strtab::default()),
    0,0));

    out_elf.sections.push(Section::new(String::from(".relink"), types::SectionType::RELINKABLE,
    types::SectionFlags::from_bits_truncate(0),
    SectionContent::Raw(sc_relink),
    0,0));

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();
    out_elf.segments = linker::segments(&out_elf).unwrap();
    out_elf.store_all().unwrap();
    out_elf.to_writer(&mut out_file).unwrap();

    let mut perms = out_file.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    out_file.set_permissions(perms).unwrap();
}
