use std::io::{Read, Result};
use Header;
use types;

pub trait ElfEndianReadExt: Read {
    fn elf_read_u16(&mut self, eh: &Header) -> Result<u16> {
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        match eh.ident_endianness {
            types::Endianness::LittleEndian => self.read_u16::<LittleEndian>(),
            types::Endianness::BigEndian => self.read_u16::<BigEndian>(),
        }
    }
    fn elf_read_u32(&mut self, eh: &Header) -> Result<u32> {
        use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
        match eh.ident_endianness {
            types::Endianness::LittleEndian => self.read_u32::<LittleEndian>(),
            types::Endianness::BigEndian => self.read_u32::<BigEndian>(),
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
            types::Class::Class32=> match elf_read_u32!($header, $io) {
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
