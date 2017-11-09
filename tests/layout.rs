extern crate elfkit;

use elfkit::{Elf, Section, SectionHeader, SectionContent, types, dynamic, segment};


fn fixture_section_dynamic() -> Section {
    Section {
        name: b".dynamic".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC,
            addr:       0x123,
            offset:     0x654,
            size:       0x200,
            link:       0,
            info:       0,
            addralign:  8,
            entsize:    0,
        },
        content: SectionContent::Dynamic(vec![dynamic::Dynamic{
            dhtype:  types::DynamicType::NULL,
            content: dynamic::DynamicContent::None,
        }]),
        addrlock: false,
    }
}

fn fixture_section_interp() -> Section {
    Section {
        name: b".interp".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC,
            addr:       0x923,
            offset:     0x54,
            size:       0x1283,
            link:       0,
            info:       0,
            addralign:  1,
            entsize:    0,
        },
        content: SectionContent::Raw(b"/lib/asdfblurp.ld".to_vec()),
        addrlock: false,
    }
}

fn fixture_section_text() -> Section {
    Section {
        name: b".text".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC | types::SectionFlags::EXECINSTR,
            addr:       0x123,
            offset:     0x654,
            size:       0x200,
            link:       0,
            info:       0,
            addralign:  16,
            entsize:    0,
        },
        content: SectionContent::Raw(vec![0;149]), // this is a prime number to test alingment
        addrlock: false,
    }
}
fn fixture_section_rodata() -> Section {
    Section {
        name: b".rodata".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC,
            addr:       0x123,
            offset:     0x654,
            size:       0x200,
            link:       0,
            info:       0,
            addralign:  16,
            entsize:    0,
        },
        content: SectionContent::Raw(vec![0;149]), // this is a prime number to test alingment
        addrlock: false,
    }
}

fn fixture_section_data() -> Section {
    Section {
        name: b".data".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
            addr:       0x123,
            offset:     0x654,
            size:       0x200,
            link:       0,
            info:       0,
            addralign:  16,
            entsize:    0,
        },
        content: SectionContent::Raw(vec![0;149]), // this is a prime number to test alingment
        addrlock: false,
    }
}

fn fixture_section_bss() -> Section {
    Section {
        name: b".bss".to_vec(),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::NOBITS,
            flags:      types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
            addr:       0x123,
            offset:     0x654,
            size:       0x27,
            link:       0,
            info:       0,
            addralign:  16,
            entsize:    0,
        },
        content: SectionContent::None,
        addrlock: false,
    }
}

#[test]
fn layout_just_text() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_text());
    elf.layout().unwrap();

    assert_eq!(elf.sections[1].name, b".text",
       ".text section must be at shndx 1");
    assert_eq!(elf.sections[1].header.offset, elf.sections[1].header.addr,
       ".text section offset and address must be identical");

    assert_eq!(elf.segments.len(), 2,
        "expect exactly two segments");

    let phdr_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::PHDR).collect();
    assert_eq!(phdr_segments.len(), 1,
        "expecting exactly one phdr segment");
    let phdr = phdr_segments.get(0).unwrap();;

    assert_eq!(phdr.offset, elf.header.phoff,
        "phdr.offset must be identical to elf header phoff");

    assert_eq!(phdr.offset, elf.header.ehsize as u64,
        "(phdr.offset == elf.header.ehsize) phdr must follow exactly elf header");

    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    assert_eq!(load_segments.len(), 1,
        "expect exactly one load segment");

    let segment = load_segments.get(0).unwrap();;
    assert_eq!(segment.offset,0,
        "first load segment must start at zero");
    assert_eq!(segment.vaddr, 0,
        "first load segment must start at zero");
    assert_eq!(segment.paddr, 0,
        "first load segment must start at zero");

    assert!(segment.vaddr <= elf.header.phoff,
        "first load segment must contain phdr (because of a bug in the linux kernel");
    assert_eq!(segment.vaddr, segment.paddr,
        "first load segment must have offset == addr (because of linux kernel stuff)");
    assert_eq!(segment.vaddr, segment.offset,
        "first load segment must have offset == addr (because of linux kernel stuff)");

    assert!(segment.vaddr  <= elf.sections[1].header.addr,
        "first load segment must start at or before .text");
    assert!(segment.offset <= elf.sections[1].header.offset,
        "first load segment must start at or before .text");

    assert!(segment.vaddr  + segment.memsz  >= elf.sections[1].header.addr   + elf.sections[1].header.size,
         "first load segment must not end before .text end");
    assert!(segment.offset + segment.filesz >= elf.sections[1].header.offset + elf.sections[1].header.size,
         "first load segment must not end before .text end");

    assert!(segment.flags.contains(types::SegmentFlags::EXECUTABLE),
        "first load segment must be executable");

    assert!(!segment.flags.contains(types::SegmentFlags::WRITABLE),
        "first load segment must NOT be writable");
}

/*
#[test]
fn layout_just_bss() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_bss());
    elf.layout().unwrap();

    assert_eq!(elf.sections[1].name, b".bss",
       ".bss section must be at shndx 1");
    assert_eq!(elf.sections[1].header.offset, elf.sections[1].header.addr,
       ".bss section offset and address must be identical");

    assert_eq!(elf.segments.len(), 2,
        "expect exactly two segments");
    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    assert_eq!(load_segments.len(), 1,
        "expect xactly one load segment");
    let segment = load_segments.get(0).unwrap();;

    assert!(segment.vaddr  <= elf.sections[1].header.addr,
        "first load segment must start at or before .bss");
    assert!(segment.offset <= elf.sections[1].header.offset,
        "first load segment must start at or before .bss");

    assert!(segment.vaddr  + segment.memsz  >= elf.sections[1].header.addr   + elf.sections[1].header.size,
         "first load segment must not end before .bss end");

    assert!(segment.offset + segment.filesz < elf.sections[1].header.offset + elf.sections[1].header.size,
         "first load segment must not include .bss size as physical size");

    assert!(segment.filesz == segment.memsz - elf.sections[1].header.size,
         "first load segment filesz must be exactly memsz - .bss size");

    assert!(!segment.flags.contains(types::SegmentFlags::EXECUTABLE),
        "first load segment must NOT be executable");

    assert!(segment.flags.contains(types::SegmentFlags::WRITABLE),
        "first load segment must be writable");
}
*/

#[test]
fn layout_text_and_bss_1() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_text());
    elf.sections.push(fixture_section_bss());
    elf.layout().unwrap();

    for seg in &elf.segments {
        println!("{:?}", seg);
    }


    assert_eq!(elf.sections[1].name, b".text",
       ".text section must be at shndx 1");
    assert_eq!(elf.sections[2].name, b".bss",
       ".bss section must be at shndx 2");

    assert_eq!(elf.segments.len(), 3,
        "expect exactly 3 segments");
    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    assert_eq!(load_segments.len(), 2,
        "expect exactly 2 load segments");
    let segment0 = load_segments.get(0).unwrap();;
    let segment1 = load_segments.get(1).unwrap();;

    assert_eq!(segment0.vaddr,0 ,
        "first load segment must start at 0");

    assert!(segment0.offset <= elf.sections[1].header.offset,
        "first load segment must start at or before first section");

    assert!(segment1.vaddr  + segment1.memsz  >= elf.sections[2].header.addr + elf.sections[2].header.size,
         "second load segment must not end before last section");

    assert!(segment0.offset + segment0.filesz < elf.sections[2].header.offset + elf.sections[2].header.size,
         "first load segment must not include .bss size as physical size");

    assert!(segment0.flags.contains(types::SegmentFlags::EXECUTABLE),
        "first load segment must be executable");

    assert!(!segment0.flags.contains(types::SegmentFlags::WRITABLE),
        "first load segment must NOT be writable");
}

#[test]
fn layout_text_and_bss_2() {
    let mut elf = Elf::default();
    //seg 0
    elf.sections.push(Section::default());
    //seg 1
    elf.sections.push(fixture_section_bss());
    //seg 2
    elf.sections.push(fixture_section_text());
    elf.layout().unwrap();

    for seg in &elf.segments {
        println!("{:?}", seg);
    }

    for sec in &elf.sections{
        println!("{:?}", sec);
    }

    assert_eq!(elf.sections[1].name, b".bss",
       ".bss section must be at shndx 2");
    assert_eq!(elf.sections[2].name, b".text",
       ".text section must be at shndx 1");

    assert_eq!(elf.segments.len(), 4,
        "expect exactly 4 segments");
    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    assert_eq!(load_segments.len(), 3,
        "expect exactly 3 load segments");
    let segment0 = load_segments.get(0).unwrap();;
    let segment1 = load_segments.get(1).unwrap();;
    let segment2 = load_segments.get(2).unwrap();;

    assert_eq!(segment0.vaddr, 0,
        "first load segment must start at 0");

    assert_eq!(segment0.offset + segment0.filesz, elf.sections[1].header.offset,
         "first load segment must before .bss");

    assert!(segment0.vaddr  + segment0.memsz < elf.sections[2].header.addr,
         "first load segment must end before .text starts");

    assert!(segment0.filesz == segment0.memsz,
         "first load segment filesz must be exactly memsz ");

    assert_eq!(segment2.offset, segment2.offset + segment1.filesz,
         "third load segment must start exactly after second segment in file");

    assert_eq!(segment2.filesz, elf.sections[2].header.size,
         "third load segment filesz must be exactly as big as .text");

    assert_eq!(segment1.vaddr + segment1.memsz, elf.sections[1].header.addr + elf.sections[1].header.size,
         "second load segment memsz must be exactly .bss size");

    assert!(!segment0.flags.contains(types::SegmentFlags::EXECUTABLE),
        "first load segment must not be executable");

    assert!(segment1.flags.contains(types::SegmentFlags::EXECUTABLE),
        "second load segment must be executable");

    assert!(!segment0.flags.contains(types::SegmentFlags::WRITABLE),
        "first load segment must NOT be writable");
}

#[test]
fn layout_dynamic_and_interp() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_interp());
    elf.sections.push(fixture_section_text());
    elf.sections.push(fixture_section_bss());
    elf.sections.push(fixture_section_dynamic());

    elf.layout().unwrap();

    assert_eq!(elf.sections[4].name, b".dynamic",
       ".dynamic section must be at shndx 4");

    assert_eq!(elf.segments.len(), 6,
        "expect exactly 6 segments");
    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    assert_eq!(load_segments.len(), 3,
        "expect exactly 3 load segments");

    let dynamic:Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::DYNAMIC).collect();
    assert_eq!(dynamic.len(), 1,
        "expect exactly 1 dynamic segment");
    let dynamic = dynamic.get(0).unwrap();

    assert_eq!(dynamic.memsz, 16,
        "dynamic size must be 16");
}

#[test]
fn layout_align() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_text());
    elf.sections.push(fixture_section_text());
    elf.sections.push(fixture_section_text());
    elf.sections.push(fixture_section_text());
    elf.layout().unwrap();

    assert_eq!(elf.segments.len(), 2,
        "expect exactly two segments");

    assert!(elf.sections[1].header.addr % 16 == 0,
        "expect section 1 to be aligned by 16 bytes");
    assert!(elf.sections[2].header.addr % 16 == 0,
        "expect section 2 to be aligned by 16 bytes");
    assert!(elf.sections[3].header.addr % 16 == 0,
        "expect section 3 to be aligned by 16 bytes");
    assert!(elf.sections[4].header.addr % 16 == 0,
        "expect section 4 to be aligned by 16 bytes");
}


#[test]
fn layout_stable() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    // seg 0 + interp
    elf.sections.push(fixture_section_interp());
    elf.sections.push(fixture_section_rodata());
    // seg 1 + dynamic
    elf.sections.push(fixture_section_dynamic());
    elf.sections.push(fixture_section_bss());
    elf.layout().unwrap();

    for seg in &elf.segments {
        println!("{:?}", seg);
    }

    assert_eq!(elf.sections[0].header.addr, 0,
       "section 0 is always addr 0");
    assert_eq!(elf.sections[0].header.size, 0,
       "section 0 is always size 0");
    assert_eq!(elf.sections[0].header.offset , 0,
       "section 0 is always offset 0");

    assert_eq!(elf.segments.len(), 5,
        "expect exactly 5 segments");

    for i in 0..5 {
        elf.sections[i].addrlock = true;
    }

    elf.layout().unwrap();
}


#[test]
#[should_panic]
fn layout_enforce_addrlock() {
    let mut elf = Elf::default();
    elf.sections.push(Section::default());
    elf.sections.push(fixture_section_text());
    elf.layout().unwrap();
    elf.sections[1].addrlock = true;
    elf.sections.insert(1,fixture_section_text());
    elf.layout().unwrap();
}

#[test]
fn layout_many_bss() {
    let mut elf = Elf::default();

    // load 0
    elf.sections.push(Section::default());
    // load 1
    elf.sections.push(fixture_section_data());
    elf.sections.push(fixture_section_data());
    elf.sections.push(fixture_section_bss());
    elf.sections.push(fixture_section_bss());
    elf.sections.push(fixture_section_bss());
    elf.sections.push(fixture_section_bss());

    // load 2
    elf.sections.push(fixture_section_data());
    elf.sections.push(fixture_section_bss());
    elf.sections.push(fixture_section_bss());

    // load 3
    elf.sections.push(fixture_section_text());
    elf.layout().unwrap();

    for seg in &elf.segments {
        println!("{:?}", seg);
    }

    assert_eq!(elf.segments.len(), 5,
        "expect exactly 5 segments");

    let load_segments :Vec<&segment::SegmentHeader> = elf.segments.iter().filter(|x| x.phtype == types::SegmentType::LOAD).collect();
    let segment0 = load_segments.get(0).unwrap();;
    assert!(!segment0.flags.contains(types::SegmentFlags::WRITABLE),
        "first load segment must NOT be writable");
}
