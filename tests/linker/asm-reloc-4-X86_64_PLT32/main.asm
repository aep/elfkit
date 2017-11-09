extern hello;

section .text
    global _start
_start:
    push    rbp
    call    hello WRT ..plt
    pop	    rbp
    mov     rax, 60
    mov     rdi, 10
    syscall
    global _useless:

_useless:
    nop
