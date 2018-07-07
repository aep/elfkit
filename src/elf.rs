
use header::Header;
use types;
use error::Error;
use section::*;
use segment::*;
use symbol;
use section;
use segment;

use indexmap::{IndexMap};
use std::collections::hash_map::{HashMap};
use std::io::{Read, Seek, SeekFrom, Write};
use std;
use std::iter::FromIterator;

#[derive(Default)]
pub struct Elf {
    pub header: Header,
    pub segments: Vec<SegmentHeader>,
    pub sections: Vec<Section>,
}

impl Elf {
    pub fn from_header(header: Header) -> Self {
        Self {
            header:     header,
            segments:   Vec::new(),
            sections:   Vec::new(),
        }
    }

    pub fn from_reader<R>(io: &mut R) -> Result<Elf, Error>
    where
        R: Read + Seek,
    {
        let header = Header::from_reader(io)?;

        // parse segments
        let mut segments = Vec::with_capacity(header.phnum as usize);
        io.seek(SeekFrom::Start(header.phoff))?;
        let mut buf = vec![0; header.phentsize as usize * header.phnum as usize];
        {
            io.read_exact(&mut buf)?;
            let mut bio = buf.as_slice();
            for _ in 0..header.phnum {
                let segment = SegmentHeader::from_reader(&mut bio, &header)?;
                segments.push(segment);
            }
        }

        // parse section headers
        let mut sections = Vec::with_capacity(header.shnum as usize);
        io.seek(SeekFrom::Start(header.shoff))?;
        buf.resize(header.shnum as usize * header.shentsize as usize,0);
        {
            io.read_exact(&mut buf)?;
            let mut bio = buf.as_slice();
            for _ in 0..header.shnum {
                let sh = SectionHeader::from_reader(&mut bio, &header)?;

                sections.push(Section{
                    name:       Vec::with_capacity(0),
                    content:    SectionContent::Unloaded,
                    header:     sh,
                    addrlock:   true,
                });
            }
        }

        // resolve section names
        let shstrtab = match sections.get(header.shstrndx as usize) {
            None => return Err(Error::MissingShstrtabSection),
            Some(sec) => {
                io.seek(SeekFrom::Start(sec.header.offset))?;
                let mut shstrtab = vec![0;sec.header.size as usize];
                io.read_exact(&mut shstrtab)?;
                shstrtab
            },
        };

        for ref mut sec in &mut sections {
            sec.name = shstrtab[sec.header.name as usize..]
                .split(|e| *e == 0)
                .next()
                .unwrap_or(&[0; 0])
                .to_vec();
        }

        Ok(Elf{
            header:     header,
            segments:   segments,
            sections:   sections,
        })
    }

    pub fn load<R> (&mut self, i: usize, io: &mut R) -> Result<(), Error>
        where
        R: Read + Seek,
    {
        let mut sec = std::mem::replace(&mut self.sections[i], Section::default());
        {
            let link = sec.header.link.clone();
            let linked = {
                if link < 1 || link as usize >= self.sections.len() {
                    None
                } else {
                    self.load(link as usize, io)?;
                    Some(&self.sections[link as usize])
                }
            };
            sec.from_reader(io, linked, &self.header)?;
        }
        self.sections[i] = sec;

        Ok(())
    }

    pub fn load_all<R> (&mut self, io: &mut R) -> Result<(), Error>
        where
        R: Read + Seek,
    {
        for i in 0..self.sections.len() {
            self.load(i, io)?;
        }
        Ok(())
    }

    /// write out everything to linked sections, such as string tables
    /// after calling this function, size() is reliable for all sections
    pub fn sync_all(&mut self) -> Result<(), Error> {
        match self.sections.iter().position(|s| s.name == b".shstrtab") {
            Some(i) => {
                self.header.shstrndx = i as u16;
                let mut shstrtab = std::mem::replace(
                    &mut self.sections[self.header.shstrndx as usize].content,
                    SectionContent::default(),
                );

                for sec in &mut self.sections {
                    sec.header.name = shstrtab
                        .as_strtab_mut()
                        .unwrap()
                        .insert(&sec.name)
                        as u32;
                }
                self.sections[self.header.shstrndx as usize].content = shstrtab;
            }
            None => {}
        };


        let mut dirty: Vec<usize> = (0..self.sections.len()).collect();
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

    pub fn to_writer<R>(&mut self, mut io: R) -> Result<(), Error>
    where
        R: Write + Seek,
    {
        io.seek(SeekFrom::Start(0))?;
        let off = self.header.size();
        io.write(&vec![0; off])?;

        // segment headers
        // MUST be written before section content, because it MUST be in the first LOAD
        // otherwise the kernel passes an invalid aux vector
        if self.segments.len() > 0 {
            self.header.phoff = off as u64;
            for seg in &self.segments {
                seg.to_writer(&self.header, &mut io)?;
            }
            let at = io.seek(SeekFrom::Current(0))? as usize;
            self.header.phnum = self.segments.len() as u16;
            self.header.phentsize = ((at - off) / self.segments.len()) as u16;
        }

        let headers: Vec<SectionHeader> = self.sections.iter().map(|s| s.header.clone()).collect();
        let mut sections = std::mem::replace(&mut self.sections, Vec::new());

        //sections
        sections.sort_unstable_by(|a, b| a.header.offset.cmp(&b.header.offset));
        for sec in sections {
            assert_eq!(
                io.seek(SeekFrom::Start(sec.header.offset))?,
                sec.header.offset
            );

            sec.to_writer(&mut io, &self.header)?;
        }


        //section headers
        if self.header.shstrndx > 0 {
            self.header.shoff = io.seek(SeekFrom::End(0))?;
            let alignment = if self.header.ident_class == types::Class::Class64 { 8 } else { 4 };
            let oa = self.header.shoff % alignment;
            if oa != 0 {
                self.header.shoff += alignment - oa;
                io.seek(SeekFrom::Start(self.header.shoff))?;
            }
            for sec in &headers {
                sec.to_writer(&self.header, &mut io)?;
            }
            self.header.shnum = headers.len() as u16;
            self.header.shentsize = SectionHeader::entsize(&self.header) as u16;
        }

        //hygene
        self.header.ehsize = self.header.size() as u16;

        io.seek(SeekFrom::Start(0))?;
        self.header.to_writer(&mut io)?;

        Ok(())
    }

    ///gnu ld compatibility. this is very inefficent,
    ///but not doing this might break some GNU tools that rely on specific gnu-ld behaviour
    /// - reorder symbols to have GLOBAL last
    /// - remove original SECTION symbols and add offset to reloc addend instead
    /// - insert new symbol sections on the top
    pub fn make_symtab_gnuld_compat(&mut self) -> Result<(), Error> {
        for i in 0..self.sections.len() {
            if self.sections[i].header.shtype == types::SectionType::SYMTAB {
                self._make_symtab_gnuld_compat(i);
            }
        }
        self.sync_all()
    }

    fn _make_symtab_gnuld_compat(&mut self, shndx: usize) {

        let mut original_size = self.sections[shndx].content.as_symbols().unwrap().len();

        let mut symtab_sec = HashMap::new();
        //I = new index
        //V.0 = old index
        //V.1 = sym
        let mut symtab_remap = Vec::new();
        for (i, link)  in self.sections[shndx].content.as_symbols_mut().unwrap().drain(..).enumerate() {
            if link.stype == types::SymbolType::SECTION {
                symtab_sec.insert(i, link);
            } else {
                symtab_remap.push((i, link));
            }
        }

        let mut symtab_gs = Vec::new();
        let mut symtab_ls = Vec::new();
        for (oi,sym) in symtab_remap {
            if sym.bind == types::SymbolBind::GLOBAL {
                symtab_gs.push((oi, sym));
            } else {
                symtab_ls.push((oi, sym));
            }
        }
        symtab_gs.sort_unstable_by(|a,b|{
            a.1.value.cmp(&b.1.value)
        });


        symtab_ls.insert(0, (original_size, symbol::Symbol::default()));
        original_size += 1;

        let mut nu_sec_syms = vec![0];
        for i in 1..self.sections.len() {
            symtab_ls.insert(i, (original_size, symbol::Symbol{
                shndx:  symbol::SymbolSectionIndex::Section(i as u16),
                value:  0,
                size:   0,
                name:   Vec::new(),
                stype:  types::SymbolType::SECTION,
                bind:   types::SymbolBind::LOCAL,
                vis:    types::SymbolVis::DEFAULT,
                _name:  0,
            }));
            nu_sec_syms.push(original_size);
            original_size += 1;
        }

        symtab_ls.push((original_size, symbol::Symbol{
            shndx:  symbol::SymbolSectionIndex::Absolute,
            value:  0,
            size:   0,
            name:   Vec::new(),
            stype:  types::SymbolType::FILE,
            bind:   types::SymbolBind::LOCAL,
            vis:    types::SymbolVis::DEFAULT,
            _name:  0,
        }));
        //original_size += 1;


        let symtab_remap : IndexMap<usize, symbol::Symbol>
            = IndexMap::from_iter(symtab_ls.into_iter().chain(symtab_gs.into_iter()));

        for sec in &mut self.sections {
            match sec.header.shtype {
                types::SectionType::RELA => {
                    if sec.header.link != shndx as u32{
                        continue;
                    }
                    for reloc in sec.content.as_relocations_mut().unwrap().iter_mut() {
                        if let Some(secsym) = symtab_sec.get(&(reloc.sym as usize)) {
                            if let symbol::SymbolSectionIndex::Section(so) = secsym.shndx {
                                reloc.addend += secsym.value as i64;
                                reloc.sym     = nu_sec_syms[so as usize] as u32;
                            } else {
                                unreachable!();
                            }
                        }

                        reloc.sym = symtab_remap.get_full(&(reloc.sym as usize))
                            .expect("bug in elfkit: dangling reloc").0 as u32;
                    }
                },
                _ => {},
            }
        }

        self.sections[shndx].content = section::SectionContent::Symbols(
            symtab_remap.into_iter().map(|(_,v)|v).collect());
    }


    /// reorder to minimize segmentation
    /// will only reorder sections that come after the last locked section
    pub fn reorder(self: &mut Elf) -> Result<HashMap<usize,usize>, Error> {

        let mut reorder = Vec::new();
        loop {
            let shndx = self.sections.len() - 1;
            if shndx < 1 || self.sections[shndx].addrlock {
                break;
            }
            reorder.push((shndx, self.sections.pop().unwrap()));
        }


        reorder.sort_by(|&(_,ref s1),&(_,ref s2)|{
            if s1.header.shtype != s2.header.shtype {
                if s1.header.shtype == types::SectionType::NOBITS {
                    return std::cmp::Ordering::Greater;
                }
            }

            let s1_a = s1.header.flags.contains(types::SectionFlags::ALLOC);
            let s1_w = s1.header.flags.contains(types::SectionFlags::WRITE);
            let s2_a = s2.header.flags.contains(types::SectionFlags::ALLOC);
            let s2_w = s2.header.flags.contains(types::SectionFlags::WRITE);

            if s1_a != s2_a {
                if s1_a {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }
            if s1_w != s2_w {
                if s1_w {
                    return std::cmp::Ordering::Greater;
                } else {
                    return std::cmp::Ordering::Less;
                }
            }

            if s1.header.shtype != s2.header.shtype  {
                return s1.header.shtype.to_u32().cmp(&s2.header.shtype.to_u32());
            }

            //this is just for stabilization
            //TODO but we should probably do something else
            s1.name.cmp(&s2.name)
        });

        let mut remap = HashMap::new();
        for (i,sec) in reorder {
            remap.insert(i, self.sections.len());
            self.sections.push(sec);
        }

        for sec in &mut self.sections {
            if let Some(v) = remap.get(&(sec.header.link as usize)) {
                sec.header.link = *v as u32;
            }
            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if let Some(v) = remap.get(&(sec.header.info as usize)) {
                    sec.header.info = *v as u32;
                }
            }
        }

        Ok(remap)
    }

    pub fn layout(self: &mut Elf) -> Result<(), Error> {
        self.sync_all()?;

        let dbg_old_segments_count = self.segments.len();

        self.segments.clear();

        trace!("start of Elf::layout segmentation");

        let mut current_load_segment_flags = types::SegmentFlags::READABLE;
        let mut current_load_segment_poff = 0;
        let mut current_load_segment_voff = 0;
        let mut current_load_segment_pstart = 0;
        let mut current_load_segment_vstart = 0;

        let mut poff = 0;
        let mut voff = poff;

        let mut dbg_old_addresses = vec![self.sections[0].header.addr];

        trace!("    name     \tsize\tpoff\tvoff\tpstart\tvstart\tflags");
        for (shndx, sec)  in self.sections.iter_mut().enumerate().skip(1) {
            dbg_old_addresses.push(sec.header.addr);

            trace!(" > {:<10.10}\t{}\t{}\t{}\t{}\t{}\t{:?}",
                     String::from_utf8_lossy(&sec.name), sec.header.size, poff, voff,
                     current_load_segment_pstart,
                     current_load_segment_vstart,
                     current_load_segment_flags);

            if sec.header.addralign > 0 {
                let oa = poff % sec.header.addralign;
                if oa != 0 {
                    poff += sec.header.addralign - oa;
                    voff += sec.header.addralign - oa;
                }
                trace!("   ^ realigned for {} to voff 0x{:x}", sec.header.addralign, voff);
            }

            if sec.header.shtype != types::SectionType::NOBITS {
                if poff > voff {
                    panic!("elfkit: relayout: poff>voff 0x{:x}>0x{:x} in {}.", poff, voff,
                           String::from_utf8_lossy(&sec.name));
                }
                if (voff - poff) % 0x200000 != 0 {
                    trace!("   ^ causes segmentation by load alignment");
                    if sec.header.flags.contains(types::SectionFlags::EXECINSTR) {
                        current_load_segment_flags.insert(types::SegmentFlags::EXECUTABLE);
                    }
                    if sec.header.flags.contains(types::SectionFlags::WRITE) {
                        current_load_segment_flags.insert(types::SegmentFlags::WRITABLE);
                    }
                    self.segments.push(segment::SegmentHeader {
                        phtype: types::SegmentType::LOAD,
                        flags:  current_load_segment_flags,
                        offset: current_load_segment_pstart,
                        filesz: current_load_segment_poff - current_load_segment_pstart,
                        vaddr:  current_load_segment_vstart,
                        paddr:  current_load_segment_vstart,
                        memsz:  current_load_segment_voff - current_load_segment_vstart,
                        align:  0x200000,
                    });

                    voff += 0x200000 - ((voff - poff) % 0x200000);

                    current_load_segment_pstart = poff;
                    current_load_segment_vstart = voff;
                    current_load_segment_flags = types::SegmentFlags::READABLE;
                }
            }


            if sec.header.flags.contains(types::SectionFlags::ALLOC) {
                // can mix exec and read segments. at least gnuld does, so whatevs?
                if sec.header.flags.contains(types::SectionFlags::EXECINSTR) {
                    current_load_segment_flags.insert(types::SegmentFlags::EXECUTABLE);
                } else {}

                // cannot mix write and non write segments, danger zone
                if sec.header.flags.contains(types::SectionFlags::WRITE) !=
                current_load_segment_flags.contains(types::SegmentFlags::WRITABLE) {
                    if current_load_segment_voff >  current_load_segment_vstart || shndx == 1 {
                        //println!("   ^ causes segmentation by protection change");
                        self.segments.push(segment::SegmentHeader {
                            phtype: types::SegmentType::LOAD,
                            flags:  current_load_segment_flags,
                            offset: current_load_segment_pstart,
                            filesz: current_load_segment_poff - current_load_segment_pstart,
                            vaddr:  current_load_segment_vstart,
                            paddr:  current_load_segment_vstart,
                            memsz:  current_load_segment_voff - current_load_segment_vstart,
                            align:  0x200000,
                        });
                        voff += 0x200000 - ((voff - poff) % 0x200000);
                        current_load_segment_pstart = poff;
                        current_load_segment_vstart = voff;
                        current_load_segment_flags = types::SegmentFlags::READABLE;
                    } else {
                        trace!("   ^ segmentation protection change supressed because it would be empty \
                                 voff {} <= vstart {}",
                                 current_load_segment_voff, current_load_segment_vstart);
                    }

                    if sec.header.flags.contains(types::SectionFlags::WRITE) {
                        current_load_segment_flags.insert(types::SegmentFlags::WRITABLE);
                    } else {
                        current_load_segment_flags.remove(types::SegmentFlags::WRITABLE);
                    }
                }
            }


            sec.header.offset = poff;
            poff += sec.size(&self.header) as u64;

            sec.header.addr = voff;
            voff += sec.header.size;
            trace!("   = final addr 0x{:x}", sec.header.addr);

            if sec.header.flags.contains(types::SectionFlags::ALLOC) {
                current_load_segment_poff = poff;
                current_load_segment_voff = voff;

                if sec.header.flags.contains(types::SectionFlags::TLS) {
                    self.segments.push(segment::SegmentHeader {
                        phtype: types::SegmentType::TLS,
                        flags:  current_load_segment_flags,
                        offset: sec.header.offset,
                        filesz: sec.header.size,
                        vaddr:  sec.header.addr,
                        paddr:  sec.header.addr,
                        memsz:  sec.header.size,
                        align:  sec.header.addralign,
                    });
                }

                match sec.name.as_slice() {
                    b".dynamic" => {
                        self.segments.push(segment::SegmentHeader {
                            phtype: types::SegmentType::DYNAMIC,
                            flags: types::SegmentFlags::READABLE | types::SegmentFlags::WRITABLE,
                            offset: sec.header.offset,
                            filesz: sec.header.size,
                            vaddr: sec.header.addr,
                            paddr: sec.header.addr,
                            memsz: sec.header.size,
                            align: 0x8,
                        });
                    }
                    b".interp" => {
                        self.segments.push(segment::SegmentHeader {
                            phtype: types::SegmentType::INTERP,
                            flags: types::SegmentFlags::READABLE,
                            offset: sec.header.offset,
                            filesz: sec.header.size,
                            vaddr: sec.header.addr,
                            paddr: sec.header.addr,
                            memsz: sec.header.size,
                            align: 0x1,
                        });
                    }
                    _ => {}
                }

            }
        }
        if current_load_segment_voff >  current_load_segment_vstart {
            trace!("   > segmentation caused by end of sections");
            self.segments.push(segment::SegmentHeader {
                phtype: types::SegmentType::LOAD,
                flags:  current_load_segment_flags,
                offset: current_load_segment_pstart,
                filesz: current_load_segment_poff - current_load_segment_pstart,
                vaddr:  current_load_segment_vstart,
                paddr:  current_load_segment_vstart,
                memsz:  current_load_segment_voff - current_load_segment_vstart,
                align:  0x200000,
            });
        }


        self.header.phnum     = self.segments.len() as u16 + 1;
        self.header.phentsize = segment::SegmentHeader::entsize(&self.header) as u16;
        self.header.phoff     = self.header.size() as u64;

        self.header.ehsize    = self.header.size() as u16;
        let mut hoff = (self.header.phnum as u64 * self.header.phentsize as u64) + self.header.ehsize as u64;

        ///TODO this is shitty, because we need to replicate all the alignment code
        ///also most of those sections dont actually need to be moved
        for sec in &mut self.sections[1..] {
            if sec.header.addralign > 0 {
                let oa = hoff % sec.header.addralign;
                if oa != 0 {
                    hoff += sec.header.addralign - oa;
                }
            }
        }
        for sec in &mut self.sections[1..] {
            sec.header.offset += hoff;
            sec.header.addr   += hoff;
        }


        let mut seen_first_load = false;
        for seg in self.segments.iter_mut() {
            if seg.phtype == types::SegmentType::LOAD && !seen_first_load {
                seen_first_load = true;
                seg.memsz  += hoff;
                seg.filesz += hoff;
            } else {
                seg.offset += hoff;
                seg.vaddr  += hoff;
                seg.paddr  += hoff;
            }
        }

        self.segments.insert(0, segment::SegmentHeader {
            phtype: types::SegmentType::PHDR,
            flags: types::SegmentFlags::READABLE | types::SegmentFlags::EXECUTABLE,
            offset: self.header.phoff,
            filesz: self.header.phnum as u64 * self.header.phentsize as u64,
            vaddr:  self.header.phoff,
            paddr:  self.header.phoff,
            memsz:  self.header.phnum as u64 * self.header.phentsize as u64,
            align:  0x8,
        });

        trace!("done {} segments", self.segments.len());

        for i in 0..self.sections.len() {
            if self.sections[i].addrlock && self.sections[i].header.addr != dbg_old_addresses[i] {

                let mut cause = String::from("preceeding section or header has changed in size");
                if dbg_old_segments_count != self.segments.len() {
                    cause = format!("number of segments changed from {} to {}",
                                    dbg_old_segments_count, self.segments.len());
                }

                return Err(Error::MovingLockedSection{
                    sec: String::from_utf8_lossy(&self.sections[i].name).into_owned(),
                    old_addr: dbg_old_addresses[i],
                    new_addr: self.sections[i].header.addr,
                    cause:    cause,
                });
            }
        }


        Ok(())
    }











    //TODO this code isnt tested at all
    //TODO the warnings need to be emited when calling store_all instead
    pub fn remove_section(&mut self, at: usize) -> Result<(Section), Error> {
        let r = self.sections.remove(at);

        for sec in &mut self.sections {
            if sec.header.link == at as u32 {
                sec.header.link = 0;
                //println!("warning: removed section {} has a dangling link from {}", at, sec.name);
            } else if sec.header.link > at as u32 {
                sec.header.link -= 1;
            }

            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == at as u32 {
                    sec.header.info = 0;
                    //println!("warning: removed section {} has a dangling info link from {}", at,
                    //sec.name);
                } else if sec.header.info > at as u32 {
                    sec.header.info -= 1;
                }
            }
        }

        Ok((r))
    }
    pub fn insert_section(&mut self, at: usize, sec: Section) -> Result<(), Error> {
        self.sections.insert(at, sec);

        for sec in &mut self.sections {
            if sec.header.link >= at as u32 {
                sec.header.link += 1;
            }

            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info > at as u32 {
                    sec.header.info += 1;
                }
            }
        }

        Ok(())
    }

    pub fn move_section(&mut self, from: usize, mut to: usize) -> Result<(), Error> {
        if to == from {
            return Ok(());
        }
        if to > from {
            to -= 1;
        }


        for sec in &mut self.sections {
            if sec.header.link == from as u32 {
                sec.header.link = 999999;
            }
            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == from as u32 {
                    sec.header.info = 999999;
                }
            }
        }
        let sec = self.remove_section(from)?;
        self.insert_section(to, sec)?;
        for sec in &mut self.sections {
            if sec.header.link == 999999 {
                sec.header.link = to as u32;
            }
            if sec.header.flags.contains(types::SectionFlags::INFO_LINK) {
                if sec.header.info == 999999 {
                    sec.header.info = to as u32;
                }
            }
        }

        Ok(())
    }
}
