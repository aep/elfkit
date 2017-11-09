use std::io::{Read, Write};
use {types, Error, Header, SectionContent};
use utils::hextab;
use num_traits::{FromPrimitive, ToPrimitive};
use strtab::Strtab;
use section::{Section, SectionHeader};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SymbolSectionIndex {
    Section(u16), // 1-6551
    Undefined,    // 0
    Absolute,     // 65521,
    Common,       // 6552,
}
impl Default for SymbolSectionIndex {
    fn default() -> SymbolSectionIndex {
        SymbolSectionIndex::Undefined
    }
}

#[derive(Default, Clone)]
pub struct Symbol {
    pub shndx:  SymbolSectionIndex,
    pub value:  u64,
    pub size:   u64,

    pub name:   Vec<u8>,
    pub stype:  types::SymbolType,
    pub bind:   types::SymbolBind,
    pub vis:    types::SymbolVis,

    pub _name:  u32,
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "  {} {:>5.5} {:<7.7} {:<6.6} {:<8.8} {:<3.3} {} ",
               hextab(16, self.value),
               self.size,
               format!("{:?}", self.stype),
               format!("{:?}", self.bind),
               format!("{:?}", self.vis),
               match self.shndx {
                   SymbolSectionIndex::Undefined => String::from("UND"),
                   SymbolSectionIndex::Absolute => String::from("ABS"),
                   SymbolSectionIndex::Common => String::from("COM"),
                   SymbolSectionIndex::Section(i) => format!("{}", i),
               },
               String::from_utf8_lossy(&self.name)
              )
    }
}

impl Symbol {
    fn from_val(
        tab:    Option<&Strtab>,
        _name:  u32,
        info:   u8,
        other:  u8,
        shndx:  u16,
        value:  u64,
        size:   u64,
    ) -> Result<Symbol, Error> {
        let name = match tab {
            Some(tab) => tab.get(_name as usize),
            None => Vec::default(),
        };

        let shndx = match shndx {
            0 => SymbolSectionIndex::Undefined,
            65521 => SymbolSectionIndex::Absolute,
            65522 => SymbolSectionIndex::Common,
            _ if shndx > 0 && shndx < 6552 => SymbolSectionIndex::Section(shndx),
            _ => return Err(Error::InvalidSymbolShndx(String::from_utf8_lossy(&name).into_owned(), shndx)),
        };

        let reb = info & 0xf;
        let stype = match types::SymbolType::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolType(reb)),
        };

        let reb = info >> 4;
        let bind = match types::SymbolBind::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolBind(reb)),
        };

        let reb = other & 0x3;
        let vis = match types::SymbolVis::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolVis(reb)),
        };

        Ok(Symbol {
            shndx: shndx,
            value: value,
            size: size,

            name: name,
            stype: stype,
            bind: bind,
            vis: vis,

            _name: _name,
        })
    }

    pub fn entsize(eh: &Header) -> usize {
        match eh.ident_class {
            types::Class::Class64 => 24,
            types::Class::Class32 => 16,
        }
    }

    pub fn from_reader<R>(
        mut io: R,
        linked: Option<&SectionContent>,
        eh: &Header,
    ) -> Result<SectionContent, Error>
    where
        R: Read,
    {
        let tab = match linked {
            None => None,
            Some(&SectionContent::Strtab(ref s)) => Some(s),
            any => return Err(Error::LinkedSectionIsNotStrtab{
                during: "reading symbols",
                link: any.map(|v|v.clone()),
            }),
        };

        let mut r = Vec::new();
        let mut b = vec![0; Self::entsize(eh)];
        while io.read(&mut b)? > 0 {
            let mut br = &b[..];
            elf_dispatch_endianness!(eh => {
                let _name = read_u32(&mut br)?;
                r.push(match eh.ident_class {
                    types::Class::Class64 => {
                        let info = b[4];
                        let other = b[5];
                        br = &b[6..];
                        let shndx = read_u16(&mut br)?;
                        let value = read_u64(&mut br)?;
                        let size  = read_u64(&mut br)?;

                        Symbol::from_val(tab, _name, info, other, shndx, value, size)?
                    }
                    types::Class::Class32 => {
                        let value = read_u32(&mut br)?;
                        let size  = read_u32(&mut br)?;
                        let info  = b[12];
                        let other = b[13];
                        br = &b[14..];
                        let shndx = read_u16(&mut br)?;

                        Symbol::from_val(tab, _name, info, other, shndx, value as u64, size as u64)?
                    }
                })
            })
        }

        Ok(SectionContent::Symbols(r))
    }

    pub fn to_writer<W>(
        &self,
        mut io: W,
        eh: &Header,
    ) -> Result<(usize), Error>
    where
        W: Write,
    {
        let info = (self.bind.to_u8().unwrap() << 4) + (self.stype.to_u8().unwrap() & 0xf);
        let other = self.vis.to_u8().unwrap();

        let shndx = match self.shndx {
            SymbolSectionIndex::Section(i) => i,
            SymbolSectionIndex::Undefined => 0,
            SymbolSectionIndex::Absolute => 65521,
            SymbolSectionIndex::Common => 65522,
        };

        elf_write_u32!(eh, io, self._name)?;

        Ok(match eh.ident_class {
            types::Class::Class64 => {
                io.write(&[info, other])?;
                elf_write_u16!(eh, io, shndx)?;
                elf_write_u64!(eh, io, self.value)?;
                elf_write_u64!(eh, io, self.size)?;

                2+2+8+8
            }
            types::Class::Class32 => {
                elf_write_u32!(eh, io, self.value as u32)?;
                elf_write_u32!(eh, io, self.size as u32)?;
                io.write(&[info, other])?;
                elf_write_u16!(eh, io, shndx)?;

                4+4+2+2
            }
        })
    }

    pub fn sync(&mut self, linked: Option<&mut SectionContent>, _: &Header) -> Result<(), Error> {
        match linked {
            Some(&mut SectionContent::Strtab(ref mut strtab)) => {
                self._name = strtab.insert(&self.name) as u32;
            }
            any => return Err(Error::LinkedSectionIsNotStrtab{
                during: "syncing symbols",
                link: any.map(|v|v.clone()),
            }),
        }
        Ok(())
    }
}

pub fn sysv_hash(s: &String) -> u64 {
    let mut h: u64 = 0;
    let mut g: u64;

    for byte in s.bytes() {
        h = (h << 4) + byte as u64;
        g = h & 0xf0000000;
        if g > 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    return h;
}


pub fn symhash(eh: &Header, symbols: &Vec<Symbol>, link: u32) -> Result<Section, Error> {
    assert!(symbols.len() > 0);
    //TODO i'm too lazy to do this correctly now, so we'll just emit a hashtable with nbuckets  == 1
    let mut b = Vec::new();
    {
        let io = &mut b;
        elf_write_uclass!(eh, io, 1)?; //nbuckets
        elf_write_uclass!(eh, io, symbols.len() as u64)?; //nchains

        elf_write_uclass!(eh, io, 1)?; //the bucket. pointing at symbol 1

        elf_write_uclass!(eh, io, 0)?; //symbol 0

        //the chains. every symbol just points at the next, because nbuckets == 1
        for i in 1..symbols.len() - 1 {
            elf_write_uclass!(eh, io, i as u64 + 1)?;
        }

        //except the last one
        elf_write_uclass!(eh, io, 0)?;
    }

    Ok(Section {
        name: b".hash".to_vec(),
        header: SectionHeader {
            name: 0,
            shtype: types::SectionType::HASH,
            flags: types::SectionFlags::ALLOC,
            addr: 0,
            offset: 0,
            size: b.len() as u64,
            link: link,
            info: 0,
            addralign: 0,
            entsize: 8, // or 4 for CLass32
        },
        content: SectionContent::Raw(b),
        addrlock: false,
    })
}
