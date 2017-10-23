OUTPUTS=ld.out ek.out
INPUTS=$(sort $(patsubst %.c,%.o,$(wildcard *.c)) \
	   $(patsubst %.asm,%.o,$(wildcard *.asm)) \
	   $(patsubst %.cpp,%.o,$(wildcard *.cpp)) \
	   $(wildcard *.a) \
	   $(wildcard *.lo))

all: $(OUTPUTS)
clean:
	rm -f $(OUTPUTS) *.o

CFLAGS=-fPIC
LDFLAGS=-pie -dynamic-linker /lib64/ld-linux-x86-64.so.2

%.o: %.asm
	nasm -f elf64 -o $@ $^

ld.out: $(INPUTS)
	ld -o $@ $(LDFLAGS) $^

ek.out: $(INPUTS)
	cargo run --bin bolter  -- -o $@ $(LDFLAGS) -pie $^

.PHONY: test
test: all
	test "$$(./ld.out)" = "$$(./ek.out)"

