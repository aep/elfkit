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
    got.shndx      = SymbolSectionIndex::Global(0);
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
    if sc_interp.len() > 1 {
        out_elf.sections.push(Section::new(String::from(".interp"), types::SectionType::PROGBITS,
        types::SectionFlags::ALLOC,
        SectionContent::Raw(sc_interp), 0,0));
    }

    //--------------------- prepare bootstrap section
    let boostrap_len = relocations::len_reljumpto  + lookup.units.iter().fold(0, |acc, ref u| {
        acc + u.relocations.iter().fold(0, |acc, ref reloc|{
            acc + match reloc.rtype {
                RelocationType::R_X86_64_64 => relocations::len_bootstrap_abs64,
                RelocationType::R_X86_64_GOTPCREL |
                    RelocationType::R_X86_64_GOTPCRELX |
                    RelocationType::R_X86_64_REX_GOTPCRELX =>
                        relocations::len_bootstrap_abs64 +
                        relocations::len_bootstrap_rel32,
                    RelocationType::R_X86_64_PC32 |
                    RelocationType::R_X86_64_PLT32 => relocations::len_bootstrap_rel32,
                RelocationType::R_X86_64_TLSGD => relocations::len_bootstrap_val64 +
                    relocations::len_bootstrap_abs64 +
                    relocations::len_bootstrap_rel32,
                _ => 0,
            }
        })
    });
    let mut bootstrap = vec![0;boostrap_len];
    let sh_index_bootstrap = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".xo.bootstrap"), types::SectionType::PROGBITS,
                                       types::SectionFlags::ALLOC |
                                       types::SectionFlags::EXECINSTR,
                                       SectionContent::Raw(bootstrap),
                                       0,0));

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

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



    //----------------------------layout
    let mut sc_relink   = Vec::new();
    let mut vaddr       = out_elf.sections[sh_index_bootstrap].header.addr +
        out_elf.sections[sh_index_bootstrap].header.size;
    let mut sc_text     = Vec::new();
    let mut sc_data     = Vec::new();
    let mut sc_tls      = Vec::new();
    let mut sc_bss      = 0;
    let mut unit_addresses = HashMap::new();

    lookup.units.sort_unstable_by(|a,b| {
        a.segment.cmp(&b.segment)
    });
    lookup.reindex();

    for unit in &mut lookup.units {
        match unit.segment {
            UnitSegment::Executable => {
                sc_relink.push(sc_text.len() as u32);
                unit_addresses.insert(unit.global_id, vaddr);
                vaddr      += unit.code.len() as u64;
                sc_text.append(&mut unit.code);
            },
            UnitSegment::Data => {
                sc_relink.push(sc_data.len() as u32);
                unit_addresses.insert(unit.global_id, vaddr);
                vaddr      += unit.code.len() as u64;
                sc_data.append(&mut unit.code);
            },
            UnitSegment::Bss => {
                unit_addresses.insert(unit.global_id, vaddr);
                vaddr      += unit.code.len() as u64;
                sc_bss     += unit.code.len() as u64;
            },
            UnitSegment::Tls => {
                sc_relink.push(sc_tls.len() as u32);
                unit_addresses.insert(unit.global_id, vaddr);
                vaddr      += unit.code.len() as u64;
                sc_tls.append(&mut unit.code);
            }
        }
    }

    let sh_index_text = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".xo.text"),
    types::SectionType::PROGBITS,
    types::SectionFlags::ALLOC | types::SectionFlags::WRITE | types::SectionFlags::EXECINSTR,
    SectionContent::Raw(sc_text), 0, 0));

    let sh_index_data = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".xo.data"),
    types::SectionType::PROGBITS,
    types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
    SectionContent::Raw(sc_data), 0, 0));

    let sh_index_bss = out_elf.sections.len();
    if sc_bss > 0 {
        let mut bss = Section::new(String::from(".xo.bss"),
        types::SectionType::NOBITS,
        types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
        SectionContent::None, 0, 0);
        bss.header.size = sc_bss;
        out_elf.sections.push(bss);
    }

    let sh_index_tls = out_elf.sections.len();
    if sc_tls.len() > 0 {
        let mut tls = Section::new(String::from(".xo.tdata"),
        types::SectionType::PROGBITS,
        types::SectionFlags::ALLOC | types::SectionFlags::WRITE | types::SectionFlags::TLS,
        SectionContent::Raw(sc_tls), 0, 0);
        out_elf.sections.push(tls);
    }

    //reposition all the symbols
    for sym in &mut lookup.symbols {
        if let SymbolSectionIndex::Global(id) = sym.shndx {
            let unit = &lookup.units[lookup.by_id[&id]];
                sym.shndx = SymbolSectionIndex::Section(match unit.segment {
                    UnitSegment::Executable => sh_index_text,
                    UnitSegment::Data       => sh_index_data,
                    UnitSegment::Bss        => sh_index_bss,
                    UnitSegment::Tls        => sh_index_tls,
                } as u16);
                sym.value += unit_addresses[&unit.global_id];
        }
    }
    out_elf.header.entry = lookup.get_by_name("_start").unwrap().value;

    //symtab
    for unit in &lookup.units {
        for mut sym in unit.symbols.iter().cloned() {
            if let SymbolSectionIndex::Global(id) = sym.shndx {
                let unit = &lookup.units[lookup.by_id[&id]];
                sym.shndx = SymbolSectionIndex::Section(match unit.segment {
                    UnitSegment::Executable => sh_index_text,
                    UnitSegment::Data       => sh_index_data,
                    UnitSegment::Bss        => sh_index_bss,
                    UnitSegment::Tls        => sh_index_tls,
                } as u16);
                sym.value += unit_addresses[&unit.global_id];
            }
            sc_symtab.push(sym);
        }
    }

    //----------------------------------relocate
    let mut bootstrap : Vec<u8> = Vec::new();
    let mut got_used : u64 = 0;

    //TODO: we emit a GOT for every reloc, although some relocs point to the same symbol
    for mut unit in std::mem::replace(&mut lookup.units, Vec::new()) {
        for mut reloc in unit.relocations {
            let mut sym = &unit.symbols[reloc.sym as usize];
            let sym_addr = match sym.stype {
                types::SymbolType::SECTION => {
                    if let SymbolSectionIndex::Global(id) = sym.shndx {
                        unit_addresses[&id]
                    } else {
                        panic!("bug in elfkit linker: reloc against section that's not global. like what?");
                    }
                },
                _ => {
                    if sym.shndx == SymbolSectionIndex::Undefined {
                        match lookup.get_by_name(&sym.name) {
                            Some(s) => {
                                assert!(s.name.len() > 0);
                                s.value
                            },
                            None => {
                                panic!(
                                    "bug in elfkit linker: symbol no longer in lookup table while relocating: {:?} < {:?} < {}",
                                    sym, reloc, unit.name);
                            }
                        }
                    } else if let SymbolSectionIndex::Global(id) = sym.shndx {
                        unit_addresses[&id] + sym.value
                    } else {
                        panic!("bug in elfkit linker: symbol in reloc neither undefined nor global {:?}", sym);
                    }
                }
            };
            reloc.addr += unit_addresses[&unit.global_id];

            if sym_addr == 0 {
                assert!(sym.bind == types::SymbolBind::WEAK);
                println!("undefined weak (this is usually ok) {:?} to {}", reloc.rtype, sym.name);
            }

            match reloc.rtype {
                RelocationType::R_X86_64_64 => {
                    write_bootstrap_abs64(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          (sym_addr as i64 + reloc.addend) as u64,
                                          reloc.addr,
                                          );
                },
                RelocationType::R_X86_64_PC32 | RelocationType::R_X86_64_PLT32 => {
                    write_bootstrap_rel32(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          (sym_addr as i64 + reloc.addend) as u64,
                                          reloc.addr,
                                          );
                },
                RelocationType::R_X86_64_GOTPCREL | RelocationType::R_X86_64_GOTPCRELX | RelocationType::R_X86_64_REX_GOTPCRELX => {

                    let got_slot = vaddr;
                    vaddr += 8;
                    out_elf.sections[sh_index_bss].header.size += 8;

                    write_bootstrap_rel32(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          (got_slot as i64 + reloc.addend) as u64,
                                          reloc.addr,
                                          );

                    //this is is only really used for debugging
                    sc_symtab.push(Symbol{
                        shndx:  SymbolSectionIndex::Section(sh_index_bss  as u16),
                        value:  got_slot,
                        size:   8,
                        name:   sym.name.clone() + "__GOT",
                        stype:  types::SymbolType::OBJECT,
                        bind:   types::SymbolBind::LOCAL,
                        vis:    types::SymbolVis::DEFAULT,
                    });


                    write_bootstrap_abs64(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          sym_addr,
                                          got_slot,
                                          );
                },


                RelocationType::R_X86_64_32 | RelocationType::R_X86_64_32S => {
                    fail(format!("unsupported relocation. maybe missing -fPIC ? {:?}", reloc));
                },

                // this is the "general model"
                RelocationType:: R_X86_64_TLSGD => {

                    let got_slot_mod = vaddr;
                    let got_slot_off = vaddr + 8;
                    vaddr += 16;
                    out_elf.sections[sh_index_bss].header.size += 16;

                    write_bootstrap_val64(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          1, //module is always 1,
                                          got_slot_mod,
                                          );

                    write_bootstrap_abs64(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          sym_addr,
                                          got_slot_off,
                                          );
                    //this is is only really used for debugging
                    sc_symtab.push(Symbol{
                        shndx:  SymbolSectionIndex::Section(sh_index_bss  as u16),
                        value:  got_slot_mod,
                        size:   8,
                        name:   sym.name.clone() + "__TLSGD",
                        stype:  types::SymbolType::OBJECT,
                        bind:   types::SymbolBind::LOCAL,
                        vis:    types::SymbolVis::DEFAULT,
                    });

                    write_bootstrap_rel32(&out_elf.header,
                                          out_elf.sections[sh_index_bootstrap].header.addr,
                                          &mut bootstrap,
                                          (got_slot_mod as i64 + reloc.addend) as u64,
                                          reloc.addr,
                                          );

                },
                _ => {
                    fail(format!("unsupported relocation {:?} to {:?}", reloc, sym));
                },
            }
        }
    }

    //sc_symtab.append(&mut lookup.symbols);

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


    let sh_index_dynstr = out_elf.sections.len();
    out_elf.sections.push(Section::new(String::from(".dynstr"), types::SectionType::STRTAB,
    types::SectionFlags::ALLOC,
    SectionContent::Strtab(Strtab::default()), 0,0));

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


    let mut b_relink = Vec::new();
    for cut in sc_relink {
        let io = &mut b_relink;
        elf_write_u32!(&out_elf.header, io, cut);
    }

    out_elf.sections.push(Section::new(String::from(".relink"), types::SectionType::RELINKABLE,
    types::SectionFlags::from_bits_truncate(0),
    SectionContent::Raw(b_relink),
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
