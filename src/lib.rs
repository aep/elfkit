extern crate num;
extern crate byteorder;
#[macro_use] extern crate enum_primitive;
#[macro_use] extern crate bitflags;
#[macro_use] mod read_macros;
pub mod relocation;
pub mod types;
pub mod symbol;

use std::io::{Read, Seek, SeekFrom};
use num::FromPrimitive;

#[derive(Debug)]
pub enum Error {
    Io(::std::io::Error),
    InvalidMagic,
    InvalidFormat,
    UnsupportedFormat,
}

impl From<::std::io::Error> for Error {
    fn from(error: ::std::io::Error) -> Self {
        Error::Io(error)
    }
}

#[derive(Default, Debug)]
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


impl Header {
    pub fn from_reader<R>(io:&mut  R) -> Result<Header, Error> where R: Read{
        let mut r = Header::default();
        let mut b = [0;16];
        io.read(&mut b)?;
        r.ident_magic.clone_from_slice(&b[0..4]);

        if r.ident_magic != [0x7F,0x45,0x4c,0x46] {
            return Err(Error::InvalidMagic);
        }

        r.ident_class  = match types::Class::from_u8(b[4]) {
            Some(v) => v,
            None => return Err(Error::InvalidFormat),
        };

        r.ident_endianness = match types::Endianness::from_u8(b[5]) {
            Some(v) => v,
            None => return Err(Error::InvalidFormat),
        };

        r.ident_version    = b[6];
        if r.ident_version != 1 {
            return Err(Error::UnsupportedFormat);
        }

        r.ident_abi = match types::Abi::from_u8(b[7]) {
            Some(v) => v,
            None => return Err(Error::InvalidFormat),
        };

        r.etype     = match types::ElfType::from_u16(elf_read_u16!(r, io)?) {
            Some(v) => v,
            None => return Err(Error::InvalidFormat),
        };
        r.machine   = match types::Machine::from_u16(elf_read_u16!(r, io)?) {
            Some(v) => v,
            None => return Err(Error::InvalidFormat),
        };
        r.version   = elf_read_u32!(r, io)?;

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
}

#[derive(Default, Debug)]
pub struct Section {
    pub name:       String,
    pub shtype:     types::SectionType,
    pub flags:      types::SectionFlags,
    pub addr:       u64,
    pub offset:     u64,
    pub size:       u64,
    pub link:       u32,
    pub info:       u32,
    pub addralign:  u64,
    pub entsize:    u64,

    pub _name:      u32,
}

impl Section {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<Section, Error> where R: Read {
        let mut r = Section::default();
        let mut b = vec![0; eh.shentsize as usize];
        io.read(&mut b)?;
        let mut br = &b[..];
        r._name     = elf_read_u32!(eh, br)?;
        r.shtype    = match types::SectionType::from_u32(elf_read_u32!(eh, br)?) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
        };
        r.flags     = match types::SectionFlags::from_bits(elf_read_uclass!(eh, br)?) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
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
}


#[derive(Default, Debug)]
pub struct Segment {
    pub phtype: types::SegmentType,
    pub flags:  u32,
    pub offset: u64,
    pub vaddr:  u64,
    pub paddr:  u64,
    pub filesz: u64,
    pub memsz:  u64,
    pub align:  u64,
}

impl Segment {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<Segment, Error> where R: Read {
        let mut r = Segment::default();
        let mut b = vec![0; eh.phentsize as usize];
        io.read(&mut b)?;
        let mut br = &b[..];

        r.phtype = match types::SegmentType::from_u32(elf_read_u32!(eh, br)?) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
        };

        match eh.ident_class  {
            types::Class::Class64 => {
                r.flags     = elf_read_u32!(eh, br)?;
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
                r.align     = elf_read_u32!(eh, br)? as u64;
            },
        };
        Ok(r)
    }
}

#[derive(Default, Debug)]
pub struct Elf {
    pub header:   Header,
    pub sections: Vec<Section>,
    pub segments: Vec<Segment>,

    pub strtab:   String,
}

impl Elf {
    pub fn from_reader<R>(io: &mut R) -> Result<Elf, Error> where R: Read + Seek {
        let mut r = Elf::default();
        r.header = Header::from_reader(io)?;

        // parse program
        io.seek(SeekFrom::Start(r.header.phoff))?;
        for _ in 0..r.header.phnum {
            let segment = Segment::from_reader(io, &r.header)?;
            r.segments.push(segment);
        }

        // parse sections
        io.seek(SeekFrom::Start(r.header.shoff))?;
        for _ in 0..r.header.shnum {
            let section = Section::from_reader(io, &r.header)?;
            r.sections.push(section);
        }

        // resolve names
        let shstrtab = r.sections[r.header.shstrndx as usize].offset;
        for sec in &mut r.sections {
            let mut name = Vec::new();
            io.seek(SeekFrom::Start(shstrtab + sec._name as u64))?;
            for byte in io.bytes() {
                let byte = byte.unwrap_or(0);
                if byte == 0 {
                    break
                }
                name.push(byte);
            }
            sec.name = String::from_utf8_lossy(&name).into_owned();
        }

        for sec in &r.sections {
            if sec.name == ".strtab" {
                io.seek(SeekFrom::Start(sec.offset))?;
                let mut bb = vec![0; sec.size as usize];
                io.read(&mut bb)?;
                r.strtab = String::from_utf8_lossy(&bb).into_owned();
            }
        }

        Ok(r)
    }
}




