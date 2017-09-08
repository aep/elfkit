Elfkit
=========

an elf read and manipulation library in pure rust (no bfd, no gnu code),
intended to be used in binary manipulation utils such as strip, objcopy, link editors, etc.

Some binutils replacements are included as example code.

```
cargo run --example readelf ./tests/samples/amd64_exe
```

![screenshot](/examples/readelf-screenshot.png?raw=true)


implementation status
---------------------

by architecture

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
