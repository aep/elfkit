extern crate byteorder;
#[macro_use] extern crate enum_primitive_derive;
#[macro_use] extern crate bitflags;
#[macro_use] mod utils;
extern crate num_traits;
pub mod relocation;
pub mod types;
pub mod symbol;
pub mod dynamic;
pub mod strtab;

use std::io::{Read, Write, Seek, SeekFrom};
use std::io::BufWriter;
use num_traits::{FromPrimitive, ToPrimitive};
use std::collections::HashMap;

pub use relocation::Relocation;
pub use symbol::Symbol;
pub use strtab::Strtab;
pub use dynamic::{Dynamic, DynamicContent};

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
    InvalidHeaderFlags(u32),
    InvalidSectionFlags(u64),
    InvalidSegmentType(u32),
    InvalidSectionType(u32),
    UnsupportedMachineTypeForRelocation(types::Machine),
    InvalidSymbolType(u8),
    InvalidSymbolBind(u8),
    InvalidSymbolVis(u8),
    InvalidDynamicType(u64),
    MissingShstrtabSection,
    LinkedSectionIsNotStrtab,
    InvalidDynamicFlags1(u64),
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
    pub ident_abiversion: u8,

    pub etype:      types::ElfType,
    pub machine:    types::Machine,
    pub version:    u32, //1
    pub entry:      u64, //program counter starts here
    pub phoff:      u64, //offset of program header table
    pub shoff:      u64, //offset of section header table
    pub flags:      types::HeaderFlags,
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
        ident_abiversion: 0,
        etype:      types::ElfType::default(),
        machine:    types::Machine::default(),
        version:    1,
        entry:      0,
        phoff:      0,
        shoff:      0,
        flags:      types::HeaderFlags::default(),
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

        r.ident_abiversion = b[8];

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

        let reb = elf_read_u32!(r, io)?;
        r.flags = types::HeaderFlags::from_bits_truncate(reb);
        //r.flags = match types::HeaderFlags::from_bits(reb) {
        //    Some(v) => v,
        //    None => return Err(Error::InvalidHeaderFlags(reb)),
        //};

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
        elf_write_u32!(self, w, self.flags.bits())?;
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
        io.read_exact(&mut b)?;
        let mut br = &b[..];
        r.name     = elf_read_u32!(eh, br)?;

        let reb  = elf_read_u32!(eh, br)?;
        r.shtype = types::SectionType(reb);

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


#[derive(Default, Debug, Clone)]
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
    pub fn entsize(eh: &Header) ->  usize {
        match eh.ident_class {
            types::Class::Class64 => 4 + 4 + 6 * 8,
            types::Class::Class32 => 4 + 4 + 6 * 4,
        }
    }

    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<SegmentHeader, Error> where R: Read {
        let mut r = SegmentHeader::default();
        let mut b = vec![0; eh.phentsize as usize];
        io.read_exact(&mut b)?;
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
    Raw(Vec<u8>),
    Relocations(Vec<Relocation>),
    Symbols(Vec<Symbol>),
    Dynamic(Vec<Dynamic>),
    Strtab(Strtab),
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


impl Section {
    pub fn size(&self) -> usize {
        match self.content {
            SectionContent::None => 0,
            SectionContent::Raw(ref v) => v.len(),
            _ => panic!("tried to size a section that isn't serialized"),
        }
    }
}

pub struct Elf {
    pub header:   Header,
    pub segments: Vec<SegmentHeader>,
    pub sections: Vec<Section>,
}

impl Default for Elf {
    fn default() -> Self {
        let mut r = Elf {
            header:   Header::default(),
            segments: Vec::default(),
            sections: Vec::default(),
        };
        //always prepend a null section. i don't know yet why, but this is what everyone does.
        //TODO this is part of the linker?
        //r.sections.insert(0, Section::default());
        r
    }
}

impl Elf {

    pub fn from_reader<R>(io: &mut R) -> Result<Elf, Error> where R: Read + Seek {
        let mut r = Elf::default();
        r.header = Header::from_reader(io)?;

        // parse segments
        r.segments.clear();
        io.seek(SeekFrom::Start(r.header.phoff))?;
        for _ in 0..r.header.phnum {
            let segment = SegmentHeader::from_reader(io, &r.header)?;
            r.segments.push(segment);
        }

        // parse section headers
        r.sections.clear();
        io.seek(SeekFrom::Start(r.header.shoff))?;
        let mut section_headers = Vec::new();
        for _ in 0..r.header.shnum {
            section_headers.push(SectionHeader::from_reader(io, &r.header)?);
        }

        // read section content
        for sh in section_headers {
            r.sections.push(Section{
                name: String::default(),
                content: match sh.shtype {
                    types::SectionType::NULL | types::SectionType::NOBITS => {
                        SectionContent::None
                    },
                    _ => {
                        io.seek(SeekFrom::Start(sh.offset))?;
                        let mut bb = vec![0; sh.size as usize];
                        io.read_exact(&mut bb)?;
                        SectionContent::Raw(bb)
                    }
                },
                header: sh,
            });
        }

        // resolve section names
        let shstrtab = match r.sections.get(r.header.shstrndx as usize) {
            None => return Err(Error::MissingShstrtabSection),
            Some(sec) => {
                match sec.content {
                    SectionContent::Raw(ref s) => s,
                    _ => return Err(Error::MissingShstrtabSection),
                }
            }
        }.clone();

        for ref mut sec in &mut r.sections {
            sec.name = String::from_utf8_lossy(
                shstrtab[sec.header.name as usize ..].split(|e|*e==0).next().unwrap_or(&[0;0])
                ).into_owned();
        }

        Ok(r)
    }

    fn load(&self, raw: Vec<u8>, sh: &SectionHeader, linked: Option<&SectionContent>)
        -> Result<(SectionContent), Error> {
            Ok(match sh.shtype {
                types::SectionType::STRTAB => {
                    let io = &raw[..];
                    Strtab::from_reader(io, linked, &self.header)?
                },
                types::SectionType::RELA   => {
                    let io = &raw[..];
                    Relocation::from_reader(io, linked, &self.header)?
                },
                types::SectionType::SYMTAB | types::SectionType::DYNSYM => {
                    let io = &raw[..];
                    Symbol::from_reader(io, linked, &self.header)?
                }
                types::SectionType::DYNAMIC => {
                    let io = &raw[..];
                    Dynamic::from_reader(io, linked, &self.header)?
                }
                _ => SectionContent::Raw(raw),
            })
        }


    fn load_at(&mut self, i: usize) -> Result<(), Error>{
        let is_loaded = match self.sections[i].content {
            SectionContent::Raw(_) => false,
            _ => true,
        };

        if is_loaded {
            return Ok(())
        }

        //take out the original. this is to work around the borrow checker
        let mut sec = std::mem::replace(&mut self.sections[i], Section::default());
        {
            let linked = {
                if sec.header.link < 1 || sec.header.link as usize >= self.sections.len() {
                    None
                } else {
                    self.load_at(sec.header.link as usize);
                    Some(&self.sections[sec.header.link as usize].content)
                }
            };

            sec.content = match sec.content {
                SectionContent::Raw(raw) => {
                    self.load(raw, &sec.header, linked)?
                },
                any => any,
            };
        }

        //put it back in
        self.sections[i] = sec;

        Ok(())
    }

    pub fn load_all(&mut self) -> Result<(), Error> {
        for i in 0..self.sections.len() {
            self.load_at(i);
        }
        Ok(())
    }

}

macro_rules!  insert_stuff_with_strtab{
    ($self:ident, $sec:ident, $vv:ident, $size:expr, $rawsecs:ident, $i:ident) => {
        {
            let mut raw = Vec::new();
            {
                for v in $vv {
                    let linked = $rawsecs.get_mut(&($sec.header.link as usize)).map(|s|&mut s.content);
                    v.to_writer(&mut raw, linked, &$self.header)?;
                }
                $rawsecs.get_mut(&($sec.header.link as usize)).map(|s|s.header.size = s.size() as u64);
            }
            $sec.header.entsize = $size;
            $sec.header.size = raw.len() as u64;
            $sec.content = SectionContent::Raw(raw);
            $rawsecs.insert($i, $sec)
        }
    };
}


impl Elf {

    pub fn store_all(&mut self) -> Result<(), Error> {

        //split in raw and parsed
        let secnums = self.sections.len();
        let mut rawsecs : HashMap<usize, Section> = HashMap::new();
        let mut parsecs : HashMap<usize, Section> = HashMap::new();

        for (i, sec) in self.sections.drain(..).enumerate() {
            match (&sec.content, &sec.header.shtype) {
                (&SectionContent::Raw(_), &types::SectionType::STRTAB) => {
                    rawsecs.insert(i, sec);
                },
                (&SectionContent::None, _) => {
                    rawsecs.insert(i, sec);
                },
                (&SectionContent::Raw(_), _) => {
                    rawsecs.insert(i, sec);
                },
                _ => {
                    parsecs.insert(i, sec);
                }
            }
        }

        //correct sizes
        for (_,sec) in  &mut rawsecs {
            match sec.content { SectionContent::Raw(ref v)  => sec.header.size = v.len() as u64, _ => {},};
        }

        for i in 0..secnums {
            if let Some(mut sec) = parsecs.remove(&i) {
                match sec.content {
                    SectionContent::Relocations(vv) => {
                        let mut raw = Vec::new();
                        for v in vv {
                            v.to_writer(&mut raw, None, &self.header)?;
                        }
                        sec.header.entsize = Relocation::entsize(&self.header) as u64;
                        sec.header.size = raw.len() as u64;
                        sec.content = SectionContent::Raw(raw);
                        rawsecs.insert(i, sec);
                    },
                    SectionContent::Symbols(vv) => {
                        for (i, sym) in vv.iter().enumerate() {
                            if sym.bind == types::SymbolBind::GLOBAL {
                                sec.header.info = i as u32;
                                break;
                            }
                        }
                        insert_stuff_with_strtab!(self, sec, vv, Symbol::entsize(&self.header) as u64, rawsecs, i);
                    },
                    SectionContent::Dynamic(vv) => {
                        insert_stuff_with_strtab!(self, sec, vv, Dynamic::entsize(&self.header) as u64, rawsecs, i);
                    },
                    _ => unreachable!(),
                }
            }
        }

        for i in 0..secnums {
            if let Some(sec) = rawsecs.remove(&i) {
                self.sections.insert(i, sec);
            }
        }

        //shstrtab
        let mut shstrtab = String::from(".shstrtab\0");
        let mut inoff = shstrtab.len();
        for sec in &mut self.sections {
            shstrtab += &sec.name;
            shstrtab += "\0";
            sec.header.name = inoff as u32;
            inoff += sec.name.len() + 1;
        }

        //TODO don't push this if it exists
        self.sections.push(Section{
            header: SectionHeader {
                name:       0,
                shtype:     types::SectionType::STRTAB,
                flags:      types::SectionFlags::from_bits_truncate(0),
                addr:       0,
                offset:     0,
                size:       shstrtab.len() as u64,
                link:       0,
                info:       0,
                addralign:  0,
                entsize:    0,
            },
            name: String::from(".shstrtab"),
            content: SectionContent::Raw(shstrtab.into_bytes()),
        });
        self.header.shstrndx = self.sections.len() as u16 - 1;

        Ok(())
    }

    // at this point we assume the following state for all sections:
    //  - content is raw
    //  - header.size is correct
    //
    pub fn relayout(&mut self, pstart: u64, vstart: u64) -> Result<(), Error> {
        //calculate addresses and offsets
        let mut poff = pstart;
        let mut voff = vstart;

        for sec in &mut self.sections {
            sec.header.offset   = poff;
            sec.header.addr     = voff;
            poff += sec.size() as u64;
            voff += sec.header.size;
        };

        Ok(())
    }

    pub fn to_writer<R>(&mut self, io: &mut R) -> Result<(), Error> where R: Write + Seek {

        io.seek(SeekFrom::Start(0))?;
        let mut off = self.header.size();
        io.write(&vec![0;off])?;

        // segment headers
        // MUST be written before section content, because it MUST be in the first LOAD
        // otherwise the kernel passes an invalid aux vector
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

        //sections
        let mut sections = self.sections.to_vec();
        for sec in &mut sections {
            off = io.seek(SeekFrom::Current(0))? as usize;
            if off != sec.header.offset as usize && sec.header.offset != 0 && sec.header.flags.contains(types::SectionFlags::ALLOC) {

                //FIXME should we re-arrange the program headers instead?
                //this seems like something for a higher level api instead
                if sec.header.offset as usize > off  {
                    println!("FIXME: padding section {} because it should be at {:x} but would have been at {:x}",
                             sec.name, sec.header.offset, off);
                    io.write(&vec![0;sec.header.offset as usize - off])?;
                    off = io.seek(SeekFrom::Current(0))? as usize;
                } else {
                    println!("FIXME: section {} will be at a different physical location than intended.
                                  the resuling binary will probably not work.
                                  Section should be at {:x} but the current file is already size {:x}.",
                             sec.name, sec.header.offset, off);
                }

            }

            sec.header.offset = off as u64;
            match sec.content {
                SectionContent::Raw(ref v) => {
                    io.write(&v.as_ref())?;
                }
                _ => {},
            }
            let at = io.seek(SeekFrom::Current(0))? as usize;
            sec.header.size = (at - off) as u64;
            off = at;
        }
        self.sections = sections;


        //section headers

        let mut off = io.seek(SeekFrom::Current(0))? as usize;
        self.header.shoff = off as u64;
        for sec in &self.sections {
            sec.header.to_writer(&self.header, io)?;
        }
        let at = io.seek(SeekFrom::Current(0))? as usize;
        self.header.shnum       = self.sections.len() as u16;
        self.header.shentsize   = ((at - off)/ self.sections.len()) as u16;
        off = at;



        //hygene
        self.header.ehsize = self.header.size() as u16;

        io.seek(SeekFrom::Start(0))?;
        self.header.to_writer(io)?;

        Ok(())
    }
}




