DEFAULT REL;
common msg2 43;

section .data
    msg1 db      "The quick brown fox jumps over the lazy dog"


section .text
    global init:func
init:
    mov     rdi, [msg1]
    mov     [msg2], rdi
    mov     rdi, [msg1+8]
    mov     [msg2+8], rdi
    mov     rdi, [msg1+16]
    mov     [msg2+16], rdi
    mov     rdi, [msg1+24]
    mov     [msg2+24], rdi
    mov     rdi, [msg1+32]
    mov     [msg2+32], rdi
    mov     rdi, [msg1+40]
    mov     [msg2+40], rdi
    ret
