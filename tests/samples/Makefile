ALL: amd64_exe_asm amd64_exe_asm_pie

main_asm.o: main.asm
	nasm -f elf64 -o $@ $<

amd64_exe_asm: main_asm.o
	ld -o $@  $<

amd64_exe_asm_pie: main_asm.o
	ld -o $@ -pie $<
