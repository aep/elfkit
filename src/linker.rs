use {Elf, Error, Header, SectionHeader, SegmentHeader, types};

pub struct Linker {
}

impl Linker {
    pub fn segments(elf : &Elf) -> Result<Vec<SegmentHeader>, Error> {
        let mut r = Vec::new();
        if elf.sections.len() < 2 {
            return Ok(r)
        }

        let mut vshift = 0 as i64;
        let mut voff  = elf.sections[1].header.addr;
        let mut poff  = elf.sections[1].header.offset;
        let mut vstart = 0;
        let mut pstart = 0;
        let mut flags = types::SegmentFlags::READABLE;

        for i in 0..elf.sections.len() {
            let section = &elf.sections[i];


            match section.name.as_ref() {
                ".dynamic" => {
                    r.push(SegmentHeader{
                        phtype: types::SegmentType::DYNAMIC,
                        flags:  types::SegmentFlags::READABLE | types::SegmentFlags::WRITABLE,
                        offset: section.header.offset,
                        filesz: section.header.size,
                        vaddr:  section.header.addr,
                        paddr:  section.header.addr,
                        memsz:  section.header.size,
                        align:  0x8,
                    });
                },
                ".interp" => {
                    r.push(SegmentHeader{
                        phtype: types::SegmentType::INTERP,
                        flags:  types::SegmentFlags::READABLE,
                        offset: section.header.offset,
                        filesz: section.header.size,
                        vaddr:  section.header.addr,
                        paddr:  section.header.addr,
                        memsz:  section.header.size,
                        align:  0x1,
                    });
                },
                _ => {},
            }

            if section.header.flags.contains(types::SectionFlags::TLS) {
                    r.push(SegmentHeader{
                        phtype: types::SegmentType::TLS,
                        flags:  types::SegmentFlags::READABLE,
                        offset: section.header.offset,
                        filesz: section.header.size,
                        vaddr:  section.header.addr,
                        paddr:  section.header.addr,
                        memsz:  section.header.size,
                        align:  0x20,
                    });
            }

            //emulate ld behaviour by just skipping over sections that are not allocateable,
            //sometimes actually allocating them. pretty weird, but i'm scared of more kernel gotchas
            //if i diverge from ld behaviour
            if !section.header.flags.contains(types::SectionFlags::ALLOC) {
                continue
            }

            if section.header.shtype == types::SectionType::NOBITS {
                voff = section.header.addr + section.header.size;
                poff = section.header.offset;
                continue;
            }

            if section.header.offset as i64 + vshift != section.header.addr as i64 {
                r.push(SegmentHeader{
                    phtype: types::SegmentType::LOAD,
                    flags:  flags,
                    offset: pstart,
                    filesz: poff - pstart,
                    vaddr:  vstart,
                    paddr:  vstart,
                    memsz:  voff - vstart,
                    align:  0x10000,
                });

                vshift = section.header.addr as i64 - section.header.offset as i64;
                vstart = section.header.addr;
                pstart = section.header.offset;
                voff  = 0;
                poff  = 0;
                flags = types::SegmentFlags::READABLE;
            }

            voff = section.header.addr + section.header.size;
            poff = section.header.offset + match section.header.shtype {
                types::SectionType::NOBITS => 0,
                _ => section.header.size,
            };

            if section.header.flags.contains(types::SectionFlags::EXECINSTR) {
                flags.insert(types::SegmentFlags::EXECUTABLE);
            }
            if section.header.flags.contains(types::SectionFlags::WRITE) {
                flags.insert(types::SegmentFlags::WRITABLE);
            }
        }
        r.push(SegmentHeader{
            phtype: types::SegmentType::LOAD,
            flags:  flags,
            offset: pstart,
            filesz: poff - pstart,
            vaddr:  vstart,
            paddr:  vstart,
            memsz:  voff - vstart,
            align:  0x10000,
        });


        let first_vshift = elf.sections[1].header.addr - elf.sections[1].header.offset;
        let segments_size = SegmentHeader::entsize(&elf.header) * (r.len() + 1);
        r.insert(0,SegmentHeader{
            phtype: types::SegmentType::PHDR,
            flags:  types::SegmentFlags::READABLE | types::SegmentFlags::EXECUTABLE,
            offset: elf.header.size() as u64,
            filesz: segments_size as u64,
            vaddr:  first_vshift + elf.header.size() as u64,
            paddr:  first_vshift + elf.header.size() as u64,
            memsz:  segments_size as u64,
            align:  0x8,
        });

        Ok(r)
    }
}
