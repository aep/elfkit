use std::io::{Read, Result};
use Header;
use types;
use std;

pub trait ElfEndianReadExt: Read {
    fn elf_read_u16(&mut self, eh: &Header) -> Result<u16> {
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        match eh.ident_endianness {
            types::Endianness::LittleEndian => self.read_u16::<LittleEndian>(),
            types::Endianness::BigEndian    => self.read_u16::<BigEndian>(),
        }
    }
    fn elf_read_u32(&mut self, eh: &Header) -> Result<u32> {
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        match eh.ident_endianness {
            types::Endianness::LittleEndian => self.read_u32::<LittleEndian>(),
            types::Endianness::BigEndian    => self.read_u32::<BigEndian>(),
        }
    }
}
impl<R: Read + ?Sized> ElfEndianReadExt for R {}


//adapted from https://github.com/cole14/rust-elf/blob/master/src/utils.rs

#[macro_export]
macro_rules! elf_read_u16 {
    ($header:expr, $io:ident) => ({
        use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => $io.read_u16::<LittleEndian>(),
            types::Endianness::BigEndian    => $io.read_u16::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_u32 {
    ($header:expr, $io:ident) => ({
        use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian  => $io.read_u32::<LittleEndian>(),
            types::Endianness::BigEndian     => $io.read_u32::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_u64 {
    ($header:expr, $io:ident) => ({
        use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
             types::Endianness::LittleEndian  => $io.read_u64::<LittleEndian>(),
             types::Endianness::BigEndian     => $io.read_u64::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_uclass {
    ($header:expr, $io:ident) => ({
        use types;
        match $header.ident_class {
            types::Class::Class32 => match elf_read_u32!($header, $io) {
                Err(e) => Err(e),
                Ok(v)  => Ok(v as u64),
            },
            types::Class::Class64=> elf_read_u64!($header, $io),
        }
    });
}


#[macro_export]
macro_rules! elf_write_u16 {
    ($header:expr, $io:ident, $val:expr) => ({
        use byteorder::{LittleEndian, BigEndian, WriteBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => $io.write_u16::<LittleEndian>($val),
            types::Endianness::BigEndian    => $io.write_u16::<BigEndian>($val),
        }
    });
}

#[macro_export]
macro_rules! elf_write_u32 {
    ($header:expr, $io:ident, $val:expr) => ({
        use byteorder::{LittleEndian, BigEndian, WriteBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => $io.write_u32::<LittleEndian>($val),
            types::Endianness::BigEndian    => $io.write_u32::<BigEndian>($val),
        }
    });
}

#[macro_export]
macro_rules! elf_write_u64 {
    ($header:expr, $io:ident, $val:expr) => ({
        use byteorder::{LittleEndian, BigEndian, WriteBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => $io.write_u64::<LittleEndian>($val),
            types::Endianness::BigEndian    => $io.write_u64::<BigEndian>($val),
        }
    });
}

#[macro_export]
macro_rules! elf_write_uclass {
    ($header:expr, $io:ident, $val:expr) => ({
        use types;
        match $header.ident_class {
            types::Class::Class32 => elf_write_u32!($header, $io, $val as u32),
            types::Class::Class64 => elf_write_u64!($header, $io, $val),
        }
    });
}

#[macro_export]
macro_rules! elf_dispatch_endianness {
    ($header:expr => $block:expr) => ({
        use byteorder::{self, ReadBytesExt};
        use std;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => {
                type DispatchedEndian = byteorder::LittleEndian;
                #[allow(dead_code)]
                fn read_u16<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u16> {
                    r.read_u16::<byteorder::LittleEndian>()
                }
                #[allow(dead_code)]
                fn read_u32<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u32> {
                    r.read_u32::<byteorder::LittleEndian>()
                }
                #[allow(dead_code)]
                fn read_u64<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u64> {
                    r.read_u64::<byteorder::LittleEndian>()
                }
                $block
            },
            types::Endianness::BigEndian => {
                type DispatchedEndian = byteorder::BigEndian;
                #[allow(dead_code)]
                fn read_u16<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u16> {
                    r.read_u16::<byteorder::BigEndian>()
                }
                #[allow(dead_code)]
                fn read_u32<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u32> {
                    r.read_u32::<byteorder::BigEndian>()
                }
                #[allow(dead_code)]
                fn read_u64<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u64> {
                    r.read_u64::<byteorder::BigEndian>()
                }
                $block
            },
        }
    })
}

#[macro_export]
macro_rules! elf_dispatch_uclass {
    ($header:expr => $block:expr) => ({
        use std;
        match $header.ident_class {
            types::Class::Class32 => {
                fn read_uclass<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u64> {
                    Ok(r.read_u32::<DispatchedEndian>()? as u64)
                }
                $block
            },
            types::Class::Class64 => {
                fn read_uclass<R:ReadBytesExt>(r: &mut R) -> std::io::Result<u64> {
                    r.read_u64::<DispatchedEndian>()
                }
                $block
            },
        }
    })
}

pub fn hextab<S>(align: usize, s: S) -> String
where
    S: std::fmt::LowerHex,
{
    let s = format!("{:.align$x}", s, align = align);
    let pad: String = vec!['0'; align - s.len()].into_iter().collect();
    format!("\x1b[90m{}\x1b[0;m{}", pad, s)
}

