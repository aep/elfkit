use {Elf, Error, Header, SectionHeader, SegmentHeader, types, Dynamic};
use dynamic::DynamicContent;
use relocation::RelocationType;

/**
 * high level linker stuff
 * this is the only api making assumptions based on section names.
 */
/// generate program headers from fully layouted sections.
/// sections must be synced
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
                align:  0x200000,
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
        align:  0x200000,
    });

    if elf.sections[1].header.offset > elf.sections[1].header.addr {
        return Err(Error::FirstSectionOffsetCanNotBeLargerThanAddress);
    }

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


/// generate dynamic linker instructions from fully layouted sections.
/// sections must be synced
/// returned list is null terminated, do not append, but call insert instead.
/// some object types might need additional instructions such as NEEDED and FLAGS_1
/// which cannot be generated here

pub fn dynamic(elf: &Elf) -> Result<Vec<Dynamic>, Error> {
    let mut r = Vec::new();

    for sec in &elf.sections {
        match sec.name.as_ref() {
            ".hash" => {
                r.push(Dynamic{
                    dhtype: types::DynamicType::HASH,
                    content: DynamicContent::Address(sec.header.addr),
                });
            },
            ".dynstr" => {
                r.push(Dynamic{
                    dhtype: types::DynamicType::STRTAB,
                    content: DynamicContent::Address(sec.header.addr),
                });

                r.push(Dynamic{
                    dhtype: types::DynamicType::STRSZ,
                    content: DynamicContent::Address(sec.header.size),
                });
            },
            ".dynsym" => {
                r.push(Dynamic{
                    dhtype: types::DynamicType::SYMTAB,
                    content: DynamicContent::Address(sec.header.addr),
                });
                r.push(Dynamic{
                    dhtype: types::DynamicType::SYMENT,
                    content: DynamicContent::Address(sec.header.entsize),
                });
            },
            ".rela.dyn" => {
                r.push(Dynamic{
                    dhtype:  types::DynamicType::RELA,
                    content: DynamicContent::Address(sec.header.addr),
                });
                r.push(Dynamic{
                    dhtype:  types::DynamicType::RELASZ,
                    content: DynamicContent::Address(sec.header.size),
                });
                r.push(Dynamic{
                    dhtype:  types::DynamicType::RELAENT,
                    content: DynamicContent::Address(sec.header.entsize),
                });

                let first_non_rela = match sec.content.as_relocations() {
                    None => return Err(Error::UnexpectedSectionContent),
                    Some(v) => v.iter().position(|ref r| {
                        r.rtype != RelocationType::R_X86_64_RELATIVE &&
                            r.rtype != RelocationType::R_X86_64_JUMP_SLOT }).
                        unwrap_or(v.len()),
                } as u64;


                if first_non_rela > 0 {
                    r.push(Dynamic{
                        dhtype:  types::DynamicType::RELACOUNT,
                        content: DynamicContent::Address(first_non_rela),
                    });
                }

                if first_non_rela < sec.content.as_relocations().unwrap().len() as u64{
                    r.push(Dynamic{
                        dhtype:  types::DynamicType::TEXTREL,
                        content: DynamicContent::Address(first_non_rela),
                    });

                }
            },
            _ => {},
        }
    }

    r.push(Dynamic{
        dhtype:  types::DynamicType::NULL,
        content: DynamicContent::Address(0),
    });
    Ok(r)
}


pub fn relayout(elf: &mut Elf, pstart: u64) -> Result<(), Error> {


    let mut poff  = pstart;
    let mut voff  = pstart;

    for sec in &mut elf.sections[1..] {
        if sec.header.shtype != types::SectionType::NOBITS {
            if (voff - poff) % 0x200000 != 0{
                voff += 0x200000 - ((voff - poff) % 0x200000)
            }
        }
        sec.header.offset = poff;
        poff += sec.size(&elf.header) as u64;

        sec.header.addr = voff;
        voff += sec.header.size;

    };

    Ok(())
}
