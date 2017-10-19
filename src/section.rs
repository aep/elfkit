use error::Error;
use header::Header;
use relocation::Relocation;
use dynamic::Dynamic;
use symbol::Symbol;
use strtab::Strtab;
use types;

use std::io::{Read, Seek, SeekFrom, Write};
use std::io::BufWriter;

#[derive(Default, Debug, Clone)]
pub struct SectionHeader {
    pub name: u32,
    pub shtype: types::SectionType,
    pub flags: types::SectionFlags,
    pub addr: u64,
    pub offset: u64,
    pub size: u64,
    pub link: u32,
    pub info: u32,
    pub addralign: u64,
    pub entsize: u64,
}

impl SectionHeader {
    pub fn entsize(eh: &Header) -> usize {
        4 + 4 + match eh.ident_class {
            types::Class::Class64 => 6 * 8,
            types::Class::Class32 => 6 * 4,
        } + 4 + 4
    }

    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<SectionHeader, Error>
    where
        R: Read,
    {
        let mut r = SectionHeader::default();
        let mut b = vec![0; eh.shentsize as usize];
        io.read_exact(&mut b)?;
        let mut br = &b[..];
        r.name = elf_read_u32!(eh, br)?;

        let reb = elf_read_u32!(eh, br)?;
        r.shtype = types::SectionType(reb);

        let reb = elf_read_uclass!(eh, br)?;
        r.flags = match types::SectionFlags::from_bits(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSectionFlags(reb)),
        };
        r.addr = elf_read_uclass!(eh, br)?;
        r.offset = elf_read_uclass!(eh, br)?;
        r.size = elf_read_uclass!(eh, br)?;
        r.link = elf_read_u32!(eh, br)?;
        r.info = elf_read_u32!(eh, br)?;
        r.addralign = elf_read_uclass!(eh, br)?;
        r.entsize = elf_read_uclass!(eh, br)?;
        Ok(r)
    }

    pub fn to_writer<R>(&self, eh: &Header, io: &mut R) -> Result<(), Error>
    where
        R: Write,
    {
        let mut w = BufWriter::new(io);
        elf_write_u32!(eh, w, self.name)?;
        elf_write_u32!(eh, w, self.shtype.to_u32())?;
        elf_write_uclass!(eh, w, self.flags.bits())?;

        elf_write_uclass!(eh, w, self.addr)?;
        elf_write_uclass!(eh, w, self.offset)?;
        elf_write_uclass!(eh, w, self.size)?;
        elf_write_u32!(eh, w, self.link)?;
        elf_write_u32!(eh, w, self.info)?;
        elf_write_uclass!(eh, w, self.addralign)?;
        elf_write_uclass!(eh, w, self.entsize)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum SectionContent {
    None,
    Raw(Vec<u8>),
    Relocations(Vec<Relocation>),
    Symbols(Vec<Symbol>),
    Dynamic(Vec<Dynamic>),
    Strtab(Strtab),
}

impl Default for SectionContent {
    fn default() -> Self {
        SectionContent::None
    }
}
impl SectionContent {
    pub fn as_dynamic_mut(&mut self) -> Option<&mut Vec<Dynamic>> {
        match self {
            &mut SectionContent::Dynamic(ref mut v) => Some(v),
            _ => None,
        }
    }
    pub fn as_strtab_mut(&mut self) -> Option<&mut Strtab> {
        match self {
            &mut SectionContent::Strtab(ref mut v) => Some(v),
            _ => None,
        }
    }
    pub fn as_symbols(&self) -> Option<&Vec<Symbol>> {
        match self {
            &SectionContent::Symbols(ref v) => Some(v),
            _ => None,
        }
    }
    pub fn into_symbols(self) -> Option<Vec<Symbol>> {
        match self {
            SectionContent::Symbols(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_relocations(&self) -> Option<&Vec<Relocation>> {
        match self {
            &SectionContent::Relocations(ref v) => Some(v),
            _ => None,
        }
    }
    pub fn into_relocations(self) -> Option<Vec<Relocation>> {
        match self {
            SectionContent::Relocations(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_raw_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            &mut SectionContent::Raw(ref mut v) => Some(v),
            _ => None,
        }
    }
    pub fn into_raw(self) -> Option<Vec<u8>> {
        match self {
            SectionContent::Raw(v) => Some(v),
            _ => None,
        }
    }
    pub fn size(&self, eh: &Header) -> usize {
        match self {
            &SectionContent::None => 0,
            &SectionContent::Raw(ref v) => v.len(),
            &SectionContent::Dynamic(ref v) => v.len() * Dynamic::entsize(eh),
            &SectionContent::Strtab(ref v) => v.len(eh),
            &SectionContent::Symbols(ref v) => v.len() * Symbol::entsize(eh),
            &SectionContent::Relocations(ref v) => v.len() * Relocation::entsize(eh),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Section {
    pub header: SectionHeader,
    pub name: String,
    pub content: SectionContent,
}


impl Section {
    pub fn size(&self, eh: &Header) -> usize {
        self.content.size(eh)
    }
    pub fn new(
        name: String,
        shtype: types::SectionType,
        flags: types::SectionFlags,
        content: SectionContent,
        link: u32,
        info: u32,
    ) -> Section {
        Section {
            name: name,
            header: SectionHeader {
                name: 0,
                shtype: shtype,
                flags: flags,
                addr: 0,
                offset: 0,
                size: 0,
                link: link,
                info: info,
                addralign: 0,
                entsize: 0,
            },
            content: content,
        }
    }

    pub fn sync(
        &mut self,
        eh: &Header,
        mut linked: Option<&mut SectionContent>,
    ) -> Result<(), Error> {
        match self.content {
            SectionContent::Relocations(ref vv) => {
                self.header.entsize = Relocation::entsize(eh) as u64;
            }
            SectionContent::Symbols(ref vv) => {
                for (i, sym) in vv.iter().enumerate() {
                    if sym.bind == types::SymbolBind::GLOBAL {
                        self.header.info = i as u32;
                        break;
                    }
                }
                for v in vv {
                    v.sync(linked.as_mut().map(|r| &mut **r), eh)?;
                }
                self.header.entsize = Symbol::entsize(eh) as u64;
            }
            SectionContent::Dynamic(ref vv) => {
                for v in vv {
                    v.sync(linked.as_mut().map(|r| &mut **r), eh)?;
                }
                self.header.entsize = Dynamic::entsize(eh) as u64;
            }
            SectionContent::Strtab(ref v) => {
                self.header.entsize = Strtab::entsize(eh) as u64;
            }
            SectionContent::None | SectionContent::Raw(_) => {}
        }
        if self.header.shtype != types::SectionType::NOBITS {
            self.header.size = self.size(eh) as u64;
        }
        Ok(())
    }
}
