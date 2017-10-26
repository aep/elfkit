use std::io::Write;
use elfkit::{
    Elf, Header
};


/// given value and addr as 64bit address relative to BASE
/// at runtime write the absolute value into addr
pub const len_bootstrap_abs64 : usize = 3 + 4 + 3 + 4;
pub fn write_bootstrap_abs64(eh: &Header, codeoff: u64, code: &mut Vec<u8>, value: u64, addr: u64) {

    // the value is given as absolute value from BASE,
    // undo the rip relative so it's redone at runtime
    // to prododuce the absolute value
    // note that rip is the _next_ instruction
    let mut rip     = codeoff + code.len() as u64 + 3 + 4;
    let io          = code;
    let relative_value = ((value as i64) - (rip as i64)) as i32;

    // lea -> %rax
    io.write(&[0x48,0x8d,0x05]);
    elf_write_u32!(&eh, io, relative_value as u32);

    rip += 3 + 4;
    // the write address is given as absolute too. again, undo the rip relative
    let relative_address = ((addr as i64) - (rip  as i64)) as i32;

    //mov %rax, ..(%rip)
    io.write(&[0x48,0x89,0x05]);
    elf_write_u32!(&eh, io, relative_address as u32);
}

/// write exact absolute value (usually not an address) to addr
pub const len_bootstrap_val64 : usize = 2 + 8 + 3 + 4;
pub fn write_bootstrap_val64(eh: &Header, codeoff: u64, code: &mut Vec<u8>, value: u64, addr: u64) {

    let mut rip     = codeoff + code.len() as u64 + 3 + 4;
    let io          = code;
    let relative_value = ((value as i64) - (rip as i64)) as i32;

    // movabs value -> %rax
    io.write(&[0x48,0xb8]);
    elf_write_u64!(&eh, io, value);

    rip += 2 + 8;
    // the write address is given as absolute, undo the rip relative
    let relative_address = ((addr as i64) - (rip  as i64)) as i32;

    //mov %rax, ..(%rip)
    io.write(&[0x48,0x89,0x05]);
    elf_write_u32!(&eh, io, relative_address as u32);
}

/// given value and addr as 64bit address relative to BASE
/// at runtime write the value relative to addr into addr
pub const len_bootstrap_rel32 : usize = 2 + 4 + 4;
pub fn write_bootstrap_rel32(eh: &Header, codeoff: u64, code: &mut Vec<u8>, value: u64, addr: u64) {
    let mut rip     = codeoff + code.len() as u64 + 2 + 4 + 4;
    let io          = code;
    let relative_address = ((addr as i64)  - (rip  as i64)) as i32;
    let relative_value   = ((value as i64) - (addr as i64)) as i32;

    // movl relative_value,relative_address(%rip)
    io.write(&[0xc7,0x05]);
    elf_write_u32!(&eh, io, relative_address as u32);
    elf_write_u32!(&eh, io, relative_value   as u32);
}

pub const len_reljumpto : usize = 1+4;
pub fn write_reljumpto(eh: &Header, codeoff: u64, code: &mut Vec<u8>, targetaddr: u64) {
    let pc  = codeoff + code.len() as u64 + 1 + 4;
    let io  = code;
    let rel = ((targetaddr as i64) - (pc as i64)) as i32;
    // jmpq
    io.write(&[0xe9]);
    elf_write_u32!(&eh, io, rel as u32);
}



