ALL: amd64_pie_asm amd64_pie_c

main_asm.o: main.asm
	nasm -f elf64 -o $@ $<

amd64_pie_asm: main_asm.o
	ld -o $@ -pie $<

amd64_pie_c:   main_c.o
	gcc  -o $@ -pie $<
