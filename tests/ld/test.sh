#!/bin/sh

build_and_assert_fox() {
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


build_and_assert_fox ../samples/simple_asm.o

