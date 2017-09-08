Elfkit
=========

an elf read and manipulation library in rust,
intended to be used in binary manipulation utils such as strip, objcopy, link editors, etc.

It includes some binutils replacements as example code.

```
cargo run --example readelf ./tests/samples/amd64_exe
```

![screenshot](/examples/readelf-screenshot.png?raw=true)


references
==========
[https://en.wikipedia.org/wiki/Executable_and_Linkable_Format](Executable_and_Linkable_Format)
[https://software.intel.com/sites/default/files/article/402129/mpx-linux64-abi.pdf](X86_64 Abi)
[http://infocenter.arm.com/help/topic/com.arm.doc.ihi0044f/IHI0044F_aaelf.pdf](arm abi)
