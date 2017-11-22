use std::io::{Read, Write};
use {Error, Header, SectionContent};
use types;
use num_traits::{FromPrimitive, ToPrimitive};

/*
A Represents the addend used to compute the value of the relocatable field.

B Represents the base address at which a shared object has been loaded into memory
during execution. Generally, a shared object is built with a 0 base virtual
address, but the execution address will be different.

G Represents the offset into the global offset table at which the relocation entry’s
symbol will reside during execution.

GOT Represents the address of the global offset table.

L Represents the place (section offset or address) of the Procedure Linkage Table
entry for a symbol.

P Represents the place (section offset or address) of the storage unit being relocated
-> that is the relocations offset in loaded memory, so for example a relocation at offset 0x3 in
.text which is loaded at 0x100 will have P = 0x103

S Represents the value of the symbol whose index resides in the relocation entry.
The AMD64 ABI architectures uses only Elf64_Rela relocation entries
with explicit addends. The r_addend member serves as the relocation addend.
 */
#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, PartialEq, Clone)]
pub enum RelocationType {
    R_X86_64_NONE = 0,      // none none
    R_X86_64_64 = 1,        // word64 S + A
    R_X86_64_PC32 = 2,      // word32 S + A - P
    R_X86_64_GOT32 = 3,     // word32 G + A
    R_X86_64_PLT32 = 4,     // word32 L + A - P
    R_X86_64_COPY = 5,      // none none
    R_X86_64_GLOB_DAT = 6,  // wordclass S
    R_X86_64_JUMP_SLOT = 7, // wordclass S
    R_X86_64_RELATIVE = 8,  // wordclass B + A
    R_X86_64_GOTPCREL = 9,  // word32 G + GOT + A - P
    R_X86_64_32 = 10,       // word32 S + A
    R_X86_64_32S = 11,      // word32 S + A
    R_X86_64_16 = 12,       // word16 S + A
    R_X86_64_PC16 = 13,     // word16 S + A - P
    R_X86_64_8 = 14,        // word8 S + A
    R_X86_64_PC8 = 15,      // word8 S + A - P

    /// First part of the tls_index structure: ID of module containing symbol
    /// writes the module id at this location
    /// in an executable this is always exactly 1,
    /// so this reloc is only emitted for DYN where the dynamic linker
    /// needs to give the module an id
    R_X86_64_DTPMOD64 = 16, // word64

    /// Second part of tls_index: The Offset of the symbol in the TLS Block
    /// this is written into the GOT of _this_ unit by the dynamic linker,
    /// and the offset is into the TLS block of some other unit that actually
    /// defines that symbol
    R_X86_64_DTPOFF64 = 17, // word64

    /// Offset in initial TLS Block in initial exec model
    /// no idea why this needs a different reloc type, this appears to be identical
    /// to R_X86_64_DTPOFF64
    R_X86_64_TPOFF64 = 18, // word64

    /// PC Relative address to the tls_index structure in the GOT
    /// in general dynamic model
    R_X86_64_TLSGD = 19, // word32

    /// PC Relative address to the tls_index structure in the GOT
    /// in local dynamic model.  that index only contains the module id,
    /// since the offset is known at link time and will be accessed via
    /// R_X86_64_DTPOFF32
    R_X86_64_TLSLD = 20, // word32

    /// Offset of the symbol in TLS Block (local dynamic model)
    R_X86_64_DTPOFF32 = 21, // word32


    /// PC Relative offset of symbol in GOT in initial exec model
    /// drepper says this is emitted by the linker, and processed by the dynamic linker
    /// but that doesnt make any sense, since the initial exec model assumes
    /// that everything is in the same object anyway, so the linker knows the offset.
    /// more likely this is emitted by the compiler.
    R_X86_64_GOTTPOFF = 22, // word32

    /// offset in initial TLS entry
    R_X86_64_TPOFF32 = 23, // word32

    R_X86_64_PC64 = 24,            // word64 S + A - P
    R_X86_64_GOTOFF64 = 25,        // word64 S + A - GOT
    R_X86_64_GOTPC32 = 26,         // word32 GOT + A - P
    R_X86_64_SIZE32 = 32,          // word32 Z + A
    R_X86_64_SIZE64 = 33,          // word64 Z + A
    R_X86_64_GOTPC32_TLSDESC = 34, // word32
    R_X86_64_TLSDESC_CALL = 35,    // none
    R_X86_64_TLSDESC = 36,         // word64×2
    R_X86_64_IRELATIVE = 37,       // wordclass indirect (B + A)
    R_X86_64_RELATIVE64 = 38,      // word64 B + A

    //hopefully these are ok to be treated as R_X86_64_GOTPCREL
    R_X86_64_GOTPCRELX = 41,     // word32 G + GOT + A - P
    R_X86_64_REX_GOTPCRELX = 42, //word32 G + GOT + A - P
}
impl Default for RelocationType {
    fn default() -> Self {
        RelocationType::R_X86_64_NONE
    }
}

#[derive(Default, Debug, Clone)]
pub struct Relocation {
    pub addr: u64,
    pub sym: u32,
    pub rtype: RelocationType,
    pub addend: i64,
}

impl Relocation {
    pub fn entsize(eh: &Header) -> usize {
        match eh.machine {
            types::Machine::X86_64 => 3 * 8,
            _ => panic!("relocs for machine '{:?}' not implemented", eh.machine),
        }
    }

    pub fn from_reader<R>(
        mut io: R,
        _: Option<&SectionContent>,
        eh: &Header,
    ) -> Result<SectionContent, Error>
    where
        R: Read,
    {
        if eh.machine != types::Machine::X86_64 {
            return Err(Error::UnsupportedMachineTypeForRelocation(
                eh.machine.clone(),
            ));
        }

        let mut r = Vec::new();

        while let Ok(addr) = elf_read_u64!(eh, io) {
            let info = match elf_read_u64!(eh, io) {
                Ok(v) => v,
                _ => break,
            };

            let sym = (info >> 32) as u32;
            let rtype = (info & 0xffffffff) as u32;
            let rtype = match RelocationType::from_u32(rtype) {
                Some(v) => v,
                None => {
                    println!(
                        "warning: unknown relocation type {} skipped while reading",
                        rtype
                    );
                    elf_read_u64!(eh, io)?;
                    continue;
                }
            };

            let addend = elf_read_u64!(eh, io)?;

            r.push(Relocation {
                addr: addr,
                sym: sym,
                rtype: rtype,
                addend: addend as i64,
            });
        }

        Ok(SectionContent::Relocations(r))
    }

    pub fn to_writer<W>(
        &self,
        mut io: W,
        eh: &Header,
    ) -> Result<(usize), Error>
    where
        W: Write,
    {
        elf_write_u64!(eh, io, self.addr)?;

        let info = (self.sym.to_u64().unwrap() << 32) + self.rtype.to_u64().unwrap();
        elf_write_u64!(eh, io, info)?;

        elf_write_u64!(eh, io, self.addend as u64)?;

        Ok((8+8+8))
    }
}
