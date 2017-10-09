ALL: amd64_pie_asm amd64_pie_c dx_asm dx_asm_dyn px_asm px_asm_dyn

CFLAGS=-fPIC


simple_asm.o: simple.asm
	nasm -f elf64 -o $@ $^

amd64_pie_asm: simple_asm.o
	ld -o $@ -pie $^

amd64_pie_c:   main_c.o
	gcc -fPIC -o $@ -pie $^



dx_data.o: dx_data.asm
	nasm -f elf64 -o $@ $^

dx_main.o: dx_main.asm
	nasm -f elf64 -o $@ $^

dx_asm: dx_data.o dx_main.o
	ld -o $@ -pie $^


libdxdata.so: dx_data.o
	gcc -fPIC $^ -shared -o $@
dx_asm_dyn: libdxdata.so dx_main.o
	ld -o $@ -pie $^



px_main.o: px_main.asm
	nasm -f elf64 -o $@ $^

px_hello.o: px_hello.asm
	nasm -f elf64 -o $@ $^

px_asm: px_main.o px_hello.o dx_data.o
	ld -o $@ -pie $^

libpxhello.so: px_hello.o
	gcc -fPIC $^ -shared -o $@

px_asm_dyn: libdxdata.so libpxhello.so px_main.o
	ld -o $@ -pie $^ || true
