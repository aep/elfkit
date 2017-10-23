section .data

    global msg1:data (msg1.end - msg1)
    msg1 db      "hello, world!"
    .end:

    global msg2:data (msg2.end - msg2)
    msg2 db      "The quick brown fox jumps over the lazy dog"
    .end:
