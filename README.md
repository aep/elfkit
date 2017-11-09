[![Build Status](https://travis-ci.org/aep/elfkit.svg?branch=master)](https://travis-ci.org/aep/elfkit)
[![crates.io](http://meritbadge.herokuapp.com/elfkit)](https://crates.io/crates/elfkit)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![docs](https://docs.rs/elfkit/badge.svg)](https://docs.rs/elfkit)

Elfkit
=========

an elf read and manipulation library in pure rust (written from scratch, no bfd, no gnu code, no license infections),
intended to be used in binary manipulation utils such as strip, chrpath, objcopy and ld.
The end goal is to build a well designed library that facilitates drop-in replacements for gnu ld.

currently elfkit's ld can only link asm and C code with musl libc

```
cargo build --release --bin ld
ln -s $PWD/target/release/ld /usr/local/bin/ld.gold
#ensure /usr/local/bin/ld.gold shows up in PATH first, otherwise use a different directory
which ld.gold
musl-gcc -fuse-ld=gold main.c
```

there's also a prettier version of readelf showing of parsing capabilities

```
cargo run --example readelf ./tests/samples/amd64_exe
```

![screenshot](/bin/readelf-screenshot.png?raw=true)


modular linker toolkit
---------------------

Loader: loads elf objects from disk
Linker: produces a link graph of sections from a loader
Layout: bakes multiple sections into a single object





implementation status
---------------------

section specific parsers

| type         | read    | write   |
|--------------|---------|---------|
| symbols      | ok      | ok      |
| strtab       | ok      | ok      |
| relocations  | ok      | ok      |
| dynamic      | ok      | ok      |
| note         | -       | -       |
| gnu_hash     | -       | -       |
| hash         | -       | faked   |
| versym       | -       | -       |
| verneed      | -       | -       |

architectures

| abi          | headers | relocations    |
|--------------|---------|----------------|
| x86_64       | ok      | minimum viable |
| mips32r2 o32 | ok      |                |
| arm eabi     | ok      |                |


alternatives
----------------

- [goblin](https://crates.io/crates/goblin) mach-o and archive support, no-std support, very low level
- [elf](https://crates.io/crates/elf) most popular, most generic use case, no writing, no section parsing
- [xmas-elf](https://github.com/nrc/xmas-elf) zero alloc (good for writing an OS), read only


references
---------------------
- https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
- https://github.com/hjl-tools/x86-psABI/wiki/x86-64-psABI-r252.pdf
- https://software.intel.com/sites/default/files/article/402129/mpx-linux64-abi.pdf
- http://infocenter.arm.com/help/topic/com.arm.doc.ihi0044f/IHI0044F_aaelf.pdf
- https://dmz-portal.imgtec.com/wiki/MIPS_ABI_Project
- https://dmz-portal.imgtec.com/wiki/MIPS_O32_ABI_-_FR0_and_FR1_Interlinking
- http://www.mindfruit.co.uk/2012/06/relocations-relocations.html#reloc_types_table
