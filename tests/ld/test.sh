#!/bin/sh

link_and_assert_fox() {
    cargo run --example ld -- -dynamic-linker /lib64/ld-linux-x86-64.so.2 "$@"
    output=$(./a.out)
    if [ "$output" != "The quick brown fox jumps over the lazy dog" ]
    then
        echo "FAIL $@"
        echo "  output was: '$output'"
        exit 1
    else
        echo "PASS $@"
    fi
}


link_and_assert_fox ../samples/simple_asm.o
link_and_assert_fox ../samples/dx_main.o ../samples/dx_data.o
link_and_assert_fox ../samples/px_main.o ../samples/px_hello.o ../samples/dx_data.o
link_and_assert_fox ../samples/plt_main.o ../samples/px_hello.o ../samples/dx_data.o

