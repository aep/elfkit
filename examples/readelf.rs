extern crate elfkit;
extern crate colored;

use std::env;
use std::fs::File;
use elfkit::{Elf, SectionContent, DynamicContent, types};
use std::io::{Read, Seek, SeekFrom};
use colored::*;

fn hextab<S>(align: usize, s:S) -> String where S: std::fmt::LowerHex {
    let s   = format!("{:.align$x}", s, align=align);
    let pad : String = vec!['0'; align - s.len()].into_iter().collect();
    format!("\x1b[90m{}\x1b[0;m{}", pad, s)
}

fn main() {
    let filename = env::args().nth(1).unwrap();
    let mut file = File::open(filename).unwrap();
    let mut elf  = Elf::from_reader(&mut file).unwrap();
    elf.load_all().unwrap();

    println!("{}", "ELF Header:".bold());
    println!("  Magic:                             {:?}", elf.header.ident_magic);
    println!("  Class:                             {:?}", elf.header.ident_class);
    println!("  Data:                              {:?}", elf.header.ident_endianness);
    println!("  Version:                           {}", elf.header.ident_version);
    println!("  OS/ABI:                            {:?}", elf.header.ident_abi);
    println!("  ABI Version:                       {:?}", elf.header.ident_abiversion);
    println!("  Type:                              {:?}", elf.header.etype);
    println!("  Machine:                           {:?}", elf.header.machine);
    println!("  Version:                           {:?}", elf.header.version);
    println!("  Entry point address:               0x{:x}", elf.header.entry);
    println!("  Start of program headers:          {} (bytes into file)", elf.header.phoff);
    println!("  Start of section headers:          {} (bytes into file)", elf.header.shoff);
    println!("  Flags:                             0x{:x} {:?}", elf.header.flags, elf.header.flags);
    println!("  Size of this header:               {} (bytes)", elf.header.ehsize);
    println!("  Size of program headers:           {} (bytes)", elf.header.phentsize);
    println!("  Number of program headers:         {}", elf.header.phnum);
    println!("  Size of section headers:           {} (bytes)", elf.header.shentsize);
    println!("  Number of section headers:         {}", elf.header.shnum);
    println!("  Section header string table index: {}", elf.header.shstrndx);
    println!("");
    println!("{} at offset 0x{:x}:", "Section Headers".bold(), elf.header.shoff);
    println!("  [Nr] Name             Type           Address          Offset   Size     EntS Flg Lnk Inf Al");

    for (i, section) in elf.sections.iter().enumerate() {
        println!("  [{:>2}] {:<16.16} {} {} {} {} {} {:<3} {:<3.3} {:<3} {:<2.2}",
                 i, section.name.bold(),
                 match section.header.shtype.typename(&elf.header) {
                     Some(s) => format!("{:<14.14}", s),
                     None    => hextab(14, section.header.shtype.to_u32()),
                 },
                 hextab(16, section.header.addr), hextab(8, section.header.offset),
                 hextab(8, section.header.size), hextab(4, section.header.entsize),
                 section.header.flags, section.header.link, section.header.info, section.header.addralign
                );
    }

    println!("",);
    println!("{}", "Legend for  Flags:".bold());
    println!("  {}: write, {}: alloc, {}: execute, {}: merge, {}: strings, {}: info,
  {}: link order, {}: extra OS processing required, {}: group, {}: TLS,
  {}: compressed, {}: OS specific, {}: exclude, {}: large,
  {}: mips: global data area",
  "W".bold(), "A".bold(), "X".bold(), "M".bold(), "S".bold(), "I".bold(),
  "L".bold(), "O".bold(), "G".bold(), "T".bold(),
  "C".bold(), "o".bold(), "E".bold(), "l".bold(), "g".bold());

    if elf.segments.len() > 0 {
        println!("");
        println!("{} at offset 0x{:x}:", "Program Headers (Segments)".bold(), elf.header.phoff);
        println!("  Type           Offset             VirtAddr           PhysAddr");
        println!("                 FileSiz            MemSiz             Flags  Align");

        for ph in &elf.segments {
            println!("  {:<14.14} 0x{} 0x{} 0x{}",
                     format!("{:?}", ph.phtype), hextab(16, ph.offset), hextab(16, ph.vaddr), hextab(16, ph.paddr));

            println!("                 0x{} 0x{} {:<6} 0x{:x}",
                     hextab(16, ph.filesz), hextab(16, ph.memsz), ph.flags,  ph.align);
        }
    }

    println!("");
    println!("{}:", "File Layout".bold());

    let mut fls = vec![
        ("elf header",      0, elf.header.ehsize as u64),
        ("section headers", elf.header.shoff, (elf.header.shentsize * elf.header.shnum) as u64),
    ];
    if elf.header.phoff > 0 {
        fls.push(("segment headers", elf.header.phoff, (elf.header.phentsize * elf.header.phnum) as u64));
    }

    for section in elf.sections.iter() {
        if section.header.size < 1 {
            continue;
        }
        fls.push((&section.name, section.header.offset,
                  if section.header.shtype == types::SectionType::NOBITS {0} else {section.header.size}
                  ));
    }

    fls.sort_by(|&(_,a,_),&(_,b,_)| a.cmp(&b));

    println!("{}", "                     offset     size     segment".bold());
    if elf.segments.len() > 0 {
        for n in 0..12 {
            let n = n;
            print!(        "                                         ");
            for segment in elf.segments.iter() {
                let name = format!("{:?}", segment.phtype);
                print!(" {} |", if name.len() > n {&name[n..n+1]} else {" "});
            }
            println!("");
        }
        print!("                                         ");
        for segment in elf.segments.iter() {
            print!("---|")
        }
        println!("");
    }



    let mut cfileoff = 0;
    let mut fls_intermediate = fls.drain(..).collect::<Vec<(&str,u64,u64)>>();
    for (name, off, size) in fls_intermediate {
        if cfileoff < off {
            fls.push(("", cfileoff, off - cfileoff));
        }
        fls.push((name, off, size));
        cfileoff = off + size;
    }

    if let Some(&(name, off, size)) = fls.last() {
        let filelen = file.metadata().unwrap().len();
        if off + size < filelen {
            fls.push(("", off + size, filelen - (off + size)));
        }
    }


    for (name, off, size) in fls {

        print!("  {}   0x{:<6.6x}   0x{:<6.6x} ", format!("{:<16.16}", name).bold(), off, size);

        for segment in elf.segments.iter() {
            if off >= segment.offset && (off + size) <= (segment.offset + segment.filesz) {
                if segment.flags.contains(types::SegmentFlags::EXECUTABLE) {
                    print!("{:^3}|", format!("{}",segment.flags).green())
                } else if segment.flags.contains(types::SegmentFlags::WRITABLE) {
                    print!("{:^3}|", format!("{}",segment.flags).red())
                } else {
                    print!("{:^3}|", format!("{}",segment.flags));
                }
            } else {
                print!("   |");
            }
        }
        println!("");
    }



    for section in &elf.sections {
        match section.content {
            SectionContent::Relocations(ref relocs)=> {
                println!("");
                println!("{} relocation section at offset 0x{:x}:",
                         section.name.bold(), section.header.offset);
                println!("  Offset           Type            Symbol           Addend");

                for reloc in relocs {
                    println!("  {} {:<15.15} {: <16.16x} {: <16.16x}",
                             hextab(16, reloc.addr), &format!("{:?}", reloc.rtype)[2..], reloc.sym, reloc.addend);
                }
            },
            SectionContent::Symbols(ref symbols) => {
                println!("");
                println!("{} symbols section at offset 0x{:x}:",
                         section.name.bold(), section.header.offset);
                println!("  Num: Value             Size Type    Bind   Vis      Ndx Name");

                for (i, symbol) in symbols.iter().enumerate() {
                    println!("  {:>3}: {} {:>5.5} {:<7.7} {:<6.6} {:<8.8} {:<3.3} {} ", i,
                             hextab(16, symbol.value), symbol.size,
                             format!("{:?}", symbol.stype),
                             format!("{:?}", symbol.bind),
                             format!("{:?}", symbol.vis),
                             match symbol.shndx {
                                 0     => String::from("UND"),
                                 65521 => String::from("ABS"),
                                 v => format!("{}", v),
                             },
                             symbol.name);
                }
            },
            SectionContent::Dynamic(ref dynamics) => {
                println!("");
                println!("{} dynamic linker section at offset 0x{:x}:",
                         section.name.bold(), section.header.offset);
                println!("  Tag          Value");

                for dyn in dynamics {
                    println!("  {:<12} {}", format!("{:?}", dyn.dhtype),
                    match dyn.content {
                        DynamicContent::None  => String::default(),
                        DynamicContent::String(ref s)  => s.clone(),
                        DynamicContent::Address(u) => hextab(16, u),
                        DynamicContent::Flags1(v)  => format!("{:?}", v),
                    });

                }
            },
            SectionContent::Raw(ref s) => {
                match section.name.as_ref() {
                    ".interp" => {
                        println!("");
                        println!("{} program interpreter section at offset 0x{:x}:",
                                 section.name.bold(), section.header.offset);
                        println!("  {}",String::from_utf8_lossy(s));
                    },
                    _ => {}
                }
            }
            _ => {
            }
        }
    }
}

