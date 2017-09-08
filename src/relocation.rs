use std::io::{Read};
use {Header, Error};
use types;
use num_traits::FromPrimitive;

#[derive(Debug, Clone)]
pub struct Relocation {
    pub addr:   u64,
    pub sym:    u32,
    pub rtype:  types::RelocationType,
    pub addend: i64,
}

impl Relocation {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<Vec<Relocation>, Error> where R: Read {
        if eh.machine != types::Machine::X86_64 {
            return Err(Error::UnsupportedMachineTypeForRelocation);
        }

        let mut r = Vec::new();

        while let Ok(addr) = elf_read_u64!(eh, io) {
            let info    = elf_read_u64!(eh, io)?;

            let sym   = (info >> 32) as u32;
            let rtype = (info & 0xffffffff) as u32;
            let rtype = match types::RelocationType::from_u32(rtype) {
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

        Ok(r)
    }
}
