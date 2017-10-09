extern msg2;

section .text
    global hello
hello:
    mov     rax, 1
    mov     rdi, 1
    mov     rsi, msg2
    mov     rdx, 43
    syscall
    ret

