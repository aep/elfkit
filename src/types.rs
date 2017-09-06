enum_from_primitive! {
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
    pub enum RelocationType {
        R_X86_64_NONE       = 0, // none none
        R_X86_64_64         = 1, // word64 S + A
        R_X86_64_PC32       = 2, // word32 S + A - P
        R_X86_64_GOT32      = 3, // word32 G + A
        R_X86_64_PLT32      = 4, // word32 L + A - P
        R_X86_64_COPY       = 5, // none none
        R_X86_64_GLOB_DAT   = 6, // wordclass S
        R_X86_64_JUMP_SLOT  = 7, // wordclass S
        R_X86_64_RELATIVE   = 8, // wordclass B + A
        R_X86_64_GOTPCREL   = 9, // word32 G + GOT + A - P
        R_X86_64_32         = 10, // word32 S + A
        R_X86_64_32S        = 11, // word32 S + A
        R_X86_64_16         = 12, // word16 S + A
        R_X86_64_PC16       = 13, // word16 S + A - P
        R_X86_64_8          = 14, // word8 S + A
        R_X86_64_PC8        = 15, // word8 S + A - P
        R_X86_64_DTPMOD64   = 16, // word64
        R_X86_64_DTPOFF64   = 17, // word64
        R_X86_64_TPOFF64    = 18, // word64
        R_X86_64_TLSGD      = 19, // word32
        R_X86_64_TLSLD      = 20, // word32
        R_X86_64_DTPOFF32   = 21, // word32
        R_X86_64_GOTTPOFF   = 22, // word32
        R_X86_64_TPOFF32    = 23, // word32
        R_X86_64_PC64       = 24, // word64 S + A - P
        R_X86_64_GOTOFF64   = 25, // word64 S + A - GOT
        R_X86_64_GOTPC32    = 26, // word32 GOT + A - P
        R_X86_64_SIZE32     = 32, // word32 Z + A
        R_X86_64_SIZE64     = 33, // word64 Z + A
        R_X86_64_GOTPC32_TLSDESC    = 34, // word32
        R_X86_64_TLSDESC_CALL       = 35, // none
        R_X86_64_TLSDESC    = 36, // word64×2
        R_X86_64_IRELATIVE  = 37, // wordclass indirect (B + A)
        R_X86_64_RELATIVE64 = 38, // word64 B + A
    }
}

impl Default for RelocationType{
    fn default() -> Self {RelocationType::R_X86_64_NONE}
}


enum_from_primitive! {
#[derive(Debug, PartialEq)]
    pub enum Endianness {
        LittleEndian = 1,
        BigEndian    = 2,
    }
}

impl Default for Endianness{
    fn default() -> Self {Endianness::LittleEndian}
}



enum_from_primitive! {
#[derive(Debug, PartialEq)]
    pub enum Class {
        Class32 = 1,
        Class64 = 2,
    }
}

impl Default for Class {
    fn default() -> Self {Class::Class64}
}


enum_from_primitive! {
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
    pub enum SectionType {
        NULL            = 0,    /* Section header table entry unused */
        PROGBITS        = 1,    /* Program data */
        SYMTAB          = 2,    /* Symbol table */
        STRTAB          = 3,    /* String table */
        RELA            = 4,    /* Relocation entries with addends */
        HASH            = 5,    /* Symbol hash table */
        DYNAMIC         = 6,    /* Dynamic linking information */
        NOTE            = 7,    /* Notes */
        NOBITS          = 8,    /* Program space with no data (bss) */
        REL             = 9,    /* Relocation entries, no addends */
        SHLIB           = 10,    /* Reserved */
        DYNSYM          = 11,        /* Dynamic linker symbol table */
        INIT_ARRAY      = 14,        /* Array of constructors */
        FINI_ARRAY      = 15,        /* Array of destructors */
        PREINIT_ARRAY   = 16,        /* Array of pre-constructors */
        GROUP           = 17,        /* Section group */
        SYMTAB_SHNDX    = 18,        /* Extended section indeces */
        NUM             = 19,        /* Number of defined types.  */
        GNU_ATTRIBUTES  = 0x6ffffff5,    /* Object attributes.  */
        GNU_HASH        = 0x6ffffff6,    /* GNU-style hash table.  */
        GNU_LIBLIST     = 0x6ffffff7,    /* Prelink library list */
        CHECKSUM        = 0x6ffffff8,    /* Checksum for DSO content.  */
        SUNW_move       = 0x6ffffffa,
        SUNW_COMDAT     = 0x6ffffffb,
        SUNW_syminfo    = 0x6ffffffc,
        GNU_VERDEF      = 0x6ffffffd,   /* Version definition section.  */
        GNU_VERNEED     = 0x6ffffffe,   /* Version needs section.  */
        GNU_VERSYM      = 0x6fffffff,   /* Version symbol table.  */
    }
}

impl Default for SectionType {
    fn default() -> Self {SectionType::NULL}
}

enum_from_primitive! {
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
    pub enum SymbolType {
        NOTYPE      = 0,    /* Symbol type is unspecified */
        OBJECT      = 1,    /* Symbol is a data object */
        FUNC        = 2,    /* Symbol is a code object */
        SECTION     = 3,    /* Symbol associated with a section */
        FILE        = 4,    /* Symbol's name is file name */
        COMMON      = 5,    /* Symbol is a common data object */
        TLS         = 6,    /* Symbol is thread-local data object*/
        NUM         = 7,    /* Number of defined types.  */
        GNU_IFUNC   = 10,   /* Symbol is indirect code object */
    }
}

impl Default for SymbolType {
    fn default() -> Self {SymbolType::NOTYPE}
}


enum_from_primitive! {
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
    pub enum SymbolBind {
        LOCAL       = 0,    /* Local symbol */
        GLOBAL      = 1,    /* Global symbol */
        WEAK        = 2,    /* Weak symbol */
        NUM         = 3,    /* Number of defined types.  */
        GNU_UNIQUE  = 10,   /* Unique symbol.  */
    }
}

impl Default for SymbolBind{
    fn default() -> Self {SymbolBind::LOCAL}
}

enum_from_primitive! {
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
    pub enum SymbolVis {
        DEFAULT     = 0,    /* Default symbol visibility rules */
        INTERNAL    = 1,    /* Processor specific hidden class */
        HIDDEN      = 2,    /* Sym unavailable in other modules */
        PROTECTED   = 3,    /* Not preemptible, not exported */
    }
}

impl Default for SymbolVis{
    fn default() -> Self {SymbolVis::DEFAULT}
}


