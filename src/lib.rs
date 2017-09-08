extern crate byteorder;
#[macro_use] extern crate enum_primitive_derive;
#[macro_use] extern crate bitflags;
#[macro_use] mod read_macros;
extern crate num_traits;
pub mod relocation;
pub mod types;
pub mod symbol;
pub mod dynamic;

use std::io::{Read, Write, Seek, SeekFrom};
use std::io::BufWriter;
use num_traits::{FromPrimitive, ToPrimitive};

pub use relocation::Relocation;
pub use symbol::Symbol;
pub use dynamic::Dynamic;

#[derive(Debug)]
pub enum Error {
    Io(::std::io::Error),
    InvalidMagic,
    InvalidIdentClass(u8),
    InvalidEndianness(u8),
    InvalidIdentVersion(u8),
    InvalidVersion(u32),
    InvalidAbi(u8),
    InvalidElfType(u16),
    InvalidMachineType(u16),
    InvalidSectionFlags(u64),
    InvalidSegmentType(u32),
    InvalidSectionType(u32),
    UnsupportedMachineTypeForRelocation,
    InvalidSymbolType(u8),
    InvalidSymbolBind(u8),
    InvalidSymbolVis(u8),
    InvalidDynamicType(u64),
    MissingShstrtabSection,
}

impl From<::std::io::Error> for Error {
    fn from(error: ::std::io::Error) -> Self {
        Error::Io(error)
    }
}

#[derive(Debug)]
pub struct Header {
    pub ident_magic:      [u8;4],
    pub ident_class:      types::Class,
    pub ident_endianness: types::Endianness,
    pub ident_version:    u8, // 1
    pub ident_abi:        types::Abi,

    pub etype:      types::ElfType,
    pub machine:    types::Machine,
    pub version:    u32, //1
    pub entry:      u64, //program counter starts here
    pub phoff:      u64, //offset of program header table
    pub shoff:      u64, //offset of section header table
    pub flags:      u32, //no idea
    pub ehsize:     u16, //size of this header (who cares?)
    pub phentsize:  u16, //the size of a program header table entry
    pub phnum:      u16, //the number of entries in the program header table
    pub shentsize:  u16, //the size of a section header table entry
    pub shnum:      u16, //the number of entries in the section header table
    pub shstrndx:   u16, //where to find section names
}

impl Default for Header {
    fn default() -> Self {Header{
        ident_magic:      [0x7F,0x45,0x4c, 0x46],
        ident_class:      types::Class::Class64,
        ident_endianness: types::Endianness::LittleEndian,
        ident_version:    1,
        ident_abi:        types::Abi::SYSV,
        etype:      types::ElfType::default(),
        machine:    types::Machine::default(),
        version:    1,
        entry:      0,
        phoff:      0,
        shoff:      0,
        flags:      0,
        ehsize:     0,
        phentsize:  0,
        phnum:      0,
        shentsize:  0,
        shnum:      0,
        shstrndx:   0,
    }}
}


impl Header {
    pub fn from_reader<R>(io:&mut  R) -> Result<Header, Error> where R: Read{
        let mut r = Header::default();
        let mut b = [0;16];
        io.read_exact(&mut b)?;
        r.ident_magic.clone_from_slice(&b[0..4]);

        if r.ident_magic != [0x7F,0x45,0x4c,0x46] {
            return Err(Error::InvalidMagic);
        }

        r.ident_class  = match types::Class::from_u8(b[4]) {
            Some(v) => v,
            None => return Err(Error::InvalidIdentClass(b[4])),
        };

        r.ident_endianness = match types::Endianness::from_u8(b[5]) {
            Some(v) => v,
            None => return Err(Error::InvalidEndianness(b[5])),
        };

        r.ident_version = b[6];
        if r.ident_version != 1 {
            return Err(Error::InvalidIdentVersion(b[6]));
        }

        r.ident_abi = match types::Abi::from_u8(b[7]) {
            Some(v) => v,
            None => return Err(Error::InvalidAbi(b[7])),
        };

        let reb = elf_read_u16!(r, io)?;
        r.etype     = match types::ElfType::from_u16(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidElfType(reb)),
        };

        let reb = elf_read_u16!(r, io)?;
        r.machine   = match types::Machine::from_u16(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidMachineType(reb)),
        };

        r.version   = elf_read_u32!(r, io)?;
        if r.version != 1 {
            return Err(Error::InvalidVersion(r.version));
        }

        r.entry     = elf_read_uclass!(r, io)?;
        r.phoff     = elf_read_uclass!(r, io)?;
        r.shoff     = elf_read_uclass!(r, io)?;
        r.flags     = elf_read_u32!(r, io)?;
        r.ehsize    = elf_read_u16!(r, io)?;
        r.phentsize = elf_read_u16!(r, io)?;
        r.phnum     = elf_read_u16!(r, io)?;
        r.shentsize = elf_read_u16!(r, io)?;
        r.shnum     = elf_read_u16!(r, io)?;
        r.shstrndx  = elf_read_u16!(r, io)?;

        Ok(r)
    }

    pub fn to_writer<R>(&self, io:&mut  R) -> Result<(), Error> where R: Write {
        let mut w = BufWriter::new(io);
        w.write(&self.ident_magic)?;
        w.write(&[self.ident_class.to_u8().unwrap()])?;
        w.write(&[self.ident_endianness.to_u8().unwrap()])?;
        w.write(&[self.ident_version.to_u8().unwrap()])?;
        w.write(&[self.ident_abi.to_u8().unwrap()])?;
        w.write(&[0;8])?;

        elf_write_u16!(self, w, self.etype.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.machine.to_u16().unwrap())?;
        elf_write_u32!(self, w, self.version.to_u32().unwrap())?;
        elf_write_uclass!(self, w, self.entry.to_u64().unwrap())?;
        elf_write_uclass!(self, w, self.phoff.to_u64().unwrap())?;
        elf_write_uclass!(self, w, self.shoff.to_u64().unwrap())?;
        elf_write_u32!(self, w, self.flags.to_u32().unwrap())?;
        elf_write_u16!(self, w, self.ehsize.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.phentsize.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.phnum.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.shentsize.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.shnum.to_u16().unwrap())?;
        elf_write_u16!(self, w, self.shstrndx.to_u16().unwrap())?;

        Ok(())
    }

    pub fn size(&self) ->  usize {
        16 + 2 + 2 + 4 +
            match self.ident_class {
                types::Class::Class32 => 4 + 4 + 4,
                types::Class::Class64 => 8 + 8 + 8,
            } + 4 + 2 + 2 +2 +2 +2 +2
    }
}

#[derive(Default, Debug, Clone)]
pub struct SectionHeader {
    pub name:       u32,
    pub shtype:     types::SectionType,
    pub flags:      types::SectionFlags,
    pub addr:       u64,
    pub offset:     u64,
    pub size:       u64,
    pub link:       u32,
    pub info:       u32,
    pub addralign:  u64,
    pub entsize:    u64,
}

impl SectionHeader {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<SectionHeader, Error> where R: Read {
        let mut r = SectionHeader::default();
        let mut b = vec![0; eh.shentsize as usize];
        io.read(&mut b)?;
        let mut br = &b[..];
        r.name     = elf_read_u32!(eh, br)?;

        let reb  = elf_read_u32!(eh, br)?;
        r.shtype = match types::SectionType::from_u32(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSectionType(reb)),
        };

        let reb = elf_read_uclass!(eh, br)?;
        r.flags = match types::SectionFlags::from_bits(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSectionFlags(reb)),
        };
        r.addr      = elf_read_uclass!(eh, br)?;
        r.offset    = elf_read_uclass!(eh, br)?;
        r.size      = elf_read_uclass!(eh, br)?;
        r.link      = elf_read_u32!(eh, br)?;
        r.info      = elf_read_u32!(eh, br)?;
        r.addralign = elf_read_uclass!(eh, br)?;
        r.entsize   = elf_read_uclass!(eh, br)?;
        Ok(r)
    }

    pub fn to_writer<R>(&self, eh: &Header, io: &mut  R) -> Result<(), Error> where R: Write {
        let mut w = BufWriter::new(io);
        elf_write_u32!(eh, w, self.name)?;
        elf_write_u32!(eh, w, self.shtype.to_u32().unwrap())?;
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


#[derive(Default, Debug)]
pub struct SegmentHeader {
    pub phtype: types::SegmentType,
    pub flags:  types::SegmentFlags,
    pub offset: u64,
    pub vaddr:  u64,
    pub paddr:  u64,
    pub filesz: u64,
    pub memsz:  u64,
    pub align:  u64,
}

impl SegmentHeader {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<SegmentHeader, Error> where R: Read {
        let mut r = SegmentHeader::default();
        let mut b = vec![0; eh.phentsize as usize];
        io.read(&mut b)?;
        let mut br = &b[..];

        let reb = elf_read_u32!(eh, br)?;
        r.phtype = match types::SegmentType::from_u32(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSegmentType(reb)),
        };

        match eh.ident_class  {
            types::Class::Class64 => {
                r.flags     = types::SegmentFlags::from_bits_truncate(elf_read_u32!(eh, br)? as u64);
                r.offset    = elf_read_u64!(eh, br)?;
                r.vaddr     = elf_read_u64!(eh, br)?;
                r.paddr     = elf_read_u64!(eh, br)?;
                r.filesz    = elf_read_u64!(eh, br)?;
                r.memsz     = elf_read_u64!(eh, br)?;
                r.align     = elf_read_u64!(eh, br)?;
            },
            types::Class::Class32 => {
                r.offset    = elf_read_u32!(eh, br)? as u64;
                r.vaddr     = elf_read_u32!(eh, br)? as u64;
                r.paddr     = elf_read_u32!(eh, br)? as u64;
                r.filesz    = elf_read_u32!(eh, br)? as u64;
                r.memsz     = elf_read_u32!(eh, br)? as u64;
                r.flags     = types::SegmentFlags::from_bits_truncate(elf_read_u32!(eh, br)? as u64);
                r.align     = elf_read_u32!(eh, br)? as u64;
            },
        };
        Ok(r)
    }
    pub fn to_writer<R>(&self, eh: &Header, io: &mut  R) -> Result<(), Error> where R: Write {
        let mut w = BufWriter::new(io);
        elf_write_u32!(eh, w, self.phtype.to_u32().unwrap())?;
        match eh.ident_class  {
            types::Class::Class64 => {
                elf_write_u32!(eh, w, self.flags.bits() as u32)?;
                elf_write_u64!(eh, w, self.offset)?;
                elf_write_u64!(eh, w, self.vaddr)?;
                elf_write_u64!(eh, w, self.paddr)?;
                elf_write_u64!(eh, w, self.filesz)?;
                elf_write_u64!(eh, w, self.memsz)?;
                elf_write_u64!(eh, w, self.align)?;
            },
            types::Class::Class32 => {
                elf_write_u32!(eh, w, self.offset as u32)?;
                elf_write_u32!(eh, w, self.vaddr  as u32)?;
                elf_write_u32!(eh, w, self.paddr  as u32)?;
                elf_write_u32!(eh, w, self.filesz as u32)?;
                elf_write_u32!(eh, w, self.memsz  as u32)?;
                elf_write_u32!(eh, w, self.flags.bits() as u32)?;
                elf_write_u32!(eh, w, self.align  as u32)?;
            },
        };
        Ok(())
    }
}


#[derive(Clone)]
pub enum SectionContent {
    None,
    Strings(String),
    Relocations(Vec<Relocation>),
    Symbols(Vec<Symbol>),
    Dynamic(Vec<Dynamic>),
}

impl Default for SectionContent{
    fn default() -> Self {SectionContent::None}
}

#[derive(Default, Clone)]
pub struct Section {
    pub header:  SectionHeader,
    pub name:    String,
    pub content: SectionContent,
}



#[derive(Default)]
pub struct Elf {
    pub header:   Header,
    pub segments: Vec<SegmentHeader>,
    pub sections: Vec<Section>,
}

impl Elf {

    pub fn from_reader<R>(io: &mut R) -> Result<Elf, Error> where R: Read + Seek {
        let mut r = Elf::default();
        r.header = Header::from_reader(io)?;

        // parse segments
        io.seek(SeekFrom::Start(r.header.phoff))?;
        for _ in 0..r.header.phnum {
            let segment = SegmentHeader::from_reader(io, &r.header)?;
            r.segments.push(segment);
        }

        // parse section headers
        io.seek(SeekFrom::Start(r.header.shoff))?;
        for _ in 0..r.header.shnum {
            let sh = SectionHeader::from_reader(io, &r.header)?;
            r.sections.push(Section{
                header: sh,
                name: String::default(),
                content: SectionContent::None,
            });
        }

        //resolve all string tables
        for ref mut sec in &mut r.sections {
            if sec.header.shtype == types::SectionType::STRTAB {
                io.seek(SeekFrom::Start(sec.header.offset))?;
                let mut bb = vec![0; sec.header.size as usize];
                io.read(&mut bb)?;
                sec.content = SectionContent::Strings(String::from_utf8_lossy(&bb).into_owned());
            }
        }

        // resolve section names
        let shstrtab = match r.sections.get(r.header.shstrndx as usize) {
            None => return Err(Error::MissingShstrtabSection),
            Some(sec) => {
                match sec.content {
                    SectionContent::Strings(ref s) => s,
                    _ => return Err(Error::MissingShstrtabSection),
                }
            }
        }.clone();

        for ref mut sec in &mut r.sections {
            sec.name = shstrtab[sec.header.name as usize ..].split('\0').next().unwrap_or("").to_owned();
        }

        r.load_all(io)?;
        Ok(r)
    }

    pub fn get_section_by_name(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|&s|s.name == name)
    }

    pub fn load_all<R>(&mut self, io: &mut R) -> Result<(), Error> where R: Read + Seek {

        let mut sections = self.sections.to_vec();
        for ref mut sec in &mut sections {
            io.seek(SeekFrom::Start(sec.header.offset))?;
            let io = &mut io.take(sec.header.size);

            match sec.header.shtype {
                types::SectionType::RELA => {
                    let relocs = Relocation::from_reader(io, &self.header)?;
                    sec.content = SectionContent::Relocations(relocs);
                },
                types::SectionType::SYMTAB => {
                    let strtab  = self.get_section_by_name(".strtab").map(|s| {
                        match s.content {
                            SectionContent::Strings(ref s) => s.as_ref(),
                            _ => unreachable!()
                        }
                    });
                    let symbols = Symbol::from_reader(io, strtab, &self.header).unwrap();
                    sec.content = SectionContent::Symbols(symbols);
                },
                types::SectionType::DYNSYM => {
                    let strtab  = self.get_section_by_name(".dynstr").map(|s| {
                        match s.content {
                            SectionContent::Strings(ref s) => s.as_ref(),
                            _ => unreachable!()
                        }
                    });
                    let symbols = Symbol::from_reader(io, strtab, &self.header).unwrap();
                    sec.content = SectionContent::Symbols(symbols);
                },
                types::SectionType::DYNAMIC => {
                    let strtab  = self.get_section_by_name(".strtab").map(|s| {
                        match s.content {
                            SectionContent::Strings(ref s) => s.as_ref(),
                            _ => unreachable!()
                        }
                    });
                    let dynamics = Dynamic::from_reader(io, strtab, &self.header).unwrap();
                    sec.content  = SectionContent::Dynamic(dynamics);
                }
                _ => {},
            }
        }
        self.sections = sections;

        Ok(())
    }

    pub fn write_start<R>(&mut self, io: &mut R) -> Result<(), Error> where R: Write + Seek {
        io.seek(SeekFrom::Start(0))?;
        let off = self.header.size();
        io.write(&vec![0;off])?;
        Ok(())
    }

    pub fn write_end<R>(&mut self, io: &mut R) -> Result<(), Error> where R: Write + Seek {
        let mut off = io.seek(SeekFrom::Current(0))? as usize;

        //segments
        if self.segments.len() > 0 {
            self.header.phoff = off as u64;
            for seg in &self.segments {
                seg.to_writer(&self.header, io)?;
            }
            let at = io.seek(SeekFrom::Current(0))? as usize;
            self.header.phnum       = self.segments.len() as u16;
            self.header.phentsize   = ((at - off)/ self.segments.len()) as u16;
            off = at;
        }


        //always prepend a null section. i don't know yet why, but this is what everyone does.
        self.sections.insert(0, Section::default());

        /*
        //shstrtab
        let mut shstrtab = Section::default();
        shstrtab.name = String::from(".shstrtab");
        shstrtab.offset = off as u64;
        self.header.shstrndx = self.sections.len() as u16;
        self.sections.push(shstrtab);


        let mut inoff = 0;
        for sec in &mut self.sections {
            io.write(&sec.name.as_ref())?;
            io.write(&[0;1])?;
            sec._name = inoff as u32;
            inoff += sec.name.len() + 1;
        }
        let at = io.seek(SeekFrom::Current(0))? as usize;
        self.sections[self.header.shstrndx as usize].size = (at - off) as u64;
        off = at;


        //section headers
        self.header.shoff = off as u64;
        for sec in &self.sections {
            sec.to_writer(&self.header, io)?;
        }
        let at = io.seek(SeekFrom::Current(0))? as usize;
        self.header.shnum       = self.sections.len() as u16;
        self.header.shentsize   = ((at - off)/ self.sections.len()) as u16;
        off = at;

        io.seek(SeekFrom::Start(0))?;
        self.header.to_writer(io)?;
        */
        Ok(())
    }
}




