notepad for useful learnings
===========================


segment headers must come before sections
-----------------------------------------

A 'bug' in linux causes it to pass an incorrect pointer to PHDR to ld via auxv,
if there are holes between the program header and the base load address.

As a result, the segment headers must be written before the section content, so
that the first LOAD segment contains both.

This prevents usecases which would require to rewrite the program header of existing binaries,
since you can't create additional room for more program headers by moving them to the end of the file.
A commonly applied hack seems to be to move some sections from the front to the back instead.


DT_RELA
------------

DT_RELA points into .rela.dyn for relocations to be applied by the dynloader (the thin in .interp)
DT_RELACOUNT counts ONLY R_X86_64_RELATIVE which are executed before loading libraries
while DT_RELASZ is the size of the full section, potentially containing more reloc types

according to the glibc code (untested) these additional relocations should work:
  - R_X86_64_SIZE64
  - R_X86_64_SIZE32
  - R_X86_64_GLOB_DAT
  - R_X86_64_DTPMOD64
  - R_X86_64_DTPOFF64
  - R_X86_64_TLSDESC
  - R_X86_64_TPOFF64
  - R_X86_64_64
  - R_X86_64_SIZE32
  - R_X86_64_32
  - R_X86_64_PC32
  - R_X86_64_COPY
  - R_X86_64_IRELATIVE


there's also DT_TEXTREL, which may contain even more relocation types. it seems to be a count offset into DT_RELA

X86_64 RIP Relative Relocations
-------------------------------

so on x86_64 there's a thing called RIP. i'm not sure if that's an actual register,
it sort of is a pointer to the next instruction, but RELATIVE to the program load address.
so if program loads at 0xwhatever, it'll still be 0x1 for the first instruction executed.
I still have no idea how that even works, because how does the cpu know where the thing was loaded?
anyway..

Together with X86_64_GOTPCREL the compiler emits a LEA instruction with an offset from RIP, so for example:

```
    0x0: "hello"
    0x5: 48 8d 35 06 00 00 00   lea -0xb(%rip),%rsi
    0xb: ...
```

which means "add -0xb to the position of the next instruction and store that POSITION in rsi"
that's different from mov, which would store the value at that position in rsi

in this case X86_64_GOTPCREL has
 - offset=0x9 (the address offset part of lea)
 - addend=-0x4 (i am currently assuming this is the start of the instruction from offset)
 - symbol=something pointing at 0x0 hello

if the linker knows the address of hello, it can simply write that at reloc.offset.
Otherwise, it's supposed to

 - change the instruction from lea to mov
 - emit a Global Offset Table (.got) section with 8 bytes zeros
 - write the address to that into reloc.offset
 - emit something like X86_64_GLOB_DAT which will at runtime copy the address of hello to .got

so the linked executable will look like:

```
    0x0: 00 00 00 00
    0x5: 48 8d 35 06 00 00 00   mov -0xb(%rip),%rsi
    0xb: ...
```

when loading, the dynloader then puts the address of hello in there

```
    0x0: 00 00 00 0f
    0x5: 48 8d 35 06 00 00 00   mov -0xb(%rip),%rsi
    0xb: ...
    0xf: "hello"
```

the mov instruction will load (unlike lea) the value from 0x0, apply the rip offset to get an absolute address,
and store it in rsi

if this sounds rather complicated, it's because it really is. i still haven't figured out why this is done.
until i understand that, elfkit just emits X86_64_PC32, which will instruct the dynlinker to write the address
of hello directly into the lea.


useful debugging help
------------------------

make ld show the aux vector:

    LD_SHOW_AUXV=1 /bin/something


show memory map in gdb:

    info proc mappings
