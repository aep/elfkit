use types;
use section::SectionContent;

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
    LinkedSectionIsNotStrtab{
        during: &'static str,
        link:  Option<SectionContent>,
    },
    InvalidDynamicFlags1(u64),
    FirstSectionOffsetCanNotBeLargerThanAddress,
    MissingSymtabSection,
    LinkedSectionIsNotSymtab,
    UnexpectedSectionContent,
    InvalidSymbolShndx(String, u16),
    DynsymInStaticLibrary,
    SymbolSectionIndexExtendedCannotBeWritten,
    WritingNotSynced,
    SyncingUnloadedSection,
    WritingUnloadedSection,
    NoSymbolsInObject,
    MultipleSymbolSections,
    ConflictingSymbol{
        sym:        String,
        obj1_name:  String,
        obj2_name:  String,
        obj1_hash:  String,
        obj2_hash:  String,
    },
    UndefinedReference{
        sym:    String,
        obj:    String,
    },
    MovingLockedSection{
        sec:        String,
        old_addr:   u64,
        new_addr:   u64,
        cause:      String,
    }
}

impl From<::std::io::Error> for Error {
    fn from(error: ::std::io::Error) -> Self {
        Error::Io(error)
    }
}
