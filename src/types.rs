use std::fmt;

bitflags! {
#[derive(Default)]
    pub struct SectionFlags: u64 {
        const WRITE             = (1 << 0);   /* Writable */
        const ALLOC             = (1 << 1);   /* Occupies memory during execution */
        const EXECINSTR         = (1 << 2);   /* Executable */
        const MERGE             = (1 << 4);   /* Contains nul-terminated strings */
        const STRINGS           = (1 << 5);   /* Contains nul-terminated strings */
        const INFO_LINK         = (1 << 6);   /* `sh_info' contains SHT index */

        const LINK_ORDER        = (1 << 7);   /* Preserve order after combining */
        const OS_NONCONFORMING  = (1 << 8);   /* Non-standard OS specific handling required */
        const GROUP             = (1 << 9);   /* Section is member of a group.  */
        const TLS               = (1 << 10);  /* Section hold thread-local data.  */

        const COMPRESSED        = (1 << 11);  /* Section with compressed data. */
        const MASKOS            = 0x0ff00000; /* OS-specific.  */
        const MASKPROC          = 0xf0000000; /* Processor-specific */
        const ORDERED           = (1 << 30);  /* Special ordering requirement (Solaris).  */
        const EXCLUDE           = (1 << 31);  /* Section is excluded unless referenced or allocated (Solaris).*/
    }
}

impl fmt::Display for SectionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dstr = format!("{:?}", self);
        if dstr == "(empty)" {
            return "".fmt(f);
        }
        let dstr = dstr.split("|").map(|s| {
            let s = s.trim();
            match s {
                "MASKOS" => "o",
                "MASKPROC" => "p",
                "EXECINSTR" => "X",
                v => &v[0..1],
            }
        }).fold(String::new(), |acc, s| acc + s );
        dstr.fmt(f)
    }
}

bitflags! {
#[derive(Default)]
    pub struct SegmentFlags: u64 {
        const READABLE = (1 << 2);
        const WRITABLE  = (1 << 1);
        const EXECUTABLE = (1 << 0);
    }
}

impl fmt::Display for SegmentFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dstr = format!("{:?}", self);
        if dstr == "(empty)" {
            return "".fmt(f);
        }
        let dstr = dstr.split("|").map(|s| {&s.trim()[0..1]}).fold(String::new(), |acc, s| acc + s );
        dstr.fmt(f)
    }
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
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
impl Default for RelocationType{
    fn default() -> Self {RelocationType::R_X86_64_NONE}
}


#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Endianness {
    LittleEndian = 1,
    BigEndian    = 2,
}
impl Default for Endianness{
    fn default() -> Self {Endianness::LittleEndian}
}



#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Class {
    Class32 = 1,
    Class64 = 2,
}
impl Default for Class {
    fn default() -> Self {Class::Class64}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
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
impl Default for SectionType {
    fn default() -> Self {SectionType::NULL}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Abi {
    SYSV        = 0,
    HPUX        = 1,
    NETBSD      = 2,
    GNU         = 3,
    SOLARIS     = 6,
    AIX         = 7,
    IRIX        = 8,
    FREEBSD     = 9,
    TRU64       = 10,
    MODESTO     = 11,
    OPENBSD     = 12,
    ARM_AEABI   = 64,
    ARM         = 97,
    STANDALONE  = 255,
}
impl Default for Abi {
    fn default() -> Self {Abi::SYSV}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum ElfType {
    NONE  = 0,
    REL   = 1,
    EXEC  = 2,
    DYN   = 3,
    CORE  = 4,
}
impl Default for ElfType {
    fn default() -> Self {ElfType::NONE}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
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
impl Default for SymbolType {
    fn default() -> Self {SymbolType::NOTYPE}
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SymbolBind {
    LOCAL       = 0,    /* Local symbol */
    GLOBAL      = 1,    /* Global symbol */
    WEAK        = 2,    /* Weak symbol */
    NUM         = 3,    /* Number of defined types.  */
    GNU_UNIQUE  = 10,   /* Unique symbol.  */
}
impl Default for SymbolBind{
    fn default() -> Self {SymbolBind::LOCAL}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SymbolVis {
    DEFAULT     = 0,    /* Default symbol visibility rules */
    INTERNAL    = 1,    /* Processor specific hidden class */
    HIDDEN      = 2,    /* Sym unavailable in other modules */
    PROTECTED   = 3,    /* Not preemptible, not exported */
}
impl Default for SymbolVis{
    fn default() -> Self {SymbolVis::DEFAULT}
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Machine{
    NONE         = 0,     /* No machine */
    M32          = 1,     /* AT&T WE 32100 */
    SPARC        = 2,     /* SUN SPARC */
    EM386        = 3,     /* Intel 80386 */
    EM68K        = 4,     /* Motorola m68k family */
    EM88K        = 5,     /* Motorola m88k family */
    IAMCU        = 6,     /* Intel MCU */
    EM860        = 7,     /* Intel 80860 */
    MIPS         = 8,     /* MIPS R3000 big-endian */
    S370         = 9,     /* IBM System/370 */
    MIPS_RS3_LE  = 10,    /* MIPS R3000 little-endian */
    PARISC       = 15,    /* HPPA */
    VPP500       = 17,    /* Fujitsu VPP500 */
    SPARC32PLUS  = 18,    /* Sun's "v8plus" */
    EM960        = 19,    /* Intel 80960 */
    PPC          = 20,    /* PowerPC */
    PPC64        = 21,    /* PowerPC 64-bit */
    S390         = 22,    /* IBM S390 */
    SPU          = 23,    /* IBM SPU/SPC */
    V800         = 36,    /* NEC V800 series */
    FR20         = 37,    /* Fujitsu FR20 */
    RH32         = 38,    /* TRW RH-32 */
    RCE          = 39,    /* Motorola RCE */
    ARM          = 40,    /* ARM */
    FAKE_ALPHA   = 41,    /* Digital Alpha */
    SH           = 42,    /* Hitachi SH */
    SPARCV9      = 43,    /* SPARC v9 64-bit */
    TRICORE      = 44,    /* Siemens Tricore */
    ARC          = 45,    /* Argonaut RISC Core */
    H8_300       = 46,    /* Hitachi H8/300 */
    H8_300H      = 47,    /* Hitachi H8/300H */
    H8S          = 48,    /* Hitachi H8S */
    H8_500       = 49,    /* Hitachi H8/500 */
    IA_64        = 50,    /* Intel Merced */
    MIPS_X       = 51,    /* Stanford MIPS-X */
    COLDFIRE     = 52,    /* Motorola Coldfire */
    EM68HC12     = 53,    /* Motorola M68HC12 */
    MMA          = 54,    /* Fujitsu MMA Multimedia Accelerator */
    PCP          = 55,    /* Siemens PCP */
    NCPU         = 56,    /* Sony nCPU embeeded RISC */
    NDR1         = 57,    /* Denso NDR1 microprocessor */
    STARCORE     = 58,    /* Motorola Start*Core processor */
    ME16         = 59,    /* Toyota ME16 processor */
    ST100        = 60,    /* STMicroelectronic ST100 processor */
    TINYJ        = 61,    /* Advanced Logic Corp. Tinyj emb.fam */
    X86_64       = 62,    /* AMD x86-64 architecture */
    PDSP         = 63,    /* Sony DSP Processor */
    PDP10        = 64,    /* Digital PDP-10 */
    PDP11        = 65,    /* Digital PDP-11 */
    FX66         = 66,    /* Siemens FX66 microcontroller */
    ST9PLUS      = 67,    /* STMicroelectronics ST9+ 8/16 mc */
    ST7          = 68,    /* STmicroelectronics ST7 8 bit mc */
    EM68HC16     = 69,    /* Motorola MC68HC16 microcontroller */
    EM68HC11     = 70,    /* Motorola MC68HC11 microcontroller */
    EM68HC08     = 71,    /* Motorola MC68HC08 microcontroller */
    EM68HC05     = 72,    /* Motorola MC68HC05 microcontroller */
    SVX          = 73,    /* Silicon Graphics SVx */
    ST19         = 74,    /* STMicroelectronics ST19 8 bit mc */
    VAX          = 75,    /* Digital VAX */
    CRIS         = 76,    /* Axis Communications 32-bit emb.proc */
    JAVELIN      = 77,    /* Infineon Technologies 32-bit emb.proc */
    FIREPATH     = 78,    /* Element 14 64-bit DSP Processor */
    ZSP          = 79,    /* LSI Logic 16-bit DSP Processor */
    MMIX         = 80,    /* Donald Knuth's educational 64-bit proc */
    HUANY        = 81,    /* Harvard University machine-independent object files */
    PRISM        = 82,    /* SiTera Prism */
    AVR          = 83,    /* Atmel AVR 8-bit microcontroller */
    FR30         = 84,    /* Fujitsu FR30 */
    D10V         = 85,    /* Mitsubishi D10V */
    D30V         = 86,    /* Mitsubishi D30V */
    V850         = 87,    /* NEC v850 */
    M32R         = 88,    /* Mitsubishi M32R */
    MN10300      = 89,    /* Matsushita MN10300 */
    MN10200      = 90,    /* Matsushita MN10200 */
    PJ           = 91,    /* picoJava */
    OPENRISC     = 92,    /* OpenRISC 32-bit embedded processor */
    ARC_COMPACT  = 93,    /* ARC International ARCompact */
    XTENSA       = 94,    /* Tensilica Xtensa Architecture */
    VIDEOCORE    = 95,    /* Alphamosaic VideoCore */
    TMM_GPP      = 96,    /* Thompson Multimedia General Purpose Proc */
    NS32K        = 97,    /* National Semi. 32000 */
    TPC          = 98,    /* Tenor Network TPC */
    SNP1K        = 99,    /* Trebia SNP 1000 */
    ST200        = 100,   /* STMicroelectronics ST200 */
    IP2K         = 101,   /* Ubicom IP2xxx */
    MAX          = 102,   /* MAX processor */
    CR           = 103,   /* National Semi. CompactRISC */
    F2MC16       = 104,   /* Fujitsu F2MC16 */
    MSP430       = 105,   /* Texas Instruments msp430 */
    BLACKFIN     = 106,   /* Analog Devices Blackfin DSP */
    SE_C33       = 107,   /* Seiko Epson S1C33 family */
    SEP          = 108,   /* Sharp embedded microprocessor */
    ARCA         = 109,   /* Arca RISC */
    UNICORE      = 110,   /* PKU-Unity & MPRC Peking Uni. mc series */
    EXCESS       = 111,   /* eXcess configurable cpu */
    DXP          = 112,   /* Icera Semi. Deep Execution Processor */
    ALTERA_NIOS2 = 113,   /* Altera Nios II */
    CRX          = 114,   /* National Semi. CompactRISC CRX */
    XGATE        = 115,   /* Motorola XGATE */
    C166         = 116,   /* Infineon C16x/XC16x */
    M16C         = 117,   /* Renesas M16C */
    DSPIC30F     = 118,   /* Microchip Technology dsPIC30F */
    CE           = 119,   /* Freescale Communication Engine RISC */
    M32C         = 120,   /* Renesas M32C */
    TSK3000      = 131,   /* Altium TSK3000 */
    RS08         = 132,   /* Freescale RS08 */
    SHARC        = 133,   /* Analog Devices SHARC family */
    ECOG2        = 134,   /* Cyan Technology eCOG2 */
    SCORE7       = 135,   /* Sunplus S+core7 RISC */
    DSP24        = 136,   /* New Japan Radio (NJR) 24-bit DSP */
    VIDEOCORE3   = 137,   /* Broadcom VideoCore III */
    LATTICEMIC32 = 138,   /* RISC for Lattice FPGA */
    SE_C17       = 139,   /* Seiko Epson C17 */
    TI_C6000     = 140,   /* Texas Instruments TMS320C6000 DSP */
    TI_C2000     = 141,   /* Texas Instruments TMS320C2000 DSP */
    TI_C5500     = 142,   /* Texas Instruments TMS320C55x DSP */
    TI_ARP32     = 143,   /* Texas Instruments App. Specific RISC */
    TI_PRU       = 144,   /* Texas Instruments Prog. Realtime Unit */
    MMDSP_PLUS   = 160,   /* STMicroelectronics 64bit VLIW DSP */
    CYPRESS_M8C  = 161,   /* Cypress M8C */
    R32C         = 162,   /* Renesas R32C */
    TRIMEDIA     = 163,   /* NXP Semi. TriMedia */
    QDSP6        = 164,   /* QUALCOMM DSP6 */
    EM8051       = 165,   /* Intel 8051 and variants */
    STXP7X       = 166,   /* STMicroelectronics STxP7x */
    NDS32        = 167,   /* Andes Tech. compact code emb. RISC */
    ECOG1X       = 168,   /* Cyan Technology eCOG1X */
    MAXQ30       = 169,   /* Dallas Semi. MAXQ30 mc */
    XIMO16       = 170,   /* New Japan Radio (NJR) 16-bit DSP */
    MANIK        = 171,   /* M2000 Reconfigurable RISC */
    CRAYNV2      = 172,   /* Cray NV2 vector architecture */
    RX           = 173,   /* Renesas RX */
    METAG        = 174,   /* Imagination Tech. META */
    MCST_ELBRUS  = 175,   /* MCST Elbrus */
    ECOG16       = 176,   /* Cyan Technology eCOG16 */
    CR16         = 177,   /* National Semi. CompactRISC CR16 */
    ETPU         = 178,   /* Freescale Extended Time Processing Unit */
    SLE9X        = 179,   /* Infineon Tech. SLE9X */
    L10M         = 180,   /* Intel L10M */
    K10M         = 181,   /* Intel K10M */
    AARCH64      = 183,   /* ARM AARCH64 */
    AVR32        = 185,   /* Amtel 32-bit microprocessor */
    STM8         = 186,   /* STMicroelectronics STM8 */
    TILE64       = 187,   /* Tileta TILE64 */
    TILEPRO      = 188,   /* Tilera TILEPro */
    MICROBLAZE   = 189,   /* Xilinx MicroBlaze */
    CUDA         = 190,   /* NVIDIA CUDA */
    TILEGX       = 191,   /* Tilera TILE-Gx */
    CLOUDSHIELD  = 192,   /* CloudShield */
    COREA_1ST    = 193,   /* KIPO-KAIST Core-A 1st gen. */
    COREA_2ND    = 194,   /* KIPO-KAIST Core-A 2nd gen. */
    ARC_COMPACT2 = 195,   /* Synopsys ARCompact V2 */
    OPEN8        = 196,   /* Open8 RISC */
    RL78         = 197,   /* Renesas RL78 */
    VIDEOCORE5   = 198,   /* Broadcom VideoCore V */
    EM78KOR      = 199,   /* Renesas 78KOR */
    EM56800EX    = 200,   /* Freescale 56800EX DSC */
    BA1          = 201,   /* Beyond BA1 */
    BA2          = 202,   /* Beyond BA2 */
    XCORE        = 203,   /* XMOS xCORE */
    MCHP_PIC     = 204,   /* Microchip 8-bit PIC(r) */
    KM32         = 210,   /* KM211 KM32 */
    KMX32        = 211,   /* KM211 KMX32 */
    EMX16        = 212,   /* KM211 KMX16 */
    EMX8         = 213,   /* KM211 KMX8 */
    KVARC        = 214,   /* KM211 KVARC */
    CDP          = 215,   /* Paneve CDP */
    COGE         = 216,   /* Cognitive Smart Memory Processor */
    COOL         = 217,   /* Bluechip CoolEngine */
    NORC         = 218,   /* Nanoradio Optimized RISC */
    CSR_KALIMBA  = 219,   /* CSR Kalimba */
    Z80          = 220,   /* Zilog Z80 */
    VISIUM       = 221,   /* Controls and Data Services VISIUMcore */
    FT32         = 222,   /* FTDI Chip FT32 */
    MOXIE        = 223,   /* Moxie processor */
    AMDGPU       = 224,   /* AMD GPU */
    RISCV        = 243,   /* RISC-V */
    BPF          = 247,   /* Linux BPF -- in-kernel virtual machine */
}
impl Default for Machine {
    fn default() -> Self {Machine::NONE}
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SegmentType {
    NULL            = 0,           /* Program header table entry unused */
    LOAD            = 1,           /* Loadable program segment */
    DYNAMIC         = 2,           /* Dynamic linking information */
    INTERP          = 3,           /* Program interpreter */
    NOTE            = 4,           /* Auxiliary information */
    SHLIB           = 5,           /* Reserved */
    PHDR            = 6,           /* Entry for header table itself */
    TLS             = 7,           /* Thread-local storage segment */
    NUM             = 8,           /* Number of defined types */
    GNU_EH_FRAME    = 0x6474e550,  /* GCC .eh_frame_hdr segment */
    GNU_STACK       = 0x6474e551,  /* Indicates stack executability */
    GNU_RELRO       = 0x6474e552,  /* Read-only after relocation */
    SUNWBSS         = 0x6ffffffa,  /* Sun Specific segment */
    SUNWSTACK       = 0x6ffffffb,  /* Stack segment */
}
impl Default for SegmentType {
    fn default() -> Self {SegmentType::NULL}
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum DynamicType {
    NULL            = 0,        /* Marks end of dynamic section */
    NEEDED          = 1,        /* Name of needed library */
    PLTRELSZ        = 2,        /* Size in bytes of PLT relocs */
    PLTGOT          = 3,        /* Processor defined value */
    HASH            = 4,        /* Address of symbol hash table */
    STRTAB          = 5,        /* Address of string table */
    SYMTAB          = 6,        /* Address of symbol table */
    RELA            = 7,        /* Address of Rela relocs */
    RELASZ          = 8,        /* Total size of Rela relocs */
    RELAENT         = 9,        /* Size of one Rela reloc */
    STRSZ           = 10,       /* Size of string table */
    SYMENT          = 11,       /* Size of one symbol table entry */
    INIT            = 12,       /* Address of init function */
    FINI            = 13,       /* Address of termination function */
    SONAME          = 14,       /* Name of shared object */
    RPATH           = 15,       /* Library search path (deprecated) */
    SYMBOLIC        = 16,       /* Start symbol search here */
    REL             = 17,       /* Address of Rel relocs */
    RELSZ           = 18,       /* Total size of Rel relocs */
    RELENT          = 19,       /* Size of one Rel reloc */
    PLTREL          = 20,       /* Type of reloc in PLT */
    DEBUG           = 21,       /* For debugging; unspecified */
    TEXTREL         = 22,       /* Reloc might modify .text */
    JMPREL          = 23,       /* Address of PLT relocs */
    BIND_NOW        = 24,       /* Process relocations of object */
    INIT_ARRAY      = 25,       /* Array with addresses of init fct */
    FINI_ARRAY      = 26,       /* Array with addresses of fini fct */
    INIT_ARRAYSZ    = 27,       /* Size in bytes of DT_INIT_ARRAY */
    FINI_ARRAYSZ    = 28,       /* Size in bytes of DT_FINI_ARRAY */
    RUNPATH         = 29,       /* Library search path */
    FLAGS           = 30,       /* Flags for the object being loaded */
    PREINIT_ARRAY   = 32,       /* Array with addresses of preinit fct*/
    PREINIT_ARRAYSZ = 33,       /* size in bytes of DT_PREINIT_ARRAY */
    NUM             = 34,       /* Number used */

    GNU_PRELINKED   = 0x6ffffdf5,   /* Prelinking timestamp */
    GNU_CONFLICTSZ  = 0x6ffffdf6,   /* Size of conflict section */
    GNU_LIBLISTSZ   = 0x6ffffdf7,   /* Size of library list */
    CHECKSUM        = 0x6ffffdf8,
    PLTPADSZ        = 0x6ffffdf9,
    MOVEENT         = 0x6ffffdfa,
    MOVESZ          = 0x6ffffdfb,
    FEATURE_1       = 0x6ffffdfc,   /* Feature selection (DTF_*).  */
    POSFLAG_1       = 0x6ffffdfd,   /* Flags for DT_* entries, effecting the following DT_* entry.  */
    SYMINSZ         = 0x6ffffdfe,   /* Size of syminfo table (in bytes) */
    SYMINENT        = 0x6ffffdff,   /* Entry size of syminfo */

    GNU_HASH        = 0x6ffffef5,   /* GNU-style hash table.  */
    TLSDESC_PLT     = 0x6ffffef6,
    TLSDESC_GOT     = 0x6ffffef7,
    GNU_CONFLICT    = 0x6ffffef8,   /* Start of conflict section */
    GNU_LIBLIST     = 0x6ffffef9,   /* Library list */
    CONFIG          = 0x6ffffefa,   /* Configuration information.  */
    DEPAUDIT        = 0x6ffffefb,   /* Dependency auditing.  */
    AUDIT           = 0x6ffffefc,   /* Object auditing.  */
    PLTPAD          = 0x6ffffefd,   /* PLT padding.  */
    MOVETAB         = 0x6ffffefe,   /* Move table.  */
    SYMINFO         = 0x6ffffeff,   /* Syminfo table.  */

    VERSYM          = 0x6ffffff0,
    RELACOUNT       = 0x6ffffff9,
    RELCOUNT        = 0x6ffffffa,
    FLAGS_1         = 0x6ffffffb,   /* State flags, see DF_1_* below.  */
    VERDEF          = 0x6ffffffc,   /* Address of version definition table */
    VERDEFNUM       = 0x6ffffffd,   /* Number of version definitions */
    VERNEED         = 0x6ffffffe,   /* Address of table with needed versions */
    VERNEEDNUM      = 0x6fffffff,   /* Number of needed versions */
    AUXILIARY       = 0x7ffffffd,   /* Shared object to load before self */
    FILTER          = 0x7fffffff,   /* Shared object to get values from */
}

impl Default for DynamicType {
    fn default() -> Self {DynamicType::NULL}
}
