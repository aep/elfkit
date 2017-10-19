use error::Error;
use types;
use header::Header;

use std::io::BufWriter;
use std::io::{Read, Write};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Default, Debug, Clone)]
pub struct SegmentHeader {
    pub phtype: types::SegmentType,
    pub flags: types::SegmentFlags,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

impl SegmentHeader {
    pub fn entsize(eh: &Header) -> usize {
        match eh.ident_class {
            types::Class::Class64 => 4 + 4 + 6 * 8,
            types::Class::Class32 => 4 + 4 + 6 * 4,
        }
    }

    pub fn from_reader<R>(io: &mut R, eh: &Header) -> Result<SegmentHeader, Error>
    where
        R: Read,
    {
        let mut r = SegmentHeader::default();
        let mut b = vec![0; eh.phentsize as usize];
        io.read_exact(&mut b)?;
        let mut br = &b[..];

        let reb = elf_read_u32!(eh, br)?;
        r.phtype = match types::SegmentType::from_u32(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSegmentType(reb)),
        };

        match eh.ident_class {
            types::Class::Class64 => {
                r.flags = types::SegmentFlags::from_bits_truncate(elf_read_u32!(eh, br)? as u64);
                r.offset = elf_read_u64!(eh, br)?;
                r.vaddr = elf_read_u64!(eh, br)?;
                r.paddr = elf_read_u64!(eh, br)?;
                r.filesz = elf_read_u64!(eh, br)?;
                r.memsz = elf_read_u64!(eh, br)?;
                r.align = elf_read_u64!(eh, br)?;
            }
            types::Class::Class32 => {
                r.offset = elf_read_u32!(eh, br)? as u64;
                r.vaddr = elf_read_u32!(eh, br)? as u64;
                r.paddr = elf_read_u32!(eh, br)? as u64;
                r.filesz = elf_read_u32!(eh, br)? as u64;
                r.memsz = elf_read_u32!(eh, br)? as u64;
                r.flags = types::SegmentFlags::from_bits_truncate(elf_read_u32!(eh, br)? as u64);
                r.align = elf_read_u32!(eh, br)? as u64;
            }
        };
        Ok(r)
    }
    pub fn to_writer<R>(&self, eh: &Header, io: &mut R) -> Result<(), Error>
    where
        R: Write,
    {
        let mut w = BufWriter::new(io);
        elf_write_u32!(eh, w, self.phtype.to_u32().unwrap())?;
        match eh.ident_class {
            types::Class::Class64 => {
                elf_write_u32!(eh, w, self.flags.bits() as u32)?;
                elf_write_u64!(eh, w, self.offset)?;
                elf_write_u64!(eh, w, self.vaddr)?;
                elf_write_u64!(eh, w, self.paddr)?;
                elf_write_u64!(eh, w, self.filesz)?;
                elf_write_u64!(eh, w, self.memsz)?;
                elf_write_u64!(eh, w, self.align)?;
            }
            types::Class::Class32 => {
                elf_write_u32!(eh, w, self.offset as u32)?;
                elf_write_u32!(eh, w, self.vaddr as u32)?;
                elf_write_u32!(eh, w, self.paddr as u32)?;
                elf_write_u32!(eh, w, self.filesz as u32)?;
                elf_write_u32!(eh, w, self.memsz as u32)?;
                elf_write_u32!(eh, w, self.flags.bits() as u32)?;
                elf_write_u32!(eh, w, self.align as u32)?;
            }
        };
        Ok(())
    }
}
