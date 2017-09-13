use std::io::{Read, Write};
use {Header, Error, SectionContent};
use types;
use num_traits::{FromPrimitive, ToPrimitive};

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum RelocationType {
    R_X86_64_NONE       = 0,  // none none
    R_X86_64_64         = 1,  // word64 S + A
    R_X86_64_PC32       = 2,  // word32 S + A - P
    R_X86_64_GOT32      = 3,  // word32 G + A
    R_X86_64_PLT32      = 4,  // word32 L + A - P
    R_X86_64_COPY       = 5,  // none none
    R_X86_64_GLOB_DAT   = 6,  // wordclass S
    R_X86_64_JUMP_SLOT  = 7,  // wordclass S
    R_X86_64_RELATIVE   = 8,  // wordclass B + A
    R_X86_64_GOTPCREL   = 9,  // word32 G + GOT + A - P
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

#[derive(Debug, Clone)]
pub struct Relocation {
    pub addr:   u64,
    pub sym:    u32,
    pub rtype:  RelocationType,
    pub addend: i64,
}

impl Relocation {

    pub fn entsize(eh: &Header) -> usize {
        match eh.machine {
            types::Machine::X86_64 => 3 * 8,
            _ => 0
        }
    }

    pub fn from_reader<R>(mut io: R, _: Option<&SectionContent>, eh: &Header) -> Result<SectionContent, Error> where R: Read{
        if eh.machine != types::Machine::X86_64 {
            return Err(Error::UnsupportedMachineTypeForRelocation);
        }

        let mut r = Vec::new();

        while let Ok(addr) = elf_read_u64!(eh, io) {
            let info = match elf_read_u64!(eh, io) {
                Ok(v) => v,
                _ => break,
            };

            let sym   = (info >> 32) as u32;
            let rtype = (info & 0xffffffff) as u32;
            let rtype = match RelocationType::from_u32(rtype) {
                Some(v) => v,
                None => continue,
            };

            let addend  = elf_read_u64!(eh, io)?;

            r.push(Relocation{
                addr: addr,
                sym: sym,
                rtype: rtype,
                addend: addend as i64,
            });
        }

        Ok(SectionContent::Relocations(r))
    }

    pub fn to_writer<W>(&self, mut io: W, _: Option<&mut SectionContent>, eh: &Header)
        -> Result<(), Error> where W: Write {

            elf_write_u64!(eh, io, self.addr)?;

            let info = (self.sym.to_u64().unwrap() << 32) + self.rtype.to_u64().unwrap();
            elf_write_u64!(eh, io, info)?;

            elf_write_u64!(eh, io, self.addend as u64)?;

            Ok(())
        }
}
