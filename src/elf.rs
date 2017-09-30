use header::Header;
use types;
use error::Error;
use section::*;
use symbol::*;
use dynamic::*;
use relocation::*;
use strtab::*;
use segment::*;

use std::io::{Read, Write, Seek, SeekFrom};
use num_traits::{FromPrimitive, ToPrimitive};
use std::io::BufWriter;
use std;

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
            SectionContent::Raw(_) | SectionContent::None => false,
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









    fn store(eh: &Header, mut sec: Section, mut linked: Option<&mut SectionContent>) -> Result<(Section), Error> {
        match sec.content {
            SectionContent::Relocations(vv) => {
                let mut raw = Vec::new();
                for v in vv {
                    v.to_writer(&mut raw, None, eh)?;
                }
                sec.header.entsize  = Relocation::entsize(eh) as u64;
                sec.header.size     = raw.len() as u64;
                sec.content         = SectionContent::Raw(raw);
            },
            SectionContent::Symbols(vv) => {
                for (i, sym) in vv.iter().enumerate() {
                    if sym.bind == types::SymbolBind::GLOBAL {
                        sec.header.info = i as u32;
                        break;
                    }
                }
                let mut raw = Vec::new();
                for v in vv {
                    v.to_writer(&mut raw, linked.as_mut().map(|r|&mut **r), eh)?;
                }
                sec.header.entsize  = Symbol::entsize(eh) as u64;
                sec.header.size     = raw.len() as u64;
                sec.content         = SectionContent::Raw(raw);
            },
            SectionContent::Dynamic(vv) => {
                let mut raw = Vec::new();
                for v in vv {
                    v.to_writer(&mut raw, linked.as_mut().map(|r|&mut **r), eh)?;
                }
                sec.header.entsize  = Dynamic::entsize(eh) as u64;
                sec.header.size     = raw.len() as u64;
                sec.content         = SectionContent::Raw(raw);

            },
            SectionContent::Strtab(v) => {
                let mut raw = Vec::new();
                v.to_writer(&mut raw, None, eh)?;
                sec.header.entsize  = Strtab::entsize(eh) as u64;
                sec.header.size     = raw.len() as u64;
                sec.content         = SectionContent::Raw(raw);
            },
            SectionContent::None | SectionContent::Raw(_) => {},
        };
        Ok(sec)
    }

    fn store_at(&mut self, i: usize) -> Result<(bool), Error>{
        let is_stored = match self.sections[i].content {
            SectionContent::Raw(_) | SectionContent::None => true,
            _ => false,
        };

        if is_stored {
            return Ok((false))
        }

        //take out the original. this is to work around the borrow checker
        let mut sec = std::mem::replace(&mut self.sections[i], Section::default());
        {
            //circular section dependencies are possible, so the best idea i have right now
            //is loading the linked section again and iterate until no sections are loaded anymore
            let linked = {
                if sec.header.link < 1 || sec.header.link as usize >= self.sections.len() {
                    None
                } else {
                    self.load_at(sec.header.link as usize);
                    Some(&mut self.sections[sec.header.link as usize].content)
                }
            };

            sec = Elf::store(&self.header, sec, linked)?;

        }

        //put it back in
        self.sections[i] = sec;

        Ok((true))
    }

    pub fn store_all(&mut self) -> Result<(), Error> {
        self.header.shstrndx = match self.sections.iter().position(|s|s.name == ".shstrtab") {
            Some(i) => i as u16,
            None => return Err(Error::MissingShstrtabSection),
        };
        let mut shstrtab = std::mem::replace(
            &mut self.sections[self.header.shstrndx as usize].content, SectionContent::default());

        for sec in &mut self.sections {
            sec.header.name = shstrtab.as_strtab_mut().unwrap().insert(sec.name.as_bytes().to_vec()) as u32;
        }
        self.sections[self.header.shstrndx as usize].content = shstrtab;

        loop {
            let mut still_need_to_store = false;
            for i in 0..self.sections.len(){
                still_need_to_store = still_need_to_store || self.store_at(i)?;
            }
            if !still_need_to_store {
                break
            }
        }

        Ok(())
    }

    /// write out everything to linked sections, such as string tables
    /// after calling this function, size() is reliable for all sections
    pub fn sync_all(&mut self) -> Result<(), Error> {
        match self.sections.iter().position(|s|s.name == ".shstrtab") {
            Some(i) => {
                self.header.shstrndx = i as u16;
                let mut shstrtab = std::mem::replace(
                    &mut self.sections[self.header.shstrndx as usize].content, SectionContent::default());

                for sec in &mut self.sections {
                    sec.header.name = shstrtab.as_strtab_mut().unwrap().insert(sec.name.as_bytes().to_vec()) as u32;
                }
                self.sections[self.header.shstrndx as usize].content = shstrtab;
            }
            None => {},
        };


        let mut dirty : Vec<usize> = (0..self.sections.len()).collect();
        while dirty.len() > 0 {
            for i in std::mem::replace(&mut dirty, Vec::new()).iter() {
                //work around the borrow checker
                let mut sec = std::mem::replace(&mut self.sections[*i], Section::default());
                {
                    let linked = {
                        if sec.header.link < 1 || sec.header.link as usize >= self.sections.len() {
                            None
                        } else {
                            dirty.push(sec.header.link as usize);
                            self.load_at(sec.header.link as usize);
                            Some(&mut self.sections[sec.header.link as usize].content)
                        }
                    };
                    sec.sync(&self.header, linked)?;
                }

                //put it back in
                self.sections[*i] = sec;
            }
        }

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

        let headers : Vec<SectionHeader> = self.sections.iter().map(|s|s.header.clone()).collect();
        let mut sections = std::mem::replace(&mut self.sections, Vec::new());

        //sections
        sections.sort_unstable_by(|a,b|a.header.offset.cmp(&b.header.offset));
        for sec in sections {
            let off = io.seek(SeekFrom::Current(0))? as usize;

            assert_eq!(io.seek(SeekFrom::Start(sec.header.offset))?, sec.header.offset);
            match sec.content {
                SectionContent::Raw(ref v) => {
                    if off > sec.header.offset as usize {
                        println!("BUG: section layout is broken. \
would write section '{}' at position 0x{:x} over previous section that ended at 0x{:x}",
sec.name, sec.header.offset, off);
                    }
                    io.write(&v.as_ref())?;
                }
                _ => {},
            }
        }


        //section headers
        let mut off = io.seek(SeekFrom::End(0))? as usize;
        self.header.shoff = off as u64;
        for sec in &headers{
            sec.to_writer(&self.header, io)?;
        }
        let at = io.seek(SeekFrom::Current(0))? as usize;
        self.header.shnum       = headers.len() as u16;
        self.header.shentsize   = SectionHeader::entsize(&self.header) as u16;
        off = at;

        //hygene
        self.header.ehsize = self.header.size() as u16;

        io.seek(SeekFrom::Start(0))?;
        self.header.to_writer(io)?;

        Ok(())
    }

    //TODO the warnings need to be emited when calling store_all instead
    pub fn remove_section(&mut self, at: usize) -> Result<(Section), Error> {
        let r = self.sections.remove(at);

        for sec in &mut self.sections {
            if sec.header.link == at as u32{
                sec.header.link = 0;
                //println!("warning: removed section {} has a dangling link from {}", at, sec.name);
            } else if sec.header.link > at as u32{
                sec.header.link -= 1;
            }

            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == at as u32{
                    sec.header.info = 0;
                   //println!("warning: removed section {} has a dangling info link from {}", at, sec.name);
                } else if sec.header.info > at as u32{
                    sec.header.info -= 1;
                }
            }
        }

        Ok((r))
    }
    pub fn insert_section(&mut self, at: usize, sec: Section) -> Result<(), Error> {
        self.sections.insert(at, sec);

        for sec in &mut self.sections {
            if sec.header.link >= at as u32{
                sec.header.link += 1;
            }

            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info > at as u32{
                    sec.header.info += 1;
                }
            }
        }

        Ok(())
    }

    pub fn move_section(&mut self, from: usize, mut to:usize) -> Result<(), Error> {
        if to == from {
            return Ok(())
        }
        if to > from {
            to -= 1;
        }


        for sec in &mut self.sections {
            if sec.header.link == from as u32{
                sec.header.link = 999999;
            }
            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == from as u32{
                    sec.header.info = 999999;
                }
            }
        }
        let sec = self.remove_section(from)?;
        self.insert_section(to, sec)?;
        for sec in &mut self.sections {
            if sec.header.link == 999999{
                sec.header.link = to as u32;
            }
            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == 999999{
                    sec.header.info = to as u32;
                }
            }
        }

        Ok(())
    }
}





