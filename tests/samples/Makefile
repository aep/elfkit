ALL: amd64_pie_asm amd64_pie_c dx_asm dx_asm_dyn px_asm px_asm_dyn plt_asm

CFLAGS=-fPIC

%.o: %.asm
	nasm -f elf64 -o $@ $^

%.o: %.c
	gcc -c -fPIC -o $@ -pie $^



simple_asm.o: simple.asm
	nasm -f elf64 -o $@ $^

amd64_pie_asm: simple_asm.o
	ld -o $@ -pie $^

amd64_pie_c:   main_c.o
	gcc -fPIC -o $@ -pie $^


dx_asm: dx_data.o dx_main.o
	ld -o $@ -pie $^

libdxdata.so: dx_data.o
	gcc -fPIC $^ -shared -o $@
dx_asm_dyn: libdxdata.so dx_main.o
	ld -o $@ -pie $^


px_asm: px_main.o px_hello.o dx_data.o
	ld -o $@ -pie $^

libpxhello.so: px_hello.o
	gcc -fPIC $^ -shared -o $@

px_asm_dyn: libdxdata.so libpxhello.so px_main.o
	ld -o $@ -pie $^ || true

plt_asm: plt_main.o px_hello.o dx_data.o
	ld -o $@ -pie $^
