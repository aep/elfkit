use std::collections::HashMap;
use std::collections::HashSet;
use std;
use ::fail;
use std::collections::hash_map::Entry;

use elfkit::{
    Elf, Header, types, SegmentHeader, Section, SectionContent, Error,
    SectionHeader, Dynamic, Symbol, Relocation, Strtab, SymbolSectionIndex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkBehaviour {
    Static,
    Dynamic
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
    Tls,
}
impl Default for UnitSegment{
    fn default() -> UnitSegment {
        UnitSegment::Executable
    }
}

#[derive(Default)]
pub struct Unit {
    pub global_id:   u64,
    pub name:        String,
    pub behaviour:   LinkBehaviour,
    pub segment:     UnitSegment,
    pub code:        Vec<u8>,
    pub symbols:     Vec<Symbol>,
    pub relocations: Vec<Relocation>,
    pub deps:        Vec<u64>,

    s_lookup: HashMap<String, usize>,
}


impl Unit {
    pub fn fake(name: String, behaviour: LinkBehaviour, symbols: Vec<Symbol>) -> Unit {

        let mut s_lookup = HashMap::new();
        for (i, sym) in symbols.iter().enumerate() {
            s_lookup.insert(sym.name.clone(), i);
        }

        Unit {
            global_id:  0,
            name:       name,
            behaviour:  behaviour,
            segment:    UnitSegment::Bss,
            code:       vec![0;8],
            symbols:    symbols,
            relocations:Vec::new(),
            s_lookup:   s_lookup,
            deps:   Vec::new(),
        }
    }


    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        match self.s_lookup.get(name) {
            None => None,
            Some(i) => self.symbols.get(*i),
        }
    }

    pub fn from_elf(name: String, mut elf: Elf, global_id_counter: &mut u64) -> Vec<Unit>{
        let behaviour = match elf.header.etype {
            types::ElfType::DYN => LinkBehaviour::Dynamic,
            _ => LinkBehaviour::Static,
        };

        assert!(behaviour == LinkBehaviour::Static,
                format!("{}: linking to dynamic libraries is not implemented", name));

        let mut sec2global  = HashMap::new();
        let mut units       = HashMap::new();
        let mut symbols     = (0, Vec::new());
        let mut relas       = Vec::new();

        for i in elf.sections.iter().enumerate().filter_map(|(i, ref sec)| {
            match sec.header.shtype {
                types::SectionType::SYMTAB |
                    types::SectionType::DYNSYM |
                    types::SectionType::RELA   |
                    types::SectionType::NOBITS |
                    types::SectionType::PROGBITS => Some (i),
                _ => None,
            }
        }).collect::<Vec<usize>>().iter() {
            elf.load_at(*i).unwrap();
            let sec = std::mem::replace(&mut elf.sections[*i], Section::default());

            match sec.header.shtype {
                types::SectionType::PROGBITS |
                    types::SectionType::NOBITS
                    if sec.header.flags.contains(types::SectionFlags::ALLOC)  => {
                        *global_id_counter += 1;
                        sec2global.insert(*i, *global_id_counter);

                        units.insert(*i, Unit{
                            global_id:  *global_id_counter,
                            name:       sec.name.clone() + "." + &name.clone(),
                            behaviour:  behaviour.clone(),
                            segment:    if sec.header.shtype == types::SectionType::NOBITS {
                                UnitSegment::Bss
                            } else if sec.header.flags.contains(types::SectionFlags::TLS) {
                                UnitSegment::Tls
                            } else if sec.header.flags.contains(types::SectionFlags::EXECINSTR) {
                                UnitSegment::Executable
                            } else {
                                UnitSegment::Data
                            },
                            code: if sec.header.shtype == types::SectionType::NOBITS {
                                vec![0;sec.header.size as usize]
                            } else {
                                sec.content.into_raw().unwrap()
                            },
                            symbols:        Vec::new(),
                            relocations:    Vec::new(),
                            s_lookup:       HashMap::new(),
                            deps:       Vec::new(),
                        });
                    },
                    types::SectionType::SYMTAB if behaviour == LinkBehaviour::Static => {
                        symbols = (*i, sec.content.into_symbols().unwrap());
                    },
                    types::SectionType::DYNSYM if behaviour == LinkBehaviour::Dynamic => {
                        symbols = (*i, sec.content.into_symbols().unwrap());
                    },
                    types::SectionType::RELA => {
                        relas.push((sec.header, sec.content.into_relocations().unwrap()));
                    },
                    _ => {},
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
                if header.info == *obj_shndx  as u32 {
                    if header.link != symbols.0 as u32{
                        fail(format!("{}: reloc section {} references unexpected or duplicated symbols section",
                                     name, header.name));
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
                                    None => fail(format!("{} rela {:?} -> {:?} to section {} which is not allocated",
                                                         obj.name, rela, sym, sx)),
                                    Some(id) => {
                                        obj.deps.push(*id);
                                        SymbolSectionIndex::Global(*id)
                                    },
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

        let mut units = units.into_iter().map(|(k,v)|v).collect::<Vec<Unit>>();


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
                sym.bind  = types::SymbolBind::WEAK;
                let mut symbols = vec![sym];
                let mut s_lookup = HashMap::new();
                s_lookup.insert(symname.clone(), 0);

                assert!(behaviour == LinkBehaviour::Static, "cannot use common symbol with dynamic linking");
                units.push(Unit{
                    global_id:      *global_id_counter,
                    name:           String::from(".common.") + &symname,
                    behaviour:      behaviour.clone(),
                    segment:        UnitSegment::Bss,
                    code:           vec![0;symsize as usize],
                    symbols:        symbols,
                    relocations:    Vec::new(),
                    s_lookup:       s_lookup,
                    deps:       Vec::new(),
                });
            }
        }

        //TODO we need to extract things like .init_array* , .preinit_array.* , ...
        //they have dependencies the opposite way. i.e a reloc on .init_array.bla will point into .bla,
        //because the function in .init_array.bla does stuff to memory in .bla



        // this is nessesary because coders assume that they can emit sections into objects which
        // will be made available for linking when some other section of that object gets linked in
        // this seems  only relevant when that other section actually has symbols? hopefully
        // TODO: we can later remove those extra sections again should they not become part of the
        // link, but i'll delay implementing this until the lookup logic has a proper datastructure
        let deps: Vec<u64> = units.iter().filter_map(|unit|{
            for sym in &unit.symbols {
                if sym.stype != types::SymbolType::SECTION &&
                    sym.shndx != SymbolSectionIndex::Undefined &&
                        sym.bind != types::SymbolBind::LOCAL {
                    return Some(unit.global_id);
                }
            }
            None
        }).collect();

        for unit in &mut units {
            unit.deps.extend(&deps);
        }

        units
    }
}

#[derive(Default)]
pub struct Lookup {
    pub units:          Vec<Unit>,
    pub by_id:          HashMap<u64, usize>,

    pub symbols:        Vec<Symbol>,
    pub by_name:        HashMap<String, usize>,

    pub symbols2units:  HashMap<usize, usize>,
}

impl Lookup {
    pub fn get_by_name(&self, name: &str) -> Option<&Symbol> {
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

    pub fn reindex(&mut self) {
        self.by_id.clear();
        for (i,unit) in self.units.iter().enumerate() {
            self.by_id.insert(unit.global_id.clone(), i);
        }
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
            types::SymbolType::NOTYPE | types::SymbolType::OBJECT | types::SymbolType::FUNC | types::SymbolType::TLS => {
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
                    },
                    Entry::Occupied(o) => {
                        let sym2 = &mut self.symbols[*o.get()];

                        match Lookup::symbol_lookup_priority(&sym, sym2) {
                            1 =>  {
                                std::mem::replace(sym2, sym);
                                self.symbols2units.insert(*o.get(), unit_index);
                            },
                            2 => {},
                            _ => {
                                fail(format!(
                                        "{}: re-export of symbol {:?} \n   already defined in {} as {:?}",
                                        obj_name, sym, self.units[self.symbols2units[o.get()]].name, sym2));
                            }
                        }
                        *o.get()
                    }
                }
            },
            _ => {0},
        }
    }


    pub fn link(&mut self, mut elfs: Vec<(String,Elf)>) {
        let mut global_id_counter = 10;
        let mut candidates = HashMap::new();
        loop {
            println!("lookup iteration");
            let missing = self.symbols.iter().enumerate().filter_map(|(i, ref sym)|{
                if sym.shndx == SymbolSectionIndex::Undefined && sym.bind == types::SymbolBind::GLOBAL {
                    Some(i)
                } else {
                    None
                }
            }).collect::<Vec<usize>>();

            if missing.len() < 1 {
                break;
            }

            for mi in missing {
                let mut found = None;
                let was_needed_by = self.units[self.symbols2units[&mi]].name.clone();

                let mut cont = true;
                while cont {
                    cont = false;
                    for ei in 0..elfs.len() {
                        let contains = match elfs[ei].1.contains_symbol(&self.symbols[mi].name) {
                            Ok(v)  => v,
                            Err(e) => fail(format!("error in self in {} : {:?}",
                                                   elfs[ei].0, e)),
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
                    for sym in candidate.lookup(&self.symbols[mi].name) {
                        if sym.shndx != SymbolSectionIndex::Undefined {
                            found = Some(id.clone());
                            break;
                        }
                    }
                }

                if let Some(id) = found  {
                    let unit = candidates.remove(&id).unwrap();
                    self.resursive_insert(&mut candidates, unit, &mut HashSet::new());

                    println!(" - {} <= {} <= {} ", was_needed_by,
                             &self.symbols[mi].name,
                             self.units[self.symbols2units[&mi]].name,
                             );

                    //integrity check
                    if self.symbols[mi].shndx == SymbolSectionIndex::Undefined {
                        panic!("BUG in elfkit lookup: symbol {:?} is still undefined after inserting unit where Unit:self() returned a defined symbol",
                        self.symbols[mi]);
                    }

                } else {
                    let sym = &self.symbols[mi];
                    if sym.shndx == SymbolSectionIndex::Undefined && sym.bind == types::SymbolBind::GLOBAL {
                        fail(format!("{}: undefined reference to {}",
                                     self.units[self.symbols2units[&mi]].name,
                                     self.symbols[mi].name));
                    }
                }
            }
        }
    }

    fn resursive_insert(&mut self, candidates: &mut HashMap<u64, Unit>,
                        unit: Unit, promise_insert: &mut HashSet<u64>) {
        promise_insert.insert(unit.global_id);

        for id in &unit.deps {
            if *id != unit.global_id && !promise_insert.contains(id) && self.by_id.get(id) == None {
                match candidates.remove(&id) {
                    Some(unit) =>  {
                        self.resursive_insert(candidates, unit, promise_insert);
                    },
                    None => panic!("bug in elfkit linker: {} dependant unit {} not found", unit.name, id),
                };
            }
        }
        self.insert_unit(unit);
    }



//FIXME this doesnt belong in Elf



impl Elf {
    /// check if a global defined symbol is exported from the elf file.
    /// can be used to avoid load_all if a particular elf file doesn't
    /// contain the symbol you need anyway.
    /// It uses the cheapest possible method to determine the result
    /// which is currently loading symtab into a hashmap
    /// TODO should be replaced with checking HASH and GNU_HASH
    pub fn contains_symbol(&mut self, name: &str) -> Result<bool, Error> {
        if None == self.s_lookup {
            let mut hm = HashSet::new();

            for i in self.sections
                .iter()
                .enumerate()
                .filter_map(|(i, ref sec)| {
                    if sec.header.shtype == types::SectionType::SYMTAB
                        || sec.header.shtype == types::SectionType::DYNSYM
                    {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect::<Vec<usize>>()
                .iter()
            {
                self.load(*i)?;
                for sym in self.sections[*i].content.as_symbols().unwrap() {
                    if sym.bind != types::SymbolBind::LOCAL
                        && sym.shndx != SymbolSectionIndex::Undefined
                    {
                        hm.insert(sym.name.clone());
                    }
                }
            }

            self.s_lookup = Some(hm);
        }
        Ok(self.s_lookup.as_ref().unwrap().contains(name))
    }
}
}



