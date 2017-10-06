ALL: amd64_pie_asm amd64_pie_c

CFLAGS=-fPIC

simple_asm.o: simple.asm
	nasm -f elf64 -o $@ $<

amd64_pie_asm: simple_asm.o
	ld -o $@ -pie $<

amd64_pie_c:   main_c.o
	gcc -fPIC -o $@ -pie $<
