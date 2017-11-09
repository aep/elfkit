extern msg2;

section .text
    global _start
_start:
    mov     rax, 1
    mov     rdi, 1
    mov     rsi, msg2
    mov     rdx, 43
    syscall
    mov    rax, 60
    mov    rdi, 10
    syscall

    global _useless:
_useless:
    nop
