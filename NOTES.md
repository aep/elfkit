notepad for useful learnings


DT_RELA
--------

DT_RELA points into .rela.dyn
DT_RELACOUNT counts ONLY R_X86_64_RELATIVE which are executed before loading libraries
while DT_RELASZ is the size of the full section

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


dynamic linker crashing
------------------------

program headers should apparantly come before section content?
see musl libc ./ldso/dlstart.c:45


make ld show the aux vector:
    LD_SHOW_AUXV=1 /bin/something



show memory map in gdb:
info proc mappings
