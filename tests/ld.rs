extern crate elfkit;

use std::env;
use std::fs::File;
use elfkit::Elf;
use elfkit::symbol::Symbol;
use std::io::{Read, Seek, SeekFrom};
use std::process::Command;

fn link_and_assert_fox(aargs: &[&str]) {
    let mut args = vec!["run", "--example", "ld", "--", "-dynamic-linker", "/lib64/ld-linux-x86-64.so.2"];
    args.extend(aargs);

    assert!(Command::new("cargo").args(&args).status().unwrap().success());
    assert_eq!(Command::new("./a.out").output().unwrap().stdout,
    b"The quick brown fox jumps over the lazy dog".to_vec());
}

#[test]
fn link_simple() {
    link_and_assert_fox(&["tests/samples/simple_asm.o"]);
}

#[test]
fn link_data() {
    link_and_assert_fox(&["tests/samples/dx_main.o", "tests/samples/dx_data.o"]);
}

#[test]
fn link_fun() {
    link_and_assert_fox(&["tests/samples/px_main.o", "tests/samples/dx_data.o",  "tests/samples/px_hello.o"]);
    link_and_assert_fox(&["tests/samples/px_main.o", "tests/samples/px_hello.o", "tests/samples/dx_data.o"]);
}

#[test]
fn link_plt() {
    link_and_assert_fox(&["tests/samples/plt_main.o", "tests/samples/dx_data.o", "tests/samples/px_hello.o"]);
    link_and_assert_fox(&["tests/samples/plt_main.o", "tests/samples/px_hello.o","tests/samples/dx_data.o"]);
    link_and_assert_fox(&["tests/samples/px_hello.o", "tests/samples/dx_data.o", "tests/samples/plt_main.o"]);
}

