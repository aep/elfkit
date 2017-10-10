ALL: amd64_pie_asm amd64_pie_c dx_asm dx_asm_dyn px_asm px_asm_dyn plt_asm plt_asm_dyn got_asm got_asm_dyn

CFLAGS=-fPIC
LDFLAGS=--dynamic-linker=/usr/lib/ld-2.26.so

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
	ld $(LDFLAGS) -o $@ -pie $^

libdxdata.so: dx_data.o
	gcc -fPIC $^ -shared -o $@
dx_asm_dyn: libdxdata.so dx_main.o
	ld $(LDFLAGS) -o $@ -pie $^


px_asm: px_main.o px_hello.o dx_data.o
	ld $(LDFLAGS) -o $@ -pie $^

libpxhello.so: px_hello.o
	gcc -fPIC $^ -shared -o $@

px_asm_dyn: libdxdata.so libpxhello.so px_main.o
	ld $(LDFLAGS) -o $@ -pie $^ || true

plt_asm: plt_main.o px_hello.o dx_data.o
	ld $(LDFLAGS) -o $@ -pie $^

plt_asm_dyn: libdxdata.so libpxhello.so plt_main.o
	ld $(LDFLAGS) -o $@ -pie $^

got_asm: dx_data.o got_main.o
	ld $(LDFLAGS) -o $@ -pie $^

got_asm_dyn: libdxdata.so got_main.o
	ld $(LDFLAGS) -o $@ -pie $^
