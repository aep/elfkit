[![Build Status](https://travis-ci.org/aep/elfkit.svg?branch=master)](https://travis-ci.org/aep/elfkit)
[![codecov](https://codecov.io/gh/aep/elfkit/branch/master/graph/badge.svg)](https://codecov.io/gh/aep/elfkit)
[![crates.io](http://meritbadge.herokuapp.com/elfkit)](https://crates.io/crates/elfkit)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![docs](https://docs.rs/elfkit/badge.svg)](https://docs.rs/elfkit)

Elfkit
=========

an elf read and manipulation library in pure rust (no bfd, no gnu code),
intended to be used in binary manipulation utils such as strip, objcopy, link editors, etc.

Some binutils replacements are included as example code.

```
cargo run --example readelf ./tests/samples/amd64_exe
```

![screenshot](/examples/readelf-screenshot.png?raw=true)


api design
---------------------

*low level*

Every type implements from_reader and to_writer. You can use them invdividually,
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




references
---------------------
- https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
- https://software.intel.com/sites/default/files/article/402129/mpx-linux64-abi.pdf
- http://infocenter.arm.com/help/topic/com.arm.doc.ihi0044f/IHI0044F_aaelf.pdf
- https://dmz-portal.imgtec.com/wiki/MIPS_ABI_Project
- https://dmz-portal.imgtec.com/wiki/MIPS_O32_ABI_-_FR0_and_FR1_Interlinking
