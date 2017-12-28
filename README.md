[![Build Status](https://travis-ci.org/aep/elfkit.svg?branch=master)](https://travis-ci.org/aep/elfkit)
[![crates.io](http://meritbadge.herokuapp.com/elfkit)](https://crates.io/crates/elfkit)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![docs](https://docs.rs/elfkit/badge.svg)](https://docs.rs/elfkit)

Elfkit
=========

an elf read and manipulation library in pure rust (written from scratch, no bfd, no gnu code, no license infections),
intended to be used in binary manipulation utils such as strip, chrpath, objcopy and ld.
The end goal is to build a well designed library that facilitates all sorts of binary manipulation magic.

elfkit can now link elfkit, so it's reasonably complete for x86_64. But it's definitely not stable yet and might produce incorrect code.


Using the linker
---------------------

The quickest way to use elfkit with rust is with [korhal/stasis](https://github.com/korhalio/stasis).

You can also either build from source or download binaries.
Gcc does not have an option to use a foreign linker, so we need to pretend we're ld.gold, like so:

```
curl -L https://github.com/aep/elfkit/releases/download/0.0.4/elfkit-0.0.4.tar.xz | tar xvjf -
export PATH="$PWD/elfkit-0.0.4/:$PATH"
musl-gcc -fuse-ld=gold main.c
```

for using elfkit for compiling rust code, add the following to ~/.cargo/config:

```
[target.x86_64-unknown-linux-musl]
rustflags = [
    "-C", "link-arg=-fuse-ld=gold",
    "-C", "link-arg=-Wl,-dynamic-linker,/usr/local/musl/lib/libc.so",
]
```

when compiling from source, create the ld.gold symlink manually.
```
cargo build --release --bin ld
ln -s $PWD/target/release/ld /usr/local/bin/ld.gold
```


other binutils
---------------------

readelf:
![screenshot](/bin/readelf-screenshot.png?raw=true)


implementation status
---------------------

binutils

| type         | status    | gnu compatible |
|--------------|-----------|----------------|
| ldd          | done      | no             |
| readelf      | done      | no             |
| ld           | wip       | wip            |
| objdump      | -         | -              |
| ar           | -         | -              |
| as           | -         | -              |
| nm           | -         | -              |
| strip        | -         | -              |

section parsers

| type         | read    | write   |
|--------------|---------|---------|
| symbols      | done    | done    |
| strtab       | done    | done    |
| relocations  | done    | done    |
| dynamic      | done    | done    |
| note         | -       | -       |
| gnu_hash     | -       | -       |
| hash         | -       | mvp     |
| versym       | -       | -       |
| verneed      | -       | -       |

architectures

| abi          | parser  | linker |
|--------------|---------|--------|
| x86_64       | done    | wip    |
| mips32r2 o32 | done    |        |
| arm eabi     | done    |        |


modular linker toolkit
---------------------

- Loader:       loads elf objects from disk
- Linker:       produces a link graph of sections from a loader
- Collector:    bakes multiple sections into a single object
- Relocator:    applies relocations to a combined object

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
- https://www.akkadia.org/drepper/tls.pdf
