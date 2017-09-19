notepad for useful learnings



dynamic linker crashing
------------------------

program headers should apparantly come before section content?
see musl libc ./ldso/dlstart.c:45



make ld show the aux vector:
    LD_SHOW_AUXV=1 /bin/something


