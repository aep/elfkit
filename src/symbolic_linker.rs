extern crate ordermap;

use {Header, types, symbol, relocation, section, Error};
use std;
use std::io::Write;
use std::collections::hash_map::{self, HashMap};
use loader::{self, Loader};
use std::sync::atomic::{self, AtomicUsize};

pub type LinkGlobalId = usize;

pub struct LinkableSymbol {
    pub obj:    LinkGlobalId,
    pub sym:    symbol::Symbol,
}

pub struct Object {
    /// the link layout global id, assigned by the symbolic linker
    pub lid:        LinkGlobalId,

    /// name of the object + section name
    pub name:       String,

    /// copy of the original objects elf Header
    pub header:     Header,

    /// the actual section extracted from the object
    pub section:    section::Section,

    /// relocations that need to be applied to this section
    /// reloc.sym points at SymbolicLinker.symtab
    pub relocs:     Vec<relocation::Relocation>,

    // global source object id. this is used for debugging
    oid:        LinkGlobalId,
}

#[derive(Default)]
pub struct SymbolicLinker {
    pub objects: HashMap<LinkGlobalId, Object>,
    pub symtab:  Vec<LinkableSymbol>,

    lookup:      HashMap<Vec<u8>, usize>,
    lid_counter: AtomicUsize,
}

impl SymbolicLinker {
    pub fn link_all(&mut self, loader: Vec<loader::State>) -> Result<(), Error> {
        let loader = loader.load_all(&|e,name| {
            println!("elfkit::Linker {:?} while loading {}", e, name);
            Vec::with_capacity(0)
        });
        self.objects.reserve(loader.len());
        for ma in loader {
            if let loader::State::Object{name, header, symbols, sections} = ma {
                self.insert_object(name, header, symbols, sections)?;
            }
        }
        Ok(())
    }
    pub fn link(&mut self, mut loader: Vec<loader::State>) -> Result<(), Error> {
        loop {
            let (l2, matches) = self.link_iteration(loader);
            loader = l2;
            if matches.len() == 0 {
                for link in self.symtab.iter() {
                    if link.sym.shndx == symbol::SymbolSectionIndex::Undefined &&
                        link.sym.bind == types::SymbolBind::GLOBAL {
                        return Err(Error::UndefinedReference{
                            obj: self.objects[&link.obj].name.clone(),
                            sym: String::from_utf8_lossy(&link.sym.name).into_owned(),
                        });
                    }
                }
                break;
            }

            self.objects.reserve(matches.len());
            for ma in matches {
                if let loader::State::Object{name, header, symbols, sections} = ma {
                    self.insert_object(name, header, symbols, sections)?;
                }
            }
        }
        Ok(())
    }

    fn link_iteration(&mut self, loader: Vec<loader::State>) -> (Vec<loader::State>, Vec<loader::State>) {
        let (state2, matches) : (Vec<loader::State>, Vec<loader::State>) = {
            let undefined_refs = self.symtab.iter().filter_map(|link|{
                match link.sym.shndx {
                    symbol::SymbolSectionIndex::Undefined => {
                        if link.sym.bind == types::SymbolBind::GLOBAL {
                            Some(link.sym.name.as_ref())
                        } else {
                            None
                        }
                    },
                    symbol::SymbolSectionIndex::Common => {
                        //note that this will only pull in objects that have this symbol as global,
                        //not those who merely also define it as common
                        Some(link.sym.name.as_ref())
                    },
                    _ => None,
                }
            }).collect();

            loader.load_if(&undefined_refs, &|e,name| {
                println!("elfkit::Linker {:?} while loading {}", e, name);
                Vec::with_capacity(0)
            })
        };
        (state2, matches)
    }

    fn insert_object(&mut self, name: String, header: Header, symbols: Vec<symbol::Symbol>,
                     sections: Vec<(usize, section::Section, Vec<relocation::Relocation>)>)
        -> Result<(), Error>  {

        assert!((sections.len() as u16) <= header.shnum,
        "incoming object header.shnum is {} but loader gave us {} sections ", header.shnum, sections.len());
        let lid_base = self.lid_counter.fetch_add(header.shnum as usize, atomic::Ordering::Acquire);

        let locations = match self.link_locations(lid_base, symbols) {
            Ok(v) => v,
            Err(Error::ConflictingSymbol{sym, con, ..}) => {
                return Err(Error::ConflictingSymbol{sym, con, obj:name});
            },
            Err(e) => return Err(e),
        };


        let name = name.split("/").last().unwrap().to_owned();
        for (sec_shndx, sec, mut relocs) in sections {

            // point the relocs at the global symtab
            for reloc in &mut relocs {
                reloc.sym = locations[reloc.sym as usize] as u32;
            };

            self.objects.insert(lid_base + sec_shndx as usize, Object {
                oid:        lid_base,
                lid:        lid_base + sec_shndx as usize,
                name:       name.clone() + "("+ &String::from_utf8_lossy(&sec.name) + ")",
                header:     header.clone(),
                section:    sec,
                relocs:     relocs,
            });
        }

        // TODO insert a fake object at base + 0 so error messages
        // can correctly report the name of an object when a Needed
        // symbol isn't statisfied
        // this is a bit hackish tho, and we need to rely on gc()
        // to remove the crap object before layout

        self.objects.insert(lid_base, Object {
            oid:        lid_base,
            lid:        lid_base,
            name:       name.clone(),
            header:     header.clone(),
            section:    section::Section::default(),
            relocs:     Vec::new(),
        });

        Ok(())
    }

    fn link_locations(&mut self, lid_base: LinkGlobalId, symbols: Vec<symbol::Symbol>)
        -> Result<Vec<usize>, Error> {

        let mut locations = Vec::with_capacity(symbols.len());
        for mut sym in symbols {
            match sym.shndx {
                symbol::SymbolSectionIndex::Undefined => {
                    if sym.name == b"_GLOBAL_OFFSET_TABLE_" {
                        //emit as not linkable, because nothing should relocate here
                        //the symbol appears to be mainly a hint that the linker needs to
                        //emit a GOT. which it knows from relocs anyway, so this appears to
                        //be kinda useless. We could emit a fake symbol to statisy it,
                        //but i want to ensure really nothing actually uses this symbol.
                        //Absolute will show up as error when a reloc points to it.
                        sym.shndx = symbol::SymbolSectionIndex::Absolute;
                    }
                    if sym.bind == types::SymbolBind::LOCAL {
                        if sym.name.len() > 0 {
                            panic!("local undefined symbol {:?}", sym);
                        }
                    }
                    let gsi = match self.lookup.entry(sym.name.clone()) {
                        hash_map::Entry::Occupied(e) => {
                            *e.get()
                        },
                        hash_map::Entry::Vacant(e) => {
                            let i = self.symtab.len();
                            self.symtab.push(LinkableSymbol{sym: sym, obj: lid_base});
                            e.insert(i);
                            i
                        },
                    };
                    locations.push(gsi);
                },
                symbol::SymbolSectionIndex::Common => {
                    let gsi = match self.lookup.entry(sym.name.clone()) {
                        hash_map::Entry::Occupied(e) => {
                            let i = *e.get();
                            if let symbol::SymbolSectionIndex::Undefined = self.symtab[i].sym.shndx {
                                self.symtab[i] = LinkableSymbol{sym: sym, obj: lid_base};
                            } else {
                                //TODO check that the existing symbol is common with the same size
                            }
                            i
                        },
                        hash_map::Entry::Vacant(e) => {
                            let i = self.symtab.len();
                            self.symtab.push(LinkableSymbol{sym: sym, obj: lid_base});
                            e.insert(i);
                            i
                        },
                    };
                    locations.push(gsi);
                },
                symbol::SymbolSectionIndex::Absolute  => {
                    locations.push(self.symtab.len());
                    self.symtab.push(LinkableSymbol{sym: sym, obj: lid_base});
                },
                symbol::SymbolSectionIndex::Section(shndx)  => {
                    match sym.bind {
                        types::SymbolBind::GLOBAL => {
                            let gsi = match self.lookup.entry(sym.name.clone()) {
                                hash_map::Entry::Occupied(e) => {
                                    let i = *e.get();
                                    if let symbol::SymbolSectionIndex::Section(_) = self.symtab[i].sym.shndx {
                                        if self.symtab[i].sym.bind != types::SymbolBind::WEAK {
                                            if self.objects[&self.symtab[i].obj].name.contains("::") {
                                                println!("conflicting definitions of {} \
                                                    ignored because for gnu compatibility. picking {}",
                                                    String::from_utf8_lossy(&self.symtab[i].sym.name),
                                                    self.objects[&self.symtab[i].obj].name.clone()
                                                    );
                                            } else {
                                                return Err(Error::ConflictingSymbol{
                                                    sym:   String::from_utf8_lossy(&self.symtab[i].sym.name)
                                                        .into_owned(),
                                                        obj:   String::default(),
                                                        con:   self.objects[&self.symtab[i].obj].name.clone(),
                                                });
                                            }
                                        }
                                    };
                                    self.symtab[i] = LinkableSymbol{sym: sym,
                                    obj: lid_base + shndx as usize};
                                    i
                                },
                                hash_map::Entry::Vacant(e) => {
                                    let i = self.symtab.len();
                                    self.symtab.push(LinkableSymbol{sym: sym, obj: lid_base + shndx as usize});
                                    e.insert(i);
                                    i
                                },
                            };
                            locations.push(gsi);
                        },
                        types::SymbolBind::WEAK => {
                            let gsi = match self.lookup.entry(sym.name.clone()) {
                                hash_map::Entry::Occupied(e) => {
                                    let i = e.get();
                                    if let symbol::SymbolSectionIndex::Undefined = self.symtab[*i].sym.shndx {
                                        self.symtab[*i] = LinkableSymbol{sym: sym,
                                            obj: lid_base + shndx as usize};
                                    };
                                    *i
                                },
                                hash_map::Entry::Vacant(e) => {
                                    let i = self.symtab.len();
                                    self.symtab.push(LinkableSymbol{sym: sym,
                                        obj: lid_base + shndx as usize});
                                    e.insert(i);
                                    i
                                },
                            };
                            locations.push(gsi);
                        }
                        _ => {
                            locations.push(self.symtab.len());
                            self.symtab.push(LinkableSymbol{sym: sym, obj: lid_base + shndx as usize});
                        },
                    }
                },
            }
        }
        Ok(locations)
    }

    //TODO: maybe too aggressive because stuff like .comment and .note.GNU-stack are culled?
    pub fn gc(&mut self) {

        let mut again = true;
        let mut symtab_remap : Vec<Option<usize>> = vec![None;self.symtab.len()];
        while again {
            symtab_remap = vec![None;self.symtab.len()];
            let mut removelids = HashMap::new();
            for (lid, obj) in &self.objects {

                //TODO yep yep, more hacks
                if obj.section.header.shtype == types::SectionType::INIT_ARRAY ||
                   obj.section.header.shtype == types::SectionType::FINI_ARRAY {
                   continue;
                }
                removelids.insert(*lid, true);
            }

            for (lid, obj) in &self.objects {
                //TODO oh look, more hacks
                if obj.section.name.starts_with(b".debug_") {
                    continue;
                }

                for reloc in &obj.relocs {
                    symtab_remap[reloc.sym as usize] = Some(0);

                    let link = &self.symtab[reloc.sym as usize];

                    if link.obj != *lid {
                        if let symbol::SymbolSectionIndex::Section(_) = link.sym.shndx {
                            removelids.insert(link.obj, false);
                        }
                    }
                }

            }

            //TODO this feels like a hack. I think we should be able to mark root nodes before gc
            if let Some(i) = self.lookup.get(&(b"_start".to_vec())) {
                removelids.insert(self.symtab[*i].obj, false);
            }


            again = false;
            for (lid, t) in removelids {
                if t {
                    again = true;
                    self.objects.remove(&lid);
                } else {
                    for (i, sym) in self.symtab.iter().enumerate() {
                        if sym.obj == lid {
                            symtab_remap[i] = Some(0);
                        }
                    }
                }
            }
        }

        let mut symtab = Vec::new();

        for (i, link)  in self.symtab.drain(..).enumerate() {
            if link.sym.shndx == symbol::SymbolSectionIndex::Absolute {
                symtab_remap[i] = Some(0);
            }
            if let Some(_) = symtab_remap[i] {
                symtab_remap[i] = Some(symtab.len());
                symtab.push(link);
            }
        }

        for (_, obj) in &mut self.objects {
            for reloc in &mut obj.relocs {
                reloc.sym = symtab_remap[reloc.sym as usize]
                    .expect("bug in elfkit: dangling reloc after gc") as u32;
            }
        }

        self.symtab = symtab;

    }


    pub fn write_graphviz<W : Write> (&self, mut file: W) -> std::io::Result<()> {

        for (lid, object) in self.objects.iter() {

            writeln!(file, "    o{}[group=g{}, label=\"<f0>{}|<f1> {}\"];",
                     lid, object.oid, object.oid, object.name)?;


            for reloc in &object.relocs {
                let link = &self.symtab[reloc.sym as usize];

                if link.obj != object.lid {
                    let mut style  = String::new();
                    let mut linkto = format!("o{}", link.obj);
                    let label  = String::from_utf8_lossy(&link.sym.name).to_owned();

                    if link.sym.bind == types::SymbolBind::WEAK {
                        style = String::from(", style=\"dashed\"");
                    };
                    if link.sym.shndx == symbol::SymbolSectionIndex::Common {
                        writeln!(file, "    common_{}[label=\"COMMON {}\", style=\"dotted\"];",
                                 String::from_utf8_lossy(&link.sym.name),
                                 String::from_utf8_lossy(&link.sym.name))?;

                        style  = String::from(", style=\"dotted\"");
                        linkto = format!("common_{}", String::from_utf8_lossy(&link.sym.name));
                    }
                    if link.sym.shndx == symbol::SymbolSectionIndex::Undefined {
                        writeln!(file, "    missing_{}[label=\"UNDEFINED {}\", color=\"red\", style=\"dashed\", fontcolor=\"red\"];",
                                 String::from_utf8_lossy(&link.sym.name),
                                 String::from_utf8_lossy(&link.sym.name)
                                )?;
                        style += ", color=\"red\"";
                        linkto = format!("missing_{}", String::from_utf8_lossy(&link.sym.name));
                    }

                    writeln!(file, "    o{} -> {} [label=\"{}\" {}]",
                             object.lid, linkto, label, style)?;
                }
            }
        }

        Ok(())
    }
}
