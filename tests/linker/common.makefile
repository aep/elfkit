OUTPUTS=ld.out ek.out
INPUTS=$(sort $(patsubst %.c,%.o,$(wildcard *.c)) \
	   $(patsubst %.asm,%.o,$(wildcard *.asm)) \
	   $(patsubst %.cpp,%.o,$(wildcard *.cpp)) \
	   $(wildcard *.o) \
	   $(wildcard *.a) \
	   $(wildcard *.lo))

all: $(OUTPUTS)
clean:
	rm -f $(OUTPUTS)

CFLAGS=-fPIC -g
LDFLAGS=--emit-relocs -pie -dynamic-linker /lib64/ld-linux-x86-64.so.2


%.o: %.asm
	nasm -g -f elf64 -o $@ $^

ld.out: $(INPUTS)
	ld -g -o $@ $(LDFLAGS) $^

ek.out: $(INPUTS)
	cargo run --bin ld -- -o $@ $(LDFLAGS) $^

.PHONY: test
test: all
	test "$$(./ld.out)" = "$$(./ek.out)" && echo PASS

