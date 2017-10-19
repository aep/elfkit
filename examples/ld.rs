extern crate byteorder;
extern crate colored;
extern crate elfkit;
extern crate goblin;

use std::env;
use std::io::{Cursor, Read};
use std::fs::OpenOptions;
use elfkit::{types, Dynamic, Elf, Relocation, Section, SectionContent,
             Strtab, Symbol, SymbolSectionIndex};

use elfkit::filetype;
use elfkit::linker;

use elfkit::dynamic::DynamicContent;
use elfkit::relocation::RelocationType;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use colored::*;

pub fn fail(msg: String) -> ! {
    println!("{}", msg.red());
    panic!("abort");
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


#[derive(Default)]
struct LdOptions {
    dynamic_linker: String,
    object_paths: Vec<String>,
    output_path: String,
}

fn search_lib(search_paths: &Vec<String>, needle: &String) -> String {
    let so = String::from("lib") + needle + ".a";
    for p in search_paths {
        let pc = Path::new(p).join(&so);
        if pc.exists() {
            return pc.into_os_string().into_string().unwrap();
        }
    }
    fail(format!(
        "ld.elfkit: cannot find: {} in {:?}",
        so,
        search_paths
    ));
}

fn parse_ld_options() -> LdOptions {
    let mut options = LdOptions::default();
    options.output_path = String::from("a.out");
    let mut search_paths = Vec::new();

    let mut argc = 1;
    loop {
        if argc >= env::args().len() {
            break;
        }

        let arg = env::args().nth(argc).unwrap();
        if let Some(val) = ldarg(&arg, "-L", &mut argc) {
            search_paths.push(val);
        } else if let Some(_) = ldarg(&arg, "-z", &mut argc) {
            argc += 1;
            let arg2 = env::args().nth(argc).unwrap();
            println!("{}", format!("argument ignored: {} {}", arg, arg2).yellow());
        } else if let Some(val) = ldarg(&arg, "-l", &mut argc) {
            options.object_paths.push(search_lib(&search_paths, &val));
        } else if let Some(val) = ldarg(&arg, "-m", &mut argc) {
            if val != "elf_x86_64" {
                fail(format!("machine not supported: {}", val));
            }
        } else if let Some(val) = ldarg(&arg, "-o", &mut argc) {
            options.output_path = val;
        } else if arg == "-pie" {
        } else if arg == "-dynamic-linker" {
            argc += 1;
            options.dynamic_linker = env::args().nth(argc).unwrap()
        } else if arg.starts_with("-") {
            println!("{}", format!("argument ignored: {}", arg).yellow());
        } else {
            options.object_paths.push(arg);
        }
        argc += 1;
    }

    println!("linking {:?}", options.object_paths);

    options
}

fn main() {
    let ldoptions = parse_ld_options();
    let mut elfs = load_elfs(ldoptions.object_paths);
    let mut lookup = Lookup::default();

    let mut start = Symbol::default();
    start.name = String::from("_start");
    start.bind = types::SymbolBind::GLOBAL;
    let mut got = Symbol::default();
    got.name = String::from("_GLOBAL_OFFSET_TABLE_"); //TODO
    got.shndx = SymbolSectionIndex::Section(1);
    got.bind = types::SymbolBind::GLOBAL;
    lookup.insert_unit(Unit::fake(
        String::from("exe"),
        LinkBehaviour::Static,
        vec![start, got],
    ));

    let mut global_id_counter = 10;
    let mut candidates = HashMap::new();

    loop {
        println!("lookup iteration");
        let missing = lookup
            .symbols
            .iter()
            .enumerate()
            .filter_map(|(i, ref sym)| {
                if sym.shndx == SymbolSectionIndex::Undefined
                    && sym.bind == types::SymbolBind::GLOBAL
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<usize>>();

        if missing.len() < 1 {
            break;
        }

        for mi in missing {
            let mut found = None;
            let was_needed_by = lookup.units[lookup.symbols2units[&mi]].name.clone();

            let mut cont = true;
            while cont {
                cont = false;
                for ei in 0..elfs.len() {
                    let contains = match elfs[ei].1.contains_symbol(&lookup.symbols[mi].name) {
                        Ok(v) => v,
                        Err(e) => fail(format!("error in lookup in {} : {:?}", elfs[ei].0, e)),
                    };
                    if contains {
                        let elf = elfs.swap_remove(ei);
                        for unit in Unit::from_elf(elf.0, elf.1, &mut global_id_counter) {
                            candidates.insert(unit.global_id.clone(), unit);
                        }
                        cont = true;
                        break;
                    }
                }
            }


            for (id, candidate) in candidates.iter() {
                for sym in candidate.lookup(&lookup.symbols[mi].name) {
                    if sym.shndx != SymbolSectionIndex::Undefined {
                        found = Some(id.clone());
                        break;
                    }
                }
            }

            if let Some(id) = found {
                let unit = candidates.remove(&id).unwrap();
                resursive_insert(&mut candidates, &mut lookup, unit, &mut HashSet::new());

                println!(
                    " - {} <= {} <= {} ",
                    was_needed_by,
                    &lookup.symbols[mi].name,
                    lookup.units[lookup.symbols2units[&mi]].name,
                );

                //integrity check
                if lookup.symbols[mi].shndx == SymbolSectionIndex::Undefined {
                    panic!(
                        "BUG in elfkit lookup: symbol {:?} is still undefined after inserting \
                         unit where Unit:lookup() returned a defined symbol",
                        lookup.symbols[mi]
                    );
                }
            } else {
                let sym = &lookup.symbols[mi];
                if sym.shndx == SymbolSectionIndex::Undefined
                    && sym.bind == types::SymbolBind::GLOBAL
                {
                    fail(format!(
                        "{}: undefined reference to {}",
                        lookup.units[lookup.symbols2units[&mi]].name,
                        lookup.symbols[mi].name
                    ));
                }
            }
        }
    }


    println!("linking {} units into exe", lookup.units.len());

    let mut out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(ldoptions.output_path)
        .unwrap();
    let mut out_elf = Elf::default();
    out_elf.header.ident_class = types::Class::Class64;
    out_elf.header.ident_endianness = types::Endianness::LittleEndian;
    out_elf.header.ident_abi = types::Abi::SYSV;
    out_elf.header.etype = types::ElfType::DYN;
    out_elf.header.machine = types::Machine::X86_64;

    let mut sc_interp: Vec<u8> = ldoptions.dynamic_linker.trim().bytes().collect();
    sc_interp.push(0);
    let mut sc_rela: Vec<Relocation> = Vec::new();
    let mut sc_dynsym: Vec<Symbol> = vec![Symbol::default()];
    let mut sc_dynamic: Vec<Dynamic> = vec![
        Dynamic {
            dhtype: types::DynamicType::FLAGS_1,
            content: DynamicContent::Flags1(types::DynamicFlags1::PIE),
        },
    ];
    let mut sc_symtab: Vec<Symbol> = vec![Symbol::default()];


    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section::new(
        String::from(".interp"),
        types::SectionType::PROGBITS,
        types::SectionFlags::ALLOC,
        SectionContent::Raw(sc_interp),
        0,
        0,
    ));

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();


    //sort units by segment
    lookup
        .units
        .sort_unstable_by(|a, b| a.segment.cmp(&b.segment));

    //layout all the units
    let mut global2section = HashMap::new();
    for unit in &mut lookup.units {
        assert!(
            unit.code.len() > 0,
            format!("trying to link unit size == 0 '{}'", unit.name)
        );

        let mut sec = match unit.segment {
            UnitSegment::Executable | UnitSegment::Data => {
                let mut flags = types::SectionFlags::ALLOC | types::SectionFlags::WRITE;
                if unit.segment == UnitSegment::Executable {
                    flags.insert(types::SectionFlags::EXECINSTR);
                }
                Section::new(
                    unit.name.clone(),
                    types::SectionType::PROGBITS,
                    flags,
                    SectionContent::Raw(std::mem::replace(&mut unit.code, Vec::new())),
                    0,
                    0,
                )
            }
            UnitSegment::Bss => {
                let mut sec = Section::new(
                    unit.name.clone(),
                    types::SectionType::NOBITS,
                    types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
                    SectionContent::None,
                    0,
                    0,
                );
                sec.header.size = unit.code.len() as u64;
                sec
            }
        };

        global2section.insert(unit.global_id, out_elf.sections.len());
        out_elf.sections.push(sec);
    }

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    // map all the relocs
    let mut count_got = 0;
    for mut unit in std::mem::replace(&mut lookup.units, Vec::new()) {
        for mut reloc in unit.relocations {
            let mut sym = &unit.symbols[reloc.sym as usize];
            if sym.shndx == SymbolSectionIndex::Undefined {
                sym = lookup.get_by_name(&sym.name).unwrap();
                assert!(sym.name.len() > 0);
            }
            let mut sym = sym.clone();
            if let SymbolSectionIndex::Global(id) = sym.shndx {
                sym.shndx = SymbolSectionIndex::Section(global2section[&id] as u16);
                sym.value += out_elf.sections[global2section[&id]].header.addr;

                //TODO ld.so doesn't like WEAK symbols
                //if sym.bind == types::SymbolBind::GLOBAL {
                sym.bind = types::SymbolBind::LOCAL;
                //}
            }

            reloc.addr += out_elf.sections[global2section[&unit.global_id]]
                .header
                .addr;
            reloc.sym = sc_dynsym.len() as u32;
            sc_dynsym.push(sym);
            match reloc.rtype {
                RelocationType::R_X86_64_GOTPCREL |
                RelocationType::R_X86_64_GOTPCRELX |
                RelocationType::R_X86_64_REX_GOTPCRELX => {
                    count_got += 1;
                }
                _ => {}
            }
            sc_rela.push(reloc);
        }
    }

    let sh_index_got = out_elf.sections.len();
    out_elf.sections.push(Section::new(
        String::from(".got"),
        types::SectionType::NOBITS,
        types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
        SectionContent::None,
        0,
        0,
    ));
    out_elf.sections[sh_index_got].header.size = count_got * 8;

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    let dynsym_index_got = sc_dynsym.len();
    sc_dynsym.push(Symbol {
        shndx: SymbolSectionIndex::Section(sh_index_got as u16),
        value: out_elf.sections[sh_index_got].header.addr,
        size: 0,
        name: String::from(".got"),
        stype: types::SymbolType::NOTYPE,
        bind: types::SymbolBind::LOCAL,
        vis: types::SymbolVis::DEFAULT,
    });

    //resolve some relocations that ld can't do

    let mut got_used: u64 = 0;
    let relocs = std::mem::replace(&mut sc_rela, Vec::new());
    for mut reloc in relocs {
        match reloc.rtype {
            //FIXME R_X86_64_REX_GOTPCRELX doesnt work?
            //also the addend thing is weird. why carry over the -4?
            RelocationType::R_X86_64_GOTPCREL |
            RelocationType::R_X86_64_GOTPCRELX |
            RelocationType::R_X86_64_REX_GOTPCRELX => {
                let got_slot = got_used;
                got_used += 8;

                sc_rela.push(Relocation {
                    addr: reloc.addr,
                    sym: dynsym_index_got as u32,
                    rtype: RelocationType::R_X86_64_PC32,
                    addend: got_slot as i64 + reloc.addend,
                });

                //this is is only really used for debugging, hence symtab only
                sc_symtab.push(Symbol {
                    shndx: SymbolSectionIndex::Section(sh_index_got as u16),
                    value: sc_dynsym[dynsym_index_got].value + got_slot,
                    size: 8,
                    name: sc_dynsym[reloc.sym as usize].name.clone() + "@GOT",
                    stype: types::SymbolType::OBJECT,
                    bind: types::SymbolBind::LOCAL,
                    vis: types::SymbolVis::DEFAULT,
                });

                if sc_dynsym[reloc.sym as usize].shndx == SymbolSectionIndex::Undefined {
                    assert!(sc_dynsym[reloc.sym as usize].bind == types::SymbolBind::WEAK);
                    println!(
                        "GOT slot at 0x{:x} remains 0 for undefined weak symbol {}",
                        sc_dynsym[dynsym_index_got].value + got_slot,
                        sc_dynsym[reloc.sym as usize].name
                    );
                } else {
                    sc_rela.push(Relocation {
                        addr: sc_dynsym[dynsym_index_got].value + got_slot,
                        sym: reloc.sym,
                        rtype: RelocationType::R_X86_64_64,
                        addend: 0,
                    });
                }
            }

            RelocationType::R_X86_64_PLT32 => {
                reloc.rtype = RelocationType::R_X86_64_PC32;
                //reloc.addend = 0;
                sc_rela.push(reloc);
            }
            _ => {
                sc_rela.push(reloc);
            }
        }
    }

    //reposition all the symbols for symtab
    for sym in &mut lookup.symbols {
        if let SymbolSectionIndex::Global(id) = sym.shndx {
            sym.value += out_elf.sections[global2section[&id]].header.addr;
            sym.shndx = SymbolSectionIndex::Section(global2section[&id] as u16);
            sym.bind = types::SymbolBind::LOCAL;
        }
    }
    out_elf.header.entry = lookup.get_by_name("_start").unwrap().value;
    sc_symtab.extend(lookup.symbols);


    //store on .dynamic may add strings to dynsym, which will change all the offsets.
    //this is why dynstr is last. this index needs to be changed everytime something is added
    //between here and dynstr
    let sh_index_dynstr = out_elf.sections.len() + 3;

    let sh_index_dynsym = out_elf.sections.len();
    let symhash = elfkit::symbol::symhash(&out_elf.header, &sc_dynsym, sh_index_dynsym as u32)
        .expect("error writing symhash");
    let first_global_dynsym = sc_dynsym
        .iter()
        .enumerate()
        .find(|&(_, s)| s.bind == types::SymbolBind::GLOBAL)
        .map(|(i, _)| i)
        .unwrap_or(0);
    out_elf.sections.push(Section::new(
        String::from(".dynsym"),
        types::SectionType::DYNSYM,
        types::SectionFlags::ALLOC,
        SectionContent::Symbols(sc_dynsym),
        sh_index_dynstr as u32,
        first_global_dynsym as u32,
    ));

    out_elf.sections.push(symhash);


    sc_rela.sort_unstable_by(|a, _| if a.rtype == RelocationType::R_X86_64_RELATIVE {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    });

    out_elf.sections.push(Section::new(
        String::from(".rela.dyn"),
        types::SectionType::RELA,
        types::SectionFlags::ALLOC,
        SectionContent::Relocations(sc_rela),
        sh_index_dynsym as u32,
        0,
    ));


    out_elf.sections.push(Section::new(
        String::from(".dynstr"),
        types::SectionType::STRTAB,
        types::SectionFlags::ALLOC,
        SectionContent::Strtab(Strtab::default()),
        0,
        0,
    ));


    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();

    sc_dynamic.extend(linker::dynamic(&out_elf).unwrap());
    out_elf.sections.push(Section::new(
        String::from(".dynamic"),
        types::SectionType::DYNAMIC,
        types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
        SectionContent::Dynamic(sc_dynamic),
        sh_index_dynstr as u32,
        0,
    ));

    let sh_index_strtab = out_elf.sections.len();
    out_elf.sections.push(Section::new(
        String::from(".strtab"),
        types::SectionType::STRTAB,
        types::SectionFlags::empty(),
        SectionContent::Strtab(Strtab::default()),
        0,
        0,
    ));

    //sc_symtab.sort_unstable_by(|a,b| a.bind.cmp(&b.bind));
    let first_global_symtab = sc_symtab
        .iter()
        .enumerate()
        .find(|&(_, s)| s.bind == types::SymbolBind::GLOBAL)
        .map(|(i, _)| i)
        .unwrap_or(0);
    out_elf.sections.push(Section::new(
        String::from(".symtab"),
        types::SectionType::SYMTAB,
        types::SectionFlags::empty(),
        SectionContent::Symbols(sc_symtab),
        sh_index_strtab as u32,
        first_global_symtab as u32,
    ));

    out_elf.sections.push(Section::new(
        String::from(".shstrtab"),
        types::SectionType::STRTAB,
        types::SectionFlags::from_bits_truncate(0),
        SectionContent::Strtab(Strtab::default()),
        0,
        0,
    ));

    out_elf.sync_all().unwrap();
    linker::relayout(&mut out_elf, 0x300).unwrap();
    out_elf.segments = linker::segments(&out_elf).unwrap();
    out_elf.store_all().unwrap();
    out_elf.to_writer(&mut out_file).unwrap();

    let mut perms = out_file.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    out_file.set_permissions(perms).unwrap();
}


fn resursive_insert(
    candidates: &mut HashMap<u64, Unit>,
    lookup: &mut Lookup,
    unit: Unit,
    promise_insert: &mut HashSet<u64>,
) {
    promise_insert.insert(unit.global_id);

    for id in &unit.deps {
        if *id != unit.global_id && !promise_insert.contains(id) && lookup.by_id.get(id) == None {
            match candidates.remove(&id) {
                Some(unit) => {
                    resursive_insert(candidates, lookup, unit, promise_insert);
                }
                None => panic!(
                    "bug in elfkit linker: {} dependant unit {} not found",
                    unit.name,
                    id
                ),
            };
        }
    }
    lookup.insert_unit(unit);
}



fn load_elfs(paths: Vec<String>) -> Vec<(String, Elf)> {
    let mut elfs = Vec::new();
    for in_path in paths {
        let mut in_file = match OpenOptions::new().read(true).open(&in_path) {
            Ok(f) => f,
            Err(e) => {
                fail(format!("while loading '{}' : {:?}", in_path, e));
            }
        };
        let in_name = Path::new(&in_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        match filetype::filetype(&in_file).unwrap() {
            filetype::FileType::Elf => {
                elfs.push((
                    in_name,
                    match Elf::from_reader(&mut in_file) {
                        Ok(e) => e,
                        Err(e) => {
                            fail(format!("error loading {} : {:?}", in_path, e));
                        }
                    },
                ));
            }
            filetype::FileType::Archive => {
                let mut buffer = Vec::new();
                in_file.read_to_end(&mut buffer).unwrap();
                match goblin::Object::parse(&buffer).unwrap() {
                    goblin::Object::Archive(archive) => {
                        for (name, member, _) in archive.summarize() {
                            let mut io = Cursor::new(
                                &buffer[member.offset as usize
                                            ..member.offset as usize + member.header.size],
                            );

                            match Elf::from_reader(&mut io) {
                                Ok(e) => elfs.push((String::from(name), e)),
                                Err(e) => {
                                    println!(
                                        "{}",
                                        format!("skipping {} in {}: {:?}", name, in_path, e)
                                            .yellow()
                                    );
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                fail(format!("{}: unknown file type", in_name));
            }
        }
    }
    elfs
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkBehaviour {
    Static,
    Dynamic,
}
impl Default for LinkBehaviour {
    fn default() -> LinkBehaviour {
        LinkBehaviour::Static
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnitSegment {
    Executable,
    Data,
    Bss,
}
impl Default for UnitSegment {
    fn default() -> UnitSegment {
        UnitSegment::Executable
    }
}

#[derive(Default)]
struct Unit {
    pub global_id: u64,
    pub name: String,
    pub behaviour: LinkBehaviour,
    pub segment: UnitSegment,
    pub code: Vec<u8>,
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<Relocation>,
    pub deps: Vec<u64>,

    s_lookup: HashMap<String, usize>,
}


impl Unit {
    pub fn fake(name: String, behaviour: LinkBehaviour, symbols: Vec<Symbol>) -> Unit {
        let mut s_lookup = HashMap::new();
        for (i, sym) in symbols.iter().enumerate() {
            s_lookup.insert(sym.name.clone(), i);
        }

        Unit {
            global_id: 0,
            name: name,
            behaviour: behaviour,
            segment: UnitSegment::Bss,
            code: vec![0; 8],
            symbols: symbols,
            relocations: Vec::new(),
            s_lookup: s_lookup,
            deps: Vec::new(),
        }
    }


    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        match self.s_lookup.get(name) {
            None => None,
            Some(i) => self.symbols.get(*i),
        }
    }

    pub fn from_elf(name: String, mut elf: Elf, global_id_counter: &mut u64) -> Vec<Unit> {
        let behaviour = match elf.header.etype {
            types::ElfType::DYN => LinkBehaviour::Dynamic,
            _ => LinkBehaviour::Static,
        };

        assert!(
            behaviour == LinkBehaviour::Static,
            format!("{}: linking to dynamic libraries is not implemented", name)
        );

        let mut sec2global = HashMap::new();
        let mut units = HashMap::new();
        let mut symbols = (0, Vec::new());
        let mut relas = Vec::new();

        for i in elf.sections
            .iter()
            .enumerate()
            .filter_map(|(i, ref sec)| match sec.header.shtype {
                types::SectionType::SYMTAB |
                types::SectionType::DYNSYM |
                types::SectionType::RELA |
                types::SectionType::NOBITS |
                types::SectionType::PROGBITS => Some(i),
                _ => None,
            })
            .collect::<Vec<usize>>()
            .iter()
        {
            elf.load_at(*i).unwrap();
            let mut sec = std::mem::replace(&mut elf.sections[*i], Section::default());

            match sec.header.shtype {
                types::SectionType::PROGBITS | types::SectionType::NOBITS
                    if sec.header.flags.contains(types::SectionFlags::ALLOC) =>
                {
                    *global_id_counter += 1;
                    sec2global.insert(*i, *global_id_counter);

                    units.insert(
                        *i,
                        Unit {
                            global_id: *global_id_counter,
                            name: sec.name.clone() + "." + &name.clone(),
                            behaviour: behaviour.clone(),
                            segment: if sec.header.shtype == types::SectionType::NOBITS {
                                UnitSegment::Bss
                            } else if sec.header.flags.contains(types::SectionFlags::EXECINSTR) {
                                UnitSegment::Executable
                            } else {
                                UnitSegment::Data
                            },
                            code: if sec.header.shtype == types::SectionType::NOBITS {
                                vec![0; sec.header.size as usize]
                            } else {
                                sec.content.into_raw().unwrap()
                            },
                            symbols: Vec::new(),
                            relocations: Vec::new(),
                            s_lookup: HashMap::new(),
                            deps: Vec::new(),
                        },
                    );
                }
                types::SectionType::SYMTAB if behaviour == LinkBehaviour::Static => {
                    symbols = (*i, sec.content.into_symbols().unwrap());
                }
                types::SectionType::DYNSYM if behaviour == LinkBehaviour::Dynamic => {
                    symbols = (*i, sec.content.into_symbols().unwrap());
                }
                types::SectionType::RELA => {
                    relas.push((sec.header, sec.content.into_relocations().unwrap()));
                }
                _ => {}
            }
        }

        for (obj_shndx, ref mut obj) in units.iter_mut() {
            let mut smap = HashMap::new();

            // copy all symbols from symtab where .shndx is this obj
            for (i, sym) in symbols.1.iter().enumerate() {
                if sym.shndx == SymbolSectionIndex::Section(*obj_shndx as u16) {
                    let mut sym = sym.clone();
                    sym.shndx = SymbolSectionIndex::Global(obj.global_id);
                    smap.insert(i, obj.symbols.len());
                    obj.s_lookup.insert(sym.name.clone(), obj.symbols.len());
                    obj.symbols.push(sym);
                }
            }

            // for all refs where .info is this obj
            for &(ref header, ref relas) in &relas {
                if header.info == *obj_shndx as u32 {
                    if header.link != symbols.0 as u32 {
                        fail(format!(
                            "{}: reloc section {} references unexpected or duplicated symbols \
                             section",
                            name,
                            header.name
                        ));
                    }
                    // also copy the rest of the symbols needed for reloc
                    for rela in relas {
                        if smap.get(&(rela.sym as usize)) == None {
                            let mut sym = symbols.1[rela.sym as usize].clone();

                            // if it's a global or weak, undef it,
                            // so it's actually looked up in other units

                            if sym.bind != types::SymbolBind::LOCAL {
                                sym.shndx = SymbolSectionIndex::Undefined;
                            } else if let SymbolSectionIndex::Section(sx) = sym.shndx {
                                sym.shndx = match sec2global.get(&(sx as usize)) {
                                    None => fail(format!(
                                        "{} rela {:?} -> {:?} to section {} which is not allocated",
                                        obj.name,
                                        rela,
                                        sym,
                                        sx
                                    )),
                                    Some(id) => {
                                        obj.deps.push(*id);
                                        SymbolSectionIndex::Global(*id)
                                    }
                                };
                            };
                            smap.insert(rela.sym as usize, obj.symbols.len());
                            obj.s_lookup.insert(sym.name.clone(), obj.symbols.len());
                            obj.symbols.push(sym);
                        }
                        let mut rela = rela.clone();
                        rela.sym = *smap.get(&(rela.sym as usize)).unwrap() as u32;
                        obj.relocations.push(rela);
                    }
                }
            }
        }

        let mut units = units.into_iter().map(|(_, v)| v).collect::<Vec<Unit>>();


        //we can emit COMMON symbols as WEAK in a bss
        //because we're smart enough to only layout the bss that's actually used.
        for sym in symbols.1.iter() {
            if sym.shndx == SymbolSectionIndex::Common {
                *global_id_counter += 1;

                let symname = sym.name.clone();
                let symsize = sym.size;
                let mut sym = sym.clone();
                sym.value = 0;
                sym.shndx = SymbolSectionIndex::Global(*global_id_counter);
                sym.bind = types::SymbolBind::WEAK;
                let mut symbols = vec![sym];
                let mut s_lookup = HashMap::new();
                s_lookup.insert(symname.clone(), 0);

                assert!(
                    behaviour == LinkBehaviour::Static,
                    "cannot use common symbol with dynamic linking"
                );
                units.push(Unit {
                    global_id: *global_id_counter,
                    name: String::from(".common.") + &symname,
                    behaviour: behaviour.clone(),
                    segment: UnitSegment::Bss,
                    code: vec![0; symsize as usize],
                    symbols: symbols,
                    relocations: Vec::new(),
                    s_lookup: s_lookup,
                    deps: Vec::new(),
                });
            }
        }

        //TODO we need to extract things like .init_array* , .preinit_array.* , ...
        //they have dependencies the opposite way. i.e a reloc on .init_array.bla will point into
        //.bla, because the function in .init_array.bla does stuff to memory in .bla



        // this is nessesary because compilers assume that they can emit sections into objects which
        // will be made available for linking when some other section of that object gets linked in
        // TODO this seems  only relevant when that other section actually has symbols? hopefully
        let deps: Vec<u64> = units
            .iter()
            .filter_map(|unit| {
                for sym in &unit.symbols {
                    if sym.shndx != SymbolSectionIndex::Undefined
                        && sym.bind != types::SymbolBind::LOCAL
                    {
                        return Some(unit.global_id);
                    }
                }
                None
            })
            .collect();

        for unit in &mut units {
            unit.deps.extend(&deps);
        }

        units
    }
}

#[derive(Default)]
struct Lookup {
    pub units: Vec<Unit>,
    pub by_id: HashMap<u64, usize>,

    pub symbols: Vec<Symbol>,
    pub by_name: HashMap<String, usize>,

    pub symbols2units: HashMap<usize, usize>,
}

impl Lookup {
    fn get_by_name(&self, name: &str) -> Option<&Symbol> {
        match self.by_name.get(name) {
            Some(v) => self.symbols.get(*v),
            None => None,
        }
    }

    pub fn insert_unit(&mut self, unit: Unit) {
        let ui = self.units.len();

        for sym in &unit.symbols {
            self.insert_symbol(sym.clone(), ui, &unit.name);
        }

        self.by_id.insert(unit.global_id.clone(), self.units.len());
        self.units.push(unit);
    }


    fn symbol_lookup_priority(s1: &Symbol, s2: &Symbol) -> usize {
        if s1.shndx == SymbolSectionIndex::Undefined {
            // we don't check if s2 is undefined here, because it won't matter.
            // if both are undefined, we can just pick a random one.
            return 2;
        }
        if s2.shndx == SymbolSectionIndex::Undefined {
            // we skip the rest of the checks here too.
            // any defition beats undefined
            return 1;
        }

        if s1.bind == types::SymbolBind::GLOBAL {
            if s2.bind == types::SymbolBind::GLOBAL {
                // can't have two defined global
                return 0;
            } else {
                return 1;
            }
        }

        // 1 wasn't global, so it can only be WEAK
        // 2 is either GLOBAL or WEAK, so it either beats 1 or it doesnt matter
        return 2;
    }


    fn insert_symbol(&mut self, sym: Symbol, unit_index: usize, obj_name: &str) -> usize {
        match sym.stype {
            types::SymbolType::NOTYPE | types::SymbolType::OBJECT | types::SymbolType::FUNC => {
                if sym.bind == types::SymbolBind::LOCAL {
                    return 0;
                }
                match self.by_name.entry(sym.name.clone()) {
                    Entry::Vacant(o) => {
                        let i = self.symbols.len();
                        o.insert(i);
                        self.symbols.push(sym);
                        self.symbols2units.insert(i, unit_index);
                        i
                    }
                    Entry::Occupied(o) => {
                        let sym2 = &mut self.symbols[*o.get()];

                        match Lookup::symbol_lookup_priority(&sym, sym2) {
                            1 => {
                                std::mem::replace(sym2, sym);
                                self.symbols2units.insert(*o.get(), unit_index);
                            }
                            2 => {}
                            _ => {
                                fail(format!(
                                    "{}: re-export of symbol {:?} \n   already defined in {} as \
                                     {:?}",
                                    obj_name,
                                    sym,
                                    self.units[self.symbols2units[o.get()]].name,
                                    sym2
                                ));
                            }
                        }
                        *o.get()
                    }
                }
            }
            _ => 0,
        }
    }
}
