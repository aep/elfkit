#!/bin/sh

link_and_assert_fox() {
    cargo run --example ld "$@"
    chmod +x /tmp/e
    output=$(/tmp/e)
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

