extern crate elfkit;
#[macro_use] extern crate itertools;
use itertools::Itertools;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf, types, SegmentHeader, Section, SectionContent, SectionHeader, Dynamic, Symbol};
use elfkit::dynamic::DynamicContent;

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();
    in_elf.load_all().unwrap();

    let mut out_elf = Elf::default();
    out_elf.header.ident_class  = in_elf.header.ident_class;
    out_elf.header.ident_abi    = in_elf.header.ident_abi;
    out_elf.header.etype        = types::ElfType::EXEC;
    out_elf.header.machine      = in_elf.header.machine;

    let mut sc_interp  : Vec<u8> = b"/lib64/ld-linux-x86-64.so.2\0".to_vec();
    //let mut sc_interp  : Vec<u8> = b"/usr/local/musl/lib/libc.so\0".to_vec();
    let mut sc_text    : Vec<u8> = Vec::new();
    let mut sc_dynsym  : Vec<Symbol>  = Vec::new();
    let mut sc_dynamic : Vec<Dynamic> = vec![
        Dynamic{
            dhtype: types::DynamicType::FLAGS_1,
            content: DynamicContent::Flags1(types::DynamicFlags1::PIE),
        },
        Dynamic{
            dhtype:  types::DynamicType::STRTAB,
            content: DynamicContent::Address(0),
        },
        Dynamic{
            dhtype:  types::DynamicType::SYMTAB,
            content: DynamicContent::Address(0),
        },
        Dynamic{
            dhtype:  types::DynamicType::STRSZ,
            content: DynamicContent::Address(0),
        },
        Dynamic{
            dhtype:  types::DynamicType::SYMENT,
            content: DynamicContent::Address(Symbol::entsize(&out_elf.header) as u64),
        },
        Dynamic{
            dhtype:  types::DynamicType::NULL,
            content: DynamicContent::Address(0),
        },
    ];

    for mut sec in in_elf.sections {
        if sec.header.shtype == types::SectionType::PROGBITS && sec.name == ".text" {
            match sec.content {
                SectionContent::Raw(v) => sc_text.extend(v),
                _ => unreachable!(),
            }
        }
    }

    out_elf.sections.insert(0, Section::default());
    out_elf.sections.push(Section{
        name: String::from(".interp"),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC,
            addr:       0,
            offset:     0,
            size:       0,
            link:       0,
            info:       0,
            addralign:  0,
            entsize:    0,
        },
        content: SectionContent::Raw(sc_interp),
    });
    out_elf.sections.push(Section{
        name: String::from(".text"),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::PROGBITS,
            flags:      types::SectionFlags::ALLOC | types::SectionFlags::EXECINSTR,
            addr:       0,
            offset:     0,
            size:       0,
            link:       0,
            info:       0,
            addralign:  0,
            entsize:    0,
        },
        content: SectionContent::Raw(sc_text),
    });

    let sh_dynstr_index = out_elf.sections.len();
    out_elf.sections.push(Section{
        name: String::from(".dynstr"),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::STRTAB,
            flags:      types::SectionFlags::ALLOC,
            addr:       0,
            offset:     0,
            size:       0,
            link:       0,
            info:       0,
            addralign:  0,
            entsize:    0,
        },
        content: SectionContent::Raw(Vec::new()),
    });
    out_elf.sections.push(Section{
        name: String::from(".dynsym"),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::SYMTAB,
            flags:      types::SectionFlags::ALLOC,
            addr:       0,
            offset:     0,
            size:       0,
            link:       sh_dynstr_index as u32,
            info:       0,
            addralign:  0,
            entsize:    0,
        },
        content: SectionContent::Symbols(sc_dynsym),
    });
    out_elf.sections.push(Section{
        name: String::from(".dynamic"),
        header: SectionHeader {
            name:       0,
            shtype:     types::SectionType::DYNAMIC,
            flags:      types::SectionFlags::ALLOC | types::SectionFlags::WRITE,
            addr:       0,
            offset:     0,
            size:       0,
            link:       sh_dynstr_index as u32,
            info:       0,
            addralign:  0,
            entsize:    0,
        },
        content: SectionContent::Dynamic(sc_dynamic),
    });

    //for some obscure reason we cannot LOAD to position 0.
    //gdb says: /tmp/e: failed to map segment from shared object
    let vstart = 0x10000;

    out_elf.store_all();
    out_elf.relayout(0x300, vstart + 0x300);


    // post layout calculations

    for sec  in &out_elf.sections {
        match sec.name.as_ref() {
            ".text" => {
                out_elf.header.entry = sec.header.addr;
            },
            ".interp" => out_elf.segments.push(SegmentHeader{
                phtype: types::SegmentType::INTERP,
                flags:  types::SegmentFlags::READABLE | types::SegmentFlags::EXECUTABLE,
                offset: sec.header.offset,
                filesz: sec.header.size,
                vaddr:  sec.header.addr,
                paddr:  sec.header.addr,
                memsz:  sec.header.size,
                align:  0x1,
            }),
            ".dynamic" => out_elf.segments.push(SegmentHeader{
                phtype: types::SegmentType::DYNAMIC,
                flags:  types::SegmentFlags::READABLE | types::SegmentFlags::WRITABLE,
                offset: sec.header.offset,
                filesz: sec.header.size,
                vaddr:  sec.header.addr,
                paddr:  sec.header.addr,
                memsz:  sec.header.size,
                align:  0x1,
            }),
            _ => {},
        }
    }

    //TODO ld does some pretty weird shit that's pretty hard to figure out.
    //if we don't emulate ld and just read the elf spec, we'll probably run into more kernel bugs
    //just loading the entire file for now...

    let mut total_vsize = 0x300;
    let mut total_psize = 0x300;
    for sec in &out_elf.sections {
        total_vsize += sec.header.size;
        total_psize += sec.size() as u64;
    }
    out_elf.segments.push(SegmentHeader{
        phtype: types::SegmentType::LOAD,
        flags:  types::SegmentFlags::READABLE | types::SegmentFlags::WRITABLE | types::SegmentFlags::EXECUTABLE,

        offset: 0,
        filesz: total_psize,

        vaddr:  vstart,
        paddr:  vstart,
        memsz:  total_vsize,

        align:  0x10000,
    });


    let phdrsize = (out_elf.segments.len() + 1) * SegmentHeader::entsize(&out_elf.header);
    out_elf.segments.insert(0, SegmentHeader{
        phtype: types::SegmentType::PHDR,
        flags:  types::SegmentFlags::READABLE | types::SegmentFlags::EXECUTABLE,

        offset: out_elf.header.size() as u64,
        filesz: phdrsize as u64,

        vaddr:  vstart + out_elf.header.size() as u64,
        paddr:  vstart + out_elf.header.size() as u64,
        memsz:  phdrsize as u64,
        align:  0x8,
    });



    /*
       for (flags, sections) in &out_elf.sections.iter().group_by(|s| s.header.flags) {
       let mut seg_psize  = 0;
       let mut seg_vsize  = 0;

       let mut seg_pstart = 0;
       let mut seg_vstart = 0;

       for (i,sec) in sections.enumerate() {
       if seg_pstart == 0 {
       seg_pstart = sec.header.offset;
       seg_vstart = sec.header.addr;
       }
       seg_psize += sec.size() as u64;
       seg_vsize += sec.header.size;
       }

       let mut segflags = types::SegmentFlags::READABLE;
       if !flags.contains(types::SectionFlags::ALLOC) {
       continue;
       }
       if flags.contains(types::SectionFlags::WRITE) {
       segflags.set(types::SegmentFlags::WRITABLE, true);
       }
       if flags.contains(types::SectionFlags::EXECINSTR) {
       segflags.set(types::SegmentFlags::EXECUTABLE, true);
       }

       let mut seg = SegmentHeader{
       phtype: types::SegmentType::LOAD,
       flags: segflags,

       offset: seg_pstart,
       filesz: seg_psize,

       vaddr:  seg_vstart,
       paddr:  seg_vstart,
       memsz:  seg_vsize,

       align:  0x200000,
       };

// a funny little extra oddity, the first LOAD must contain the elf and program header
if out_elf.segments.is_empty() {
seg.filesz += seg.offset;
seg.memsz  += seg.offset;
seg.vaddr  -= seg.offset;
seg.paddr  -= seg.offset;
seg.offset  = 0;
}

out_elf.segments.push(seg);

seg_pstart += seg_psize;
seg_vstart += seg_vsize;
}
*/


out_elf.to_writer(&mut out_file).unwrap();
}

