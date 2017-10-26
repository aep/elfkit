extern crate ar;
extern crate bit_vec;
extern crate fnv;
extern crate core;
extern crate rayon;

use {types, Header, Elf, Error, symbol, filetype, relocation, section};
use std;
use std::io::{Read, Seek, Cursor, BufReader, SeekFrom};
use std::hash::{Hash,Hasher};
use std::fs::{File};
use std::collections::HashMap;
use std::cell::RefCell;
use self::rayon::prelude::*;
use self::fnv::FnvHasher;
use self::bit_vec::BitVec;
use self::ar::Archive;

pub trait ReadSeekSend : Read + Seek + Send {}
impl<T> ReadSeekSend for T where T: Read + Seek + Send{}

pub enum State {
    Error{
        name:    String,
        error:   Error
    },
    Path{
        name:    String
    },
    Archive{
        name:    String,
        archive: Archive<File>,
    },
    Elf{
        name:    String,
        elf:     Elf,
        read:    RefCell<Box<ReadSeekSend>>,
        bloom:   BloomFilter,
        symbols: Vec<symbol::Symbol>,
    },
    Object{
        name:     String,
        symbols:  Vec<symbol::Symbol>,
        header:   Header,
        sections: Vec<(usize, section::Section, Vec<relocation::Relocation>)>,
    },
}

pub trait Loader {
    fn load_if<E>(self, needles: &Vec<&[u8]>, e: &E) ->  (Vec<State>,Vec<State>)
        where E: Fn(Error, String) -> Vec<State> + Sync;
}

impl Loader for Vec<State> {
    fn load_if<E>(self, needles: &Vec<&[u8]>, e: &E) ->  (Vec<State>,Vec<State>)
        where E: Fn(Error, String) -> Vec<State> + Sync
    {
        self.into_par_iter().flat_map(|l| l.load_if(needles, e))
            .partition(|o| if let &State::Object{..} = o {false} else {true})
    }
}


impl State {
    pub fn load_if<E> (mut self, needles: &Vec<&[u8]>, e: &E) -> Vec<State>
        where E: Fn(Error, String) -> Vec<State> + Sync
    {
        if needles.iter().map(|needle|self.contains(needle, BloomFilter::hash(needle))).any(|e|e==true) {
            self.load(e).into_par_iter().flat_map(|s|s.load_if(needles, e)).collect()
        } else {
            vec![self]
        }
    }

    pub fn contains(&mut self, needle: &[u8], needle_hash: [u64;2]) -> bool {
        match self {
            &mut State::Error{..}   => true,
            &mut State::Path {..}   => true,
            &mut State::Archive{ref mut archive, ..}  => {
                let symbols = match archive.symbols() {
                    Ok(v) =>  v,
                    Err(_) => return true,
                };
                for symbol in symbols {
                    if symbol.as_bytes() == needle {
                        return true;
                    }
                }
                return false;
            },
            &mut State::Elf{ref elf, ref bloom, ref symbols, ..} => {
                if bloom.contains(&needle_hash) {
                    for sym in symbols.iter() {
                        match sym.bind {
                            types::SymbolBind::GLOBAL | types::SymbolBind::WEAK => {
                                match sym.shndx {
                                    symbol::SymbolSectionIndex::Undefined |
                                        symbol::SymbolSectionIndex::Absolute  => {},
                                    _ => {
                                        if sym.name == needle {
                                            return true;
                                        }
                                    },
                                }
                            },
                            _ => {},
                        }
                    }
                }
                return false;
            },
            &mut State::Object{..} => false,
        }
    }

    pub fn load<E>(self, e: &E) -> Vec<State> where E: Fn(Error, String) -> Vec<State> {
        match self {
            State::Error{name,error} => e(error,name),
            State::Path{name} => {
                let mut f = match File::open(&name) {
                    Err(e) => return vec![State::Error{
                        error: Error::from(e),
                        name:  name
                    }],
                    Ok(f) => f,
                };
                match filetype::filetype(&mut f) {
                    Ok(filetype::FileType::Unknown) => {
                        return vec![State::Error{
                            error:  Error::InvalidMagic,
                            name:   name,
                        }];
                    },
                    Ok(filetype::FileType::Elf) => {
                        vec![match State::make_object(name.clone(), RefCell::new(Box::new(f))) {
                            Err(e) => State::Error{
                                error: e,
                                name:  name,
                            },
                            Ok(v) => v,
                        }]
                    },
                    Ok(filetype::FileType::Archive) => {
                        vec![State::Archive{
                            name:    name,
                            archive: Archive::new(f),
                        }]
                    },
                    Err(e) => vec![
                        State::Error{
                            error: Error::from(e),
                            name:  name,
                        }
                    ],
                }
            },
            State::Archive{name, mut archive}  => {
                let mut r = Vec::new();
                while let Some(entry) = archive.next_entry() {
                    let mut name = name.clone();
                    match &entry {
                        &Ok(ref entry) => name += &(String::from("::") + &entry.header().identifier()),
                        _ => {},
                    };

                    r.push(match State::make_object_ar(name.clone(), entry) {
                        Err(e) => State::Error{
                            error: e,
                            name:  name,
                        },
                        Ok(v) => v,
                    })
                }
                r
            },
            State::Elf{name, mut elf, read, symbols, ..} => {
                if let Err(e) = elf.load_all(&mut *read.borrow_mut()) {
                    return vec![State::Error{
                        name:   name,
                        error:  e,
                    }]
                }

                let mut relocs : HashMap<usize, Vec<relocation::Relocation>> = HashMap::new();
                for (i, sec) in elf.sections.iter_mut().enumerate() {
                    if sec.header.shtype == types::SectionType::RELA {
                        relocs.insert(sec.header.info as usize,
                                    std::mem::replace(sec, section::Section::default())
                                    .content.into_relocations().unwrap()
                                    );
                    }
                }

                let mut sections = Vec::new();
                for (i, sec) in elf.sections.into_iter().enumerate() {
                    match sec.header.shtype {
                        types::SectionType::NULL | types::SectionType::STRTAB => {},
                        _ => {
                            sections.push((i, sec, relocs.remove(&i).unwrap_or_else(||Vec::new())));
                        },
                    }
                }

                vec![State::Object{
                    name:       name,
                    symbols:    symbols,
                    header:     elf.header,
                    sections:   sections,
                }]
            },
            any => vec![any],
        }
    }

    fn make_object(name: String, io: RefCell<Box<ReadSeekSend>>) -> Result<State, Error> {

        let mut elf = Elf::from_reader(&mut *io.borrow_mut())?;

        let mut num_symbols = 0;

        for i in 0..elf.sections.len() {
            match elf.sections[i].header.shtype {
                types::SectionType::SYMTAB |
                    types::SectionType::DYNSYM => {
                        elf.load(i, &mut *io.borrow_mut())?;
                        num_symbols += elf.sections[i].content.as_symbols().unwrap().len();
                    },
                _ => {}
            }
        }

        if num_symbols == 0 {
            return Err(Error::NoSymbolsInObject);
        }

        let mut bloom = BloomFilter::new(num_symbols);

        let mut symbols = None;
        for i in 0..elf.sections.len() {
            if (elf.sections[i].header.shtype == types::SectionType::SYMTAB &&
               elf.header.etype == types::ElfType::REL) ||
               (elf.sections[i].header.shtype == types::SectionType::DYNSYM &&
               elf.header.etype == types::ElfType::DYN) {

               if let Some(_) = symbols {
                   return Err(Error::MultipleSymbolSections);
               }

               let syms = std::mem::replace(&mut elf.sections[i], section::Section::default())
                   .content.into_symbols().unwrap();

               for sym in syms.iter() {
                   match sym.bind {
                       types::SymbolBind::GLOBAL | types::SymbolBind::WEAK => {
                           match sym.shndx {
                               symbol::SymbolSectionIndex::Undefined |
                                   symbol::SymbolSectionIndex::Absolute  => {},
                               _ => {
                                   bloom.insert(&BloomFilter::hash(&sym.name));
                               },
                           }
                       },
                       _ => {},

                   }
               }
               symbols = Some(syms);
            }
        }

        if let None = symbols {
            return Err(Error::MissingSymtabSection);
        }

        Ok(State::Elf{
            name:    name,
            elf:     elf,
            read:    io,
            bloom:   bloom,
            symbols: symbols.unwrap(),
        })
    }

    fn make_object_ar(name: String, entry: std::io::Result<ar::Entry<File>>) -> Result<State, Error> {
        let mut entry = entry?;
        let mut buf = Vec::with_capacity(entry.header().size() as usize);
        entry.read_to_end(&mut buf)?;
        let io = Cursor::new(buf);

        State::make_object(name,
        RefCell::new(Box::new(io)))
    }

}



pub struct BloomFilter {
    bits: BitVec,
}

impl BloomFilter {

    fn new(num_items: usize) -> BloomFilter {
        BloomFilter {
            bits: BitVec::from_elem(Self::needed_bits(0.001, num_items as u32), false),
        }
    }

    fn hash(n:&[u8]) -> [u64;2] {
        let mut a1 = FnvHasher::with_key(0xcbf29ce484222325);
        let mut a2 = FnvHasher::with_key(0x84222325b444f000);
        n.hash(&mut a1);
        n.hash(&mut a2);
        [a1.finish(),a2.finish()]
    }

    fn needed_bits(false_pos_rate: f32, num_items: u32) -> usize {
        let ln22 = core::f32::consts::LN_2 * core::f32::consts::LN_2;
        (num_items as f32 * ((1.0/false_pos_rate).ln() / ln22)).round() as usize
    }

    fn insert(&mut self, nh: &[u64;2]) {
        let len = self.bits.len();
        self.bits.set(nh[0] as usize % len, true);
        self.bits.set(nh[1] as usize % len, true);
    }

    fn contains(&self, nh: &[u64;2]) -> bool {
        self.bits[nh[0] as usize % self.bits.len()] &&
        self.bits[nh[1] as usize % self.bits.len()]
    }
}

