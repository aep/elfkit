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


useful debugging help
------------------------

make ld show the aux vector:

    LD_SHOW_AUXV=1 /bin/something


show memory map in gdb:

    info proc mappings
