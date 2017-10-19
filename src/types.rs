use std::fmt;

use Header;

bitflags! {
#[derive(Default)]
    pub struct SectionFlags: u64 {

        /// Writable
        const WRITE             = (1 << 0);
        /// Occupies memory during execution
        const ALLOC             = (1 << 1);
        /// Executable
        const EXECINSTR         = (1 << 2);
        /// Contains nul-terminated strings
        const MERGE             = (1 << 4);
        /// Contains nul-terminated strings
        const STRINGS           = (1 << 5);
        /// `sh_info' contains SHT index
        const INFO_LINK         = (1 << 6);

        /// Preserve order after combining
        const LINK_ORDER        = (1 << 7);
        /// Non-standard OS specific handling required
        const OS_NONCONFORMING  = (1 << 8);
        /// Section is member of a group
        const GROUP             = (1 << 9);
        /// Section hold thread-local data
        const TLS               = (1 << 10);

        /// Section with compressed data
        const COMPRESSED        = (1 << 11);
        /// OS-specific
        const MASKOS            = 0x0ff00000;
        /// Processor-specific
        const MASKPROC          = 0xf0000000;


        /** The section contains data that must be part of the global
           data area during program execution. Data in this area
           is addressable with a gp relative address. Any section
           with the SHF_MIPS_GPREL attribute must have a section
           header index of one of the .gptab special sections in
           the sh_link member of its section header table entry. */
        const MIPS_GPREL        = 0x10000000; //
    }
}

impl fmt::Display for SectionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dstr = String::new();

        if self.bits & SectionFlags::WRITE.bits > 0 {
            dstr += "W";
        }
        if self.bits & SectionFlags::ALLOC.bits > 0 {
            dstr += "A";
        }
        if self.bits & SectionFlags::EXECINSTR.bits > 0 {
            dstr += "X";
        }
        if self.bits & SectionFlags::MERGE.bits > 0 {
            dstr += "M";
        }
        if self.bits & SectionFlags::STRINGS.bits > 0 {
            dstr += "S";
        }
        if self.bits & SectionFlags::INFO_LINK.bits > 0 {
            dstr += "I";
        }
        if self.bits & SectionFlags::LINK_ORDER.bits > 0 {
            dstr += "L";
        }
        if self.bits & SectionFlags::OS_NONCONFORMING.bits > 0 {
            dstr += "O";
        }
        if self.bits & SectionFlags::GROUP.bits > 0 {
            dstr += "G";
        }
        if self.bits & SectionFlags::TLS.bits > 0 {
            dstr += "T";
        }
        if self.bits & SectionFlags::COMPRESSED.bits > 0 {
            dstr += "C";
        }
        if self.bits & SectionFlags::MASKOS.bits > 0 {
            dstr += "o";
        }
        if self.bits & SectionFlags::MIPS_GPREL.bits > 0 {
            dstr += "g";
        }
        dstr.fmt(f)
    }
}

bitflags! {
#[derive(Default)]
    pub struct HeaderFlags: u32 {

        /// at least one .noreorder directive in an assembly language source contributes to the
        /// object module
        const MIPS_NOREORDER    = 0x00000001;

        /// the file contains position-independent code that can be relocated in memory.
        const MIPS_PIC          = 0x00000002;

        /** the file contains code
          that follows standard calling sequence rules for
          calling position-independent code. The code in
          this file is not necessarily position independent.
          The EF_MIPS_PIC and EF_MIPS_CPIC flags must be mutually exclusive */
        const MIPS_CPIC         = 0x00000004;

        /// extensions to the basic MIPS I architecture.
        /// (from the original doc, but this doesn't appear to be used)
        const MIPS_ARCH         = 0xf0000000;

        const MIPS_ARCH_32      = 0x50000000;
        const MIPS_ARCH_64      = 0x60000000;
        const MIPS_ARCH_32R2    = 0x70000000;
        const MIPS_ARCH_64R2    = 0x80000000;


        ///original 32bit abi, this appears to be a GNU specific flag
        const MIPS_ABI_O32      = 0x00001000;

        ///the o32 abi made 64 by some undocumented gnu stuff (i sincerely hope this isn't in use)
        const MIPS_ABI_O64      = 0x00002000;

        const ARM_EABI_VER1            = 0x01000000;
        const ARM_EABI_VER2            = 0x02000000;
        const ARM_EABI_VER3            = 0x03000000;
        const ARM_EABI_VER4            = 0x04000000;
        const ARM_EABI_VER5            = 0x05000000;
        const ARM_ABI_FLOAT_HARD    = 0x00000400;
        const ARM_ABI_FLOAT_SOFT    = 0x00000200;
    }
}

bitflags! {
#[derive(Default)]
    pub struct SegmentFlags: u64 {
        const READABLE   = (1 << 2);
        const WRITABLE   = (1 << 1);
        const EXECUTABLE = (1 << 0);
    }
}

impl fmt::Display for SegmentFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dstr = format!("{:?}", self);
        if dstr == "(empty)" {
            return "".fmt(f);
        }
        let dstr = dstr.split("|")
            .map(|s| &s.trim()[0..1])
            .fold(String::new(), |acc, s| acc + s);
        dstr.fmt(f)
    }
}



#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Endianness {
    LittleEndian = 1,
    BigEndian = 2,
}
impl Default for Endianness {
    fn default() -> Self {
        Endianness::LittleEndian
    }
}



#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Class {
    Class32 = 1,
    Class64 = 2,
}
impl Default for Class {
    fn default() -> Self {
        Class::Class64
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionType(pub u32);
impl SectionType {
    /// Section header table entry unused
    pub const NULL: SectionType = SectionType(0);
    /// Program data
    pub const PROGBITS: SectionType = SectionType(1);
    /// Symbol table
    pub const SYMTAB: SectionType = SectionType(2);
    /// String table
    pub const STRTAB: SectionType = SectionType(3);
    /// Relocation entries with addends
    pub const RELA: SectionType = SectionType(4);
    /// Symbol hash table
    pub const HASH: SectionType = SectionType(5);
    /// Dynamic linking information
    pub const DYNAMIC: SectionType = SectionType(6);
    /// Notes
    pub const NOTE: SectionType = SectionType(7);
    /// Program space with no data (bss
    pub const NOBITS: SectionType = SectionType(8);
    /// Relocation entries); no addends
    pub const REL: SectionType = SectionType(9);
    /// Reserved
    pub const SHLIB: SectionType = SectionType(10);
    /// Dynamic linker symbol table
    pub const DYNSYM: SectionType = SectionType(11);
    /// Array of constructors
    pub const INIT_ARRAY: SectionType = SectionType(14);
    /// Array of destructors
    pub const FINI_ARRAY: SectionType = SectionType(15);
    /// Array of pre-constructors
    pub const PREINIT_ARRAY: SectionType = SectionType(16);
    /// Section group
    pub const GROUP: SectionType = SectionType(17);
    /// Extended section indeces
    pub const SYMTAB_SHNDX: SectionType = SectionType(18);
    /// Number of defined types
    pub const NUM: SectionType = SectionType(19);

    /// Object attributes
    pub const GNU_ATTRIBUTES: SectionType = SectionType(0x6ffffff5);
    /// GNU-style hash table
    pub const GNU_HASH: SectionType = SectionType(0x6ffffff6);
    /// Prelink library list
    pub const GNU_LIBLIST: SectionType = SectionType(0x6ffffff7);
    /// Checksum for DSO content
    pub const CHECKSUM: SectionType = SectionType(0x6ffffff8);
    pub const SUNW_MOVE: SectionType = SectionType(0x6ffffffa);
    pub const SUNW_COMDAT: SectionType = SectionType(0x6ffffffb);
    pub const SUNW_SYMINFO: SectionType = SectionType(0x6ffffffc);
    /// Version definition section
    pub const GNU_VERDEF: SectionType = SectionType(0x6ffffffd);
    /// Version needs section
    pub const GNU_VERNEED: SectionType = SectionType(0x6ffffffe);
    /// Version symbol table
    pub const GNU_VERSYM: SectionType = SectionType(0x6fffffff);

    //arm
    pub const ARM_EXIDX: SectionType = SectionType(0x70000001);
    pub const ARM_PREEMPTMAP: SectionType = SectionType(0x70000002);
    pub const ARM_ATTRIBUTES: SectionType = SectionType(0x70000003);
    pub const ARM_DEBUGOVERLAY: SectionType = SectionType(0x70000004);
    pub const ARM_OVERLAYSECTION: SectionType = SectionType(0x70000005);

    //mips
    pub const MIPS_LIBLIST: SectionType = SectionType(0x70000001);
    pub const MIPS_CONFLICT: SectionType = SectionType(0x70000002);
    pub const MIPS_GPTAB: SectionType = SectionType(0x70000003);
    pub const MIPS_UCODE: SectionType = SectionType(0x70000004);
    pub const MIPS_DEBUG: SectionType = SectionType(0x70000005);
    pub const MIPS_REGINFO: SectionType = SectionType(0x70000006);

    /// Relinkable content. this is a korhal bolter extension
    pub const RELINKABLE: SectionType = SectionType(0x6fffff01);

    pub fn to_u32(&self) -> u32 {
        let &SectionType(v) = self;
        v
    }

    pub fn typename(&self, eh: &Header) -> Option<&'static str> {
        match (&eh.machine, self) {
            (_, &SectionType::NULL) => Some("NULL"),
            (_, &SectionType::PROGBITS) => Some("PROGBITS"),
            (_, &SectionType::SYMTAB) => Some("SYMTAB"),
            (_, &SectionType::STRTAB) => Some("STRTAB"),
            (_, &SectionType::RELA) => Some("RELA"),
            (_, &SectionType::HASH) => Some("HASH"),
            (_, &SectionType::DYNAMIC) => Some("DYNAMIC"),
            (_, &SectionType::NOTE) => Some("NOTE"),
            (_, &SectionType::NOBITS) => Some("NOBITS"),
            (_, &SectionType::REL) => Some("REL"),
            (_, &SectionType::SHLIB) => Some("SHLIB"),
            (_, &SectionType::DYNSYM) => Some("DYNSYM"),
            (_, &SectionType::INIT_ARRAY) => Some("INIT_ARRAY"),
            (_, &SectionType::FINI_ARRAY) => Some("FINI_ARRAY"),
            (_, &SectionType::PREINIT_ARRAY) => Some("PREINIT_ARRAY"),
            (_, &SectionType::GROUP) => Some("GROUP"),
            (_, &SectionType::SYMTAB_SHNDX) => Some("SYMTAB_SHNDX"),
            (_, &SectionType::NUM) => Some("NUM"),
            (_, &SectionType::GNU_ATTRIBUTES) => Some("GNU_ATTRIBUTES"),
            (_, &SectionType::GNU_HASH) => Some("GNU_HASH"),
            (_, &SectionType::GNU_LIBLIST) => Some("GNU_LIBLIST"),
            (_, &SectionType::CHECKSUM) => Some("CHECKSUM"),
            (_, &SectionType::SUNW_MOVE) => Some("SUNW_move"),
            (_, &SectionType::SUNW_COMDAT) => Some("SUNW_COMDAT"),
            (_, &SectionType::SUNW_SYMINFO) => Some("SUNW_syminfo"),
            (_, &SectionType::GNU_VERDEF) => Some("GNU_VERDEF"),
            (_, &SectionType::GNU_VERNEED) => Some("GNU_VERNEED"),
            (_, &SectionType::GNU_VERSYM) => Some("GNU_VERSYM"),
            (&Machine::MIPS, &SectionType::MIPS_LIBLIST) => Some("MIPS_LIBLIST"),
            (&Machine::MIPS, &SectionType::MIPS_CONFLICT) => Some("MIPS_CONFLICT"),
            (&Machine::MIPS, &SectionType::MIPS_GPTAB) => Some("MIPS_GPTAB"),
            (&Machine::MIPS, &SectionType::MIPS_UCODE) => Some("MIPS_UCODE"),
            (&Machine::MIPS, &SectionType::MIPS_DEBUG) => Some("MIPS_DEBUG"),
            (&Machine::MIPS, &SectionType::MIPS_REGINFO) => Some("MIPS_REGINFO"),
            (&Machine::ARM, &SectionType::ARM_EXIDX) => Some("ARM_EXIDX"),
            (&Machine::ARM, &SectionType::ARM_PREEMPTMAP) => Some("ARM_PREEMPTMAP"),
            (&Machine::ARM, &SectionType::ARM_ATTRIBUTES) => Some("ARM_ATTRIBUTES"),
            (&Machine::ARM, &SectionType::ARM_DEBUGOVERLAY) => Some("ARM_DEBUGOVERLAY"),
            (&Machine::ARM, &SectionType::ARM_OVERLAYSECTION) => Some("ARM_OVERLAYSECTION"),
            (_, &SectionType::RELINKABLE) => Some("RELINKABLE"),
            (_, _) => None,
        }
    }
}

impl Default for SectionType {
    fn default() -> Self {
        SectionType::NULL
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Abi {
    SYSV = 0,
    HPUX = 1,
    NETBSD = 2,
    GNU = 3,
    SOLARIS = 6,
    AIX = 7,
    IRIX = 8,
    FREEBSD = 9,
    TRU64 = 10,
    MODESTO = 11,
    OPENBSD = 12,
    ARM_AEABI = 64,
    ARM = 97,
    STANDALONE = 255,
}
impl Default for Abi {
    fn default() -> Self {
        Abi::SYSV
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum ElfType {
    NONE = 0,
    REL = 1,
    EXEC = 2,
    DYN = 3,
    CORE = 4,
}
impl Default for ElfType {
    fn default() -> Self {
        ElfType::NONE
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SymbolType {
    /// Symbol type is unspecified
    NOTYPE = 0,
    /// Symbol is a data object
    OBJECT = 1,
    /// Symbol is a code object
    FUNC = 2,
    /// Symbol associated with a section
    SECTION = 3,
    /// Symbol's name is file name
    FILE = 4,
    /// Symbol is a common data object
    COMMON = 5,
    /// Symbol is thread-local data object
    TLS = 6,
    /// Number of defined types
    NUM = 7,
    /// Symbol is indirect code object
    GNU_IFUNC = 10,
}
impl Default for SymbolType {
    fn default() -> Self {
        SymbolType::NOTYPE
    }
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialOrd, Eq, Ord, PartialEq, Clone)]
pub enum SymbolBind {
    /// Local symbol
    LOCAL = 0,
    /// Global symbol
    GLOBAL = 1,
    /// Weak symbol
    WEAK = 2,

    /// obscure gnu thing. i hope this is the same as global
    STB_GNU_UNIQUE = 10,
}
impl Default for SymbolBind {
    fn default() -> Self {
        SymbolBind::LOCAL
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SymbolVis {
    /// Default symbol visibility rules
    DEFAULT = 0,
    /// Processor specific hidden class
    INTERNAL = 1,
    /// Sym unavailable in other modules
    HIDDEN = 2,
    /// Not preemptible, not exported
    PROTECTED = 3,
}
impl Default for SymbolVis {
    fn default() -> Self {
        SymbolVis::DEFAULT
    }
}



#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum Machine {
    /// No machine
    NONE = 0,
    /// AT&T WE 32100
    M32 = 1,
    /// SUN SPARC
    SPARC = 2,
    /// Intel 80386
    EM386 = 3,
    /// Motorola m68k family
    EM68K = 4,
    /// Motorola m88k family
    EM88K = 5,
    /// Intel MCU
    IAMCU = 6,
    /// Intel 80860
    EM860 = 7,
    /// MIPS R3000 big-endian
    MIPS = 8,
    /// IBM System/370
    S370 = 9,
    /// MIPS R3000 little-endian
    MIPS_RS3_LE = 10,
    /// HPPA
    PARISC = 15,
    /// Fujitsu VPP500
    VPP500 = 17,
    /// Sun's "v8plus
    SPARC32PLUS = 18,
    /// Intel 80960
    EM960 = 19,
    /// PowerPC
    PPC = 20,
    /// PowerPC 64-bit
    PPC64 = 21,
    /// IBM S390
    S390 = 22,
    /// IBM SPU/SPC
    SPU = 23,
    /// NEC V800 series
    V800 = 36,
    /// Fujitsu FR20
    FR20 = 37,
    /// TRW RH-32
    RH32 = 38,
    /// Motorola RCE
    RCE = 39,
    /// ARM
    ARM = 40,
    /// Digital Alpha
    FAKE_ALPHA = 41,
    /// Hitachi SH
    SH = 42,
    /// SPARC v9 64-bit
    SPARCV9 = 43,
    /// Siemens Tricore
    TRICORE = 44,
    /// Argonaut RISC Core
    ARC = 45,
    /// Hitachi H8/300
    H8_300 = 46,
    /// Hitachi H8/300H
    H8_300H = 47,
    /// Hitachi H8S
    H8S = 48,
    /// Hitachi H8/500
    H8_500 = 49,
    /// Intel Merced
    IA_64 = 50,
    /// Stanford MIPS-X
    MIPS_X = 51,
    /// Motorola Coldfire
    COLDFIRE = 52,
    /// Motorola M68HC12
    EM68HC12 = 53,
    /// Fujitsu MMA Multimedia Accelerator
    MMA = 54,
    /// Siemens PCP
    PCP = 55,
    /// Sony nCPU embeeded RISC
    NCPU = 56,
    /// Denso NDR1 microprocessor
    NDR1 = 57,
    /// Motorola Start*Core processor
    STARCORE = 58,
    /// Toyota ME16 processor
    ME16 = 59,
    /// STMicroelectronic ST100 processor
    ST100 = 60,
    /// Advanced Logic Corp. Tinyj emb.fam
    TINYJ = 61,
    /// AMD x86-64 architecture
    X86_64 = 62,
    /// Sony DSP Processor
    PDSP = 63,
    /// Digital PDP-10
    PDP10 = 64,
    /// Digital PDP-11
    PDP11 = 65,
    /// Siemens FX66 microcontroller
    FX66 = 66,
    /// STMicroelectronics ST9+ 8/16 mc
    ST9PLUS = 67,
    /// STmicroelectronics ST7 8 bit mc
    ST7 = 68,
    /// Motorola MC68HC16 microcontroller
    EM68HC16 = 69,
    /// Motorola MC68HC11 microcontroller
    EM68HC11 = 70,
    /// Motorola MC68HC08 microcontroller
    EM68HC08 = 71,
    /// Motorola MC68HC05 microcontroller
    EM68HC05 = 72,
    /// Silicon Graphics SVx
    SVX = 73,
    /// STMicroelectronics ST19 8 bit mc
    ST19 = 74,
    /// Digital VAX
    VAX = 75,
    /// Axis Communications 32-bit emb.proc
    CRIS = 76,
    /// Infineon Technologies 32-bit emb.proc
    JAVELIN = 77,
    /// Element 14 64-bit DSP Processor
    FIREPATH = 78,
    /// LSI Logic 16-bit DSP Processor
    ZSP = 79,
    /// Donald Knuth's educational 64-bit proc
    MMIX = 80,
    /// Harvard University machine-independent object files
    HUANY = 81,
    /// SiTera Prism
    PRISM = 82,
    /// Atmel AVR 8-bit microcontroller
    AVR = 83,
    /// Fujitsu FR30
    FR30 = 84,
    /// Mitsubishi D10V
    D10V = 85,
    /// Mitsubishi D30V
    D30V = 86,
    /// NEC v850
    V850 = 87,
    /// Mitsubishi M32R
    M32R = 88,
    /// Matsushita MN10300
    MN10300 = 89,
    /// Matsushita MN10200
    MN10200 = 90,
    /// picoJava
    PJ = 91,
    /// OpenRISC 32-bit embedded processor
    OPENRISC = 92,
    /// ARC International ARCompact
    ARC_COMPACT = 93,
    /// Tensilica Xtensa Architecture
    XTENSA = 94,
    /// Alphamosaic VideoCore
    VIDEOCORE = 95,
    /// Thompson Multimedia General Purpose Proc
    TMM_GPP = 96,
    /// National Semi. 32000
    NS32K = 97,
    /// Tenor Network TPC
    TPC = 98,
    /// Trebia SNP 1000
    SNP1K = 99,
    /// STMicroelectronics ST200
    ST200 = 100,
    /// Ubicom IP2xxx
    IP2K = 101,
    /// MAX processor
    MAX = 102,
    /// National Semi. CompactRISC
    CR = 103,
    /// Fujitsu F2MC16
    F2MC16 = 104,
    /// Texas Instruments msp430
    MSP430 = 105,
    /// Analog Devices Blackfin DSP
    BLACKFIN = 106,
    /// Seiko Epson S1C33 family
    SE_C33 = 107,
    /// Sharp embedded microprocessor
    SEP = 108,
    /// Arca RISC
    ARCA = 109,
    /// PKU-Unity & MPRC Peking Uni. mc series
    UNICORE = 110,
    /// eXcess configurable cpu
    EXCESS = 111,
    /// Icera Semi. Deep Execution Processor
    DXP = 112,
    /// Altera Nios II
    ALTERA_NIOS2 = 113,
    /// National Semi. CompactRISC CRX
    CRX = 114,
    /// Motorola XGATE
    XGATE = 115,
    /// Infineon C16x/XC16x
    C166 = 116,
    /// Renesas M16C
    M16C = 117,
    /// Microchip Technology dsPIC30F
    DSPIC30F = 118,
    /// Freescale Communication Engine RISC
    CE = 119,
    /// Renesas M32C
    M32C = 120,
    /// Altium TSK3000
    TSK3000 = 131,
    /// Freescale RS08
    RS08 = 132,
    /// Analog Devices SHARC family
    SHARC = 133,
    /// Cyan Technology eCOG2
    ECOG2 = 134,
    /// Sunplus S+core7 RISC
    SCORE7 = 135,
    /// New Japan Radio (NJR) 24-bit DSP
    DSP24 = 136,
    /// Broadcom VideoCore III
    VIDEOCORE3 = 137,
    /// RISC for Lattice FPGA
    LATTICEMIC32 = 138,
    /// Seiko Epson C17
    SE_C17 = 139,
    /// Texas Instruments TMS320C6000 DSP
    TI_C6000 = 140,
    /// Texas Instruments TMS320C2000 DSP
    TI_C2000 = 141,
    /// Texas Instruments TMS320C55x DSP
    TI_C5500 = 142,
    /// Texas Instruments App. Specific RISC
    TI_ARP32 = 143,
    /// Texas Instruments Prog. Realtime Unit
    TI_PRU = 144,
    /// STMicroelectronics 64bit VLIW DSP
    MMDSP_PLUS = 160,
    /// Cypress M8C
    CYPRESS_M8C = 161,
    /// Renesas R32C
    R32C = 162,
    /// NXP Semi. TriMedia
    TRIMEDIA = 163,
    /// QUALCOMM DSP6
    QDSP6 = 164,
    /// Intel 8051 and variants
    EM8051 = 165,
    /// STMicroelectronics STxP7x
    STXP7X = 166,
    /// Andes Tech. compact code emb. RISC
    NDS32 = 167,
    /// Cyan Technology eCOG1X
    ECOG1X = 168,
    /// Dallas Semi. MAXQ30 mc
    MAXQ30 = 169,
    /// New Japan Radio (NJR) 16-bit DSP
    XIMO16 = 170,
    /// M2000 Reconfigurable RISC
    MANIK = 171,
    /// Cray NV2 vector architecture
    CRAYNV2 = 172,
    /// Renesas RX
    RX = 173,
    /// Imagination Tech. META
    METAG = 174,
    /// MCST Elbrus
    MCST_ELBRUS = 175,
    /// Cyan Technology eCOG16
    ECOG16 = 176,
    /// National Semi. CompactRISC CR16
    CR16 = 177,
    /// Freescale Extended Time Processing Unit
    ETPU = 178,
    /// Infineon Tech. SLE9X
    SLE9X = 179,
    /// Intel L10M
    L10M = 180,
    /// Intel K10M
    K10M = 181,
    /// ARM AARCH64
    AARCH64 = 183,
    /// Amtel 32-bit microprocessor
    AVR32 = 185,
    /// STMicroelectronics STM8
    STM8 = 186,
    /// Tileta TILE64
    TILE64 = 187,
    /// Tilera TILEPro
    TILEPRO = 188,
    /// Xilinx MicroBlaze
    MICROBLAZE = 189,
    /// NVIDIA CUDA
    CUDA = 190,
    /// Tilera TILE-Gx
    TILEGX = 191,
    /// CloudShield
    CLOUDSHIELD = 192,
    /// KIPO-KAIST Core-A 1st gen
    COREA_1ST = 193,
    /// KIPO-KAIST Core-A 2nd gen
    COREA_2ND = 194,
    /// Synopsys ARCompact V2
    ARC_COMPACT2 = 195,
    /// Open8 RISC
    OPEN8 = 196,
    /// Renesas RL78
    RL78 = 197,
    /// Broadcom VideoCore V
    VIDEOCORE5 = 198,
    /// Renesas 78KOR
    EM78KOR = 199,
    /// Freescale 56800EX DSC
    EM56800EX = 200,
    /// Beyond BA1
    BA1 = 201,
    /// Beyond BA2
    BA2 = 202,
    /// XMOS xCORE
    XCORE = 203,
    /// Microchip 8-bit PIC(r
    MCHP_PIC = 204,
    /// KM211 KM32
    KM32 = 210,
    /// KM211 KMX32
    KMX32 = 211,
    /// KM211 KMX16
    EMX16 = 212,
    /// KM211 KMX8
    EMX8 = 213,
    /// KM211 KVARC
    KVARC = 214,
    /// Paneve CDP
    CDP = 215,
    /// Cognitive Smart Memory Processor
    COGE = 216,
    /// Bluechip CoolEngine
    COOL = 217,
    /// Nanoradio Optimized RISC
    NORC = 218,
    /// CSR Kalimba
    CSR_KALIMBA = 219,
    /// Zilog Z80
    Z80 = 220,
    /// Controls and Data Services VISIUMcore
    VISIUM = 221,
    /// FTDI Chip FT32
    FT32 = 222,
    /// Moxie processor
    MOXIE = 223,
    /// AMD GPU
    AMDGPU = 224,
    /// RISC-V
    RISCV = 243,
    /// Linux BPF -- in-kernel virtual machine
    BPF = 247,
}
impl Default for Machine {
    fn default() -> Self {
        Machine::NONE
    }
}


#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum SegmentType {
    /// Program header table entry unused
    NULL = 0,
    /// Loadable program segment
    LOAD = 1,
    /// Dynamic linking information
    DYNAMIC = 2,
    /// Program interpreter
    INTERP = 3,
    /// Auxiliary information
    NOTE = 4,
    /// Reserved
    SHLIB = 5,
    /// Entry for header table itself
    PHDR = 6,
    /// Thread-local storage segment
    TLS = 7,
    /// Number of defined types
    NUM = 8,
    /// GCC .eh_frame_hdr segment
    GNU_EH_FRAME = 0x6474e550,
    /// Indicates stack executability
    GNU_STACK = 0x6474e551,
    /// Read-only after relocation
    GNU_RELRO = 0x6474e552,
    /// Sun Specific segment
    SUNWBSS = 0x6ffffffa,
    /// Stack segment
    SUNWSTACK = 0x6ffffffb,

    CPU0 = 0x70000000,
    CPU1 = 0x70000001,
    CPU2 = 0x70000002,
    CPU3 = 0x70000003,
}

impl Default for SegmentType {
    fn default() -> Self {
        SegmentType::NULL
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum DynamicType {
    /// Marks end of dynamic section
    NULL = 0,
    /// Name of needed library
    NEEDED = 1,
    /// Size in bytes of PLT relocs
    PLTRELSZ = 2,
    /// Processor defined value
    PLTGOT = 3,
    /// Address of symbol hash table
    HASH = 4,
    /// Address of string table
    STRTAB = 5,
    /// Address of symbol table
    SYMTAB = 6,
    /// Address of Rela relocs
    RELA = 7,
    /// Total size of Rela relocs
    RELASZ = 8,
    /// Size of one Rela reloc
    RELAENT = 9,
    /// Size of string table
    STRSZ = 10,
    /// Size of one symbol table entry
    SYMENT = 11,
    /// Address of init function
    INIT = 12,
    /// Address of termination function
    FINI = 13,
    /// Name of shared object
    SONAME = 14,
    /// Library search path (deprecated
    RPATH = 15,
    /// Start symbol search here
    SYMBOLIC = 16,
    /// Address of Rel relocs
    REL = 17,
    /// Total size of Rel relocs
    RELSZ = 18,
    /// Size of one Rel reloc
    RELENT = 19,
    /// Type of reloc in PLT
    PLTREL = 20,
    /// For debugging; unspecified
    DEBUG = 21,
    /// Reloc might modify .text
    TEXTREL = 22,
    /// Address of PLT relocs
    JMPREL = 23,
    /// Process relocations of object
    BIND_NOW = 24,
    /// Array with addresses of init fct
    INIT_ARRAY = 25,
    /// Array with addresses of fini fct
    FINI_ARRAY = 26,
    /// Size in bytes of DT_INIT_ARRAY
    INIT_ARRAYSZ = 27,
    /// Size in bytes of DT_FINI_ARRAY
    FINI_ARRAYSZ = 28,
    /// Library search path
    RUNPATH = 29,
    /// Flags for the object being loaded
    FLAGS = 30,
    /// Array with addresses of preinit fct
    PREINIT_ARRAY = 32,
    /// size in bytes of DT_PREINIT_ARRAY
    PREINIT_ARRAYSZ = 33,
    /// Number used
    NUM = 34,

    /// Prelinking timestamp
    GNU_PRELINKED = 0x6ffffdf5,
    /// Size of conflict section
    GNU_CONFLICTSZ = 0x6ffffdf6,
    /// Size of library list
    GNU_LIBLISTSZ = 0x6ffffdf7,
    CHECKSUM = 0x6ffffdf8,
    PLTPADSZ = 0x6ffffdf9,
    MOVEENT = 0x6ffffdfa,
    MOVESZ = 0x6ffffdfb,
    /// Feature selection (DTF_
    FEATURE_1 = 0x6ffffdfc,
    /// Flags for DT_* entries, effecting the following DT_* entry
    POSFLAG_1 = 0x6ffffdfd,
    /// Size of syminfo table (in bytes
    SYMINSZ = 0x6ffffdfe,
    /// Entry size of syminfo
    SYMINENT = 0x6ffffdff,

    /// GNU-style hash table
    GNU_HASH = 0x6ffffef5,
    TLSDESC_PLT = 0x6ffffef6,
    TLSDESC_GOT = 0x6ffffef7,
    /// Start of conflict section
    GNU_CONFLICT = 0x6ffffef8,
    /// Library list
    GNU_LIBLIST = 0x6ffffef9,
    /// Configuration information
    CONFIG = 0x6ffffefa,
    /// Dependency auditing
    DEPAUDIT = 0x6ffffefb,
    /// Object auditing
    AUDIT = 0x6ffffefc,
    /// PLT padding
    PLTPAD = 0x6ffffefd,
    /// Move table
    MOVETAB = 0x6ffffefe,
    /// Syminfo table
    SYMINFO = 0x6ffffeff,

    VERSYM = 0x6ffffff0,
    RELACOUNT = 0x6ffffff9,
    RELCOUNT = 0x6ffffffa,
    /// State flags, see DF_1_* below
    FLAGS_1 = 0x6ffffffb,
    /// Address of version definition table
    VERDEF = 0x6ffffffc,
    /// Number of version definitions
    VERDEFNUM = 0x6ffffffd,
    /// Address of table with needed versions
    VERNEED = 0x6ffffffe,
    /// Number of needed versions
    VERNEEDNUM = 0x6fffffff,
    /// Shared object to load before self
    AUXILIARY = 0x7ffffffd,
    /// Shared object to get values from
    FILTER = 0x7fffffff,

    MIPS_RLD_VERSION = 0x70000001,
    MIPS_TIME_STAMP = 0x70000002,
    MIPS_ICHECKSUM = 0x70000003,
    MIPS_IVERSION = 0x70000004,
    MIPS_FLAGS = 0x70000005,
    MIPS_BASE_ADDRESS = 0x70000006,
    MIPS_CONFLICT = 0x70000008,
    MIPS_LIBLIST = 0x70000009,
    MIPS_LOCAL_GOTNO = 0x7000000A,
    MIPS_CONFLICTNO = 0x7000000B,
    MIPS_LIBLISTNO = 0x70000010,
    MIPS_SYMTABNO = 0x70000011,
    MIPS_UNREFEXTNO = 0x70000012,
    MIPS_GOTSYM = 0x70000013,
    MIPS_HIPAGENO = 0x70000014,
    MIPS_RLD_MAP = 0x70000016,
}


impl Default for DynamicType {
    fn default() -> Self {
        DynamicType::NULL
    }
}



// docs text from https://git.kindwolf.org/elfwalk/blob/master/elfwalk
bitflags! {
#[derive(Default)]
    pub struct DynamicFlags1: u64 {
            ///perform complete relocation processing
            const NOW       = 1 << 0;
            ///set RTLD_GLOBAL for this object
            const GLOBAL    = 1 << 1;
            ///indicate object is a member of a group
            const GROUP     = 1 << 2;
            ///object cannot be deleted from a process
            const NODELETE  = 1 << 3;
            ///ensure immediate loading of filtees
            const LOADFLTR  = 1 << 4;
            ///object's initialization occurs first
            const INITFIRST = 1 << 5;
            ///object cannot be used with dlopen()
            const NOOPEN    = 1 << 6;
            ///$ORIGIN processing required
            const ORIGIN    = 1 << 7;
            ///direct bindings enabled
            const DIRECT    = 1 << 8;
            ///meaning unknown / undefined
            const TRANS     = 1 << 9;
            ///object is an interposer
            const INTERPOSE = 1 << 10;
            ///ignore the default library search path
            const NODEFLIB  = 1 << 11;
            ///object cannot be dumped with dldump()
            const NODUMP    = 1 << 12;
            ///object is a configuration alternative
            const CONFALT   = 1 << 13;
            ///filtee terminates filter's search
            const ENDFILTEE = 1 << 14;
            ///displacement relocation has been carried out at build time
            const DISPRELDNE= 1 << 15;
            ///displacement relocation pending (to be applied at run-time)
            const DISPRELPND= 1 << 16;
            ///object contains non-direct bindings
            const NODIRECT  = 1 << 17;
            ///internal use
            const IGNMULDEF = 1 << 18;
            ///internal use
            const NOKSYMS   = 1 << 19;
            ///internal use
            const NOHDR     = 1 << 20;
            ///object has been modified since originally built
            const EDITED    = 1 << 21;
            ///internal use
            const NORELOC   = 1 << 22;
            ///individual symbol interposers exist for this object
            const SYMINTPOSE= 1 << 23;
            ///establish global auditing
            const GLOBAUDIT = 1 << 24;
            ///singleton symbols are used
            const SINGLETON = 1 << 25;
            ///stub
            const STUB = 1 << 26;
            ///position independant executable
            const PIE  = 1 << 27;
    }
}
