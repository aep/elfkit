common msg2 43;
extern init;

section .text
    global _start
_start:
    call    init
    mov     rax, 1
    mov     rdi, 1
    mov     rsi, msg2
    mov     rdx, 43
    syscall
    mov    rax, 60
    mov    rdi, 10
    syscall
