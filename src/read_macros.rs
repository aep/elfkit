//adapted from https://github.com/cole14/rust-elf/blob/master/src/utils.rs

#[macro_export]
macro_rules! elf_read_u16 {
    ($header:ident, $io:ident) => ({
        use self::byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian => $io.read_u16::<LittleEndian>(),
            types::Endianness::BigEndian    => $io.read_u16::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_u32 {
    ($header:ident, $io:ident) => ({
        use self::byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
            types::Endianness::LittleEndian  => $io.read_u32::<LittleEndian>(),
            types::Endianness::BigEndian     => $io.read_u32::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_u64 {
    ($header:ident, $io:ident) => ({
        use self::byteorder::{LittleEndian, BigEndian, ReadBytesExt};
        use types;
        match $header.ident_endianness {
             types::Endianness::LittleEndian  => $io.read_u64::<LittleEndian>(),
             types::Endianness::BigEndian     => $io.read_u64::<BigEndian>(),
        }
    });
}

#[macro_export]
macro_rules! elf_read_uclass {
    ($header:ident, $io:ident) => ({
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

