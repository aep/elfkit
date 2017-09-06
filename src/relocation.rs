extern crate byteorder;
extern crate num;

use std::io::{Read};
use {Header, Error};
use types::RelocationType;
use num::FromPrimitive;

#[derive(Debug)]
pub struct Amd64Relocation {
    pub addr:   u64,
    pub sym:    u32,
    pub rtype:  RelocationType,
    pub addend: i64,
}

impl Amd64Relocation {
    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<Vec<Amd64Relocation>, Error> where R: Read {
        let mut r = Vec::new();

        while let Ok(addr) = elf_read_u64!(eh, io) {
            let info    = elf_read_u64!(eh, io)?;

            let sym   = (info >> 32) as u32;
            let rtype = (info & 0xffffffff) as u32;
            let rtype = match RelocationType::from_u32(rtype) {
                Some(v) => v,
                None => continue,
            };

            let addend  = elf_read_u64!(eh, io)?;

            r.push(Amd64Relocation{
                addr: addr,
                sym: sym,
                rtype: rtype,
                addend: addend as i64,
            });
        }

        Ok(r)
    }
}
