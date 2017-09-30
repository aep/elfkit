use types;

#[derive(Debug)]
pub enum Error {
    Io(::std::io::Error),
    InvalidMagic,
    InvalidIdentClass(u8),
    InvalidEndianness(u8),
    InvalidIdentVersion(u8),
    InvalidVersion(u32),
    InvalidAbi(u8),
    InvalidElfType(u16),
    InvalidMachineType(u16),
    InvalidHeaderFlags(u32),
    InvalidSectionFlags(u64),
    InvalidSegmentType(u32),
    InvalidSectionType(u32),
    UnsupportedMachineTypeForRelocation(types::Machine),
    InvalidSymbolType(u8),
    InvalidSymbolBind(u8),
    InvalidSymbolVis(u8),
    InvalidDynamicType(u64),
    MissingShstrtabSection,
    LinkedSectionIsNotStrtab(&'static str),
    InvalidDynamicFlags1(u64),
    FirstSectionOffsetCanNotBeLargerThanAddress,
    MissingSymtabSection,
}

impl From<::std::io::Error> for Error {
    fn from(error: ::std::io::Error) -> Self {
        Error::Io(error)
    }
}

