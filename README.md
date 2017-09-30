[![Build Status](https://travis-ci.org/aep/elfkit.svg?branch=master)](https://travis-ci.org/aep/elfkit)
[![codecov](https://codecov.io/gh/aep/elfkit/branch/master/graph/badge.svg)](https://codecov.io/gh/aep/elfkit)
[![crates.io](http://meritbadge.herokuapp.com/elfkit)](https://crates.io/crates/elfkit)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![docs](https://docs.rs/elfkit/badge.svg)](https://docs.rs/elfkit)

Elfkit
=========

an elf read and manipulation library in pure rust (no bfd, no gnu code),
intended to be used in binary manipulation utils such as strip, objcopy, linkers.
The end goal is to build a drop-in replacement for gnu ld.

Some gnu binutils replacements are included as example code.

__warning: the high level api is a moving target. do not start using load/store yet__

```
cargo run --example readelf ./tests/samples/amd64_exe
```

![screenshot](/examples/readelf-screenshot.png?raw=true)

strip can be implemented like:

```rust
extern crate elfkit;

use std::env;
use std::fs::OpenOptions;
use elfkit::{Elf,types};

fn main() {
    let in_filename  = env::args().nth(1).unwrap();
    let out_filename = env::args().nth(2).unwrap();
    let mut in_file  = OpenOptions::new().read(true).open(in_filename).unwrap();
    let mut out_file = OpenOptions::new().write(true).truncate(true).create(true).open(out_filename).unwrap();

    let mut in_elf  = Elf::from_reader(&mut in_file).unwrap();

    let mut out_elf = Elf::default();

    out_elf.header.ident_class  = in_elf.header.ident_class;
    out_elf.header.ident_abi    = in_elf.header.ident_abi;
    out_elf.header.etype        = in_elf.header.etype;
    out_elf.header.machine      = in_elf.header.machine;
    out_elf.header.entry        = in_elf.header.entry;

    out_elf.segments = in_elf.segments.clone();

    // sections which do not have an ALLOC flag aren't needed by the dynamic linker
    // but also keep the first NULL section for padding
    out_elf.sections = in_elf.sections.drain(..).filter(|sec|{
        sec.header.flags.contains(types::SectionFlags::ALLOC) ||
        sec.header.shtype == types::SectionType::NULL
    }).collect();

    out_elf.to_writer(&mut out_file).unwrap();
}

```

api design
---------------------

*lower level*

everything is based on std::io::{Read,Write}. This is most convenient for userspace editors.
For writing a kernel check alternatives below.

Every type implements from_reader and to_writer. You can use them individually,
but you'll always need a Header to tell the de/serializers about things like endianness, bitwidth,..

*structured elf*

the most versatile api is propably Elf::from_reader/to_writer.
You can use it as is, which will hold all sectionc content in Vec<u8> or call Elf::load_all() which will parse
the sections into their detailed specific structures, such as symbols, relocations, dynamic linker instructions, etc..


implementation status
---------------------

section specific parsers

| type         | read    | write   |
|--------------|---------|---------|
| symtab       | ok      | ok      |
| rela         | ok      | ok      |
| dynamic      | ok      | ok      |
| rel          | -       | -       |
| note         | -       | -       |
| gnu_hash     | -       | -       |
| versym       | -       | -       |
| verneed      | -       | -       |

architectures

| abi          | headers | relocations | 
|--------------|---------|-------------|
| x86_64       | ok      | wip         |
| mips32r2 o32 | ok      |             |
| arm eabi     | ok      |             |


alternatives
----------------

- [goblin](https://crates.io/crates/goblin) mach-o and archive support, no-std support, very low level
- [elf](https://crates.io/crates/elf) most popular, most generic use case, no writing, no section parsing
- [xmas-elf](https://github.com/nrc/xmas-elf) zero alloc (good for writing an OS), read only


references
---------------------
- https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
- https://software.intel.com/sites/default/files/article/402129/mpx-linux64-abi.pdf
- http://infocenter.arm.com/help/topic/com.arm.doc.ihi0044f/IHI0044F_aaelf.pdf
- https://dmz-portal.imgtec.com/wiki/MIPS_ABI_Project
- https://dmz-portal.imgtec.com/wiki/MIPS_O32_ABI_-_FR0_and_FR1_Interlinking
