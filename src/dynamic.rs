use std::io::{Read, Write};
use {Header, Error, SectionContent};
use types;
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone)]
pub enum DynamicContent {
    None,
    String(String),
    Address(u64),
}

#[derive(Debug, Clone)]
pub struct Dynamic {
    pub dhtype:  types::DynamicType,
    pub content: DynamicContent,
}

impl Dynamic {
    pub fn from_reader<R>(io: &mut R, strtab: Option<&str>,eh: &Header) -> Result<Vec<Dynamic>, Error> where R: Read {
        let mut r = Vec::new();

        while let Ok(tag) = elf_read_uclass!(eh, io) {
            let val = elf_read_uclass!(eh, io)?;

            match types::DynamicType::from_u64(tag) {
                None => return Err(Error::InvalidDynamicType(tag)),
                Some(types::DynamicType::NULL) => {
                    r.push(Dynamic{
                        dhtype:  types::DynamicType::NULL,
                        content: DynamicContent::None,
                    });
                    break;
                },
                Some(types::DynamicType::NEEDED) => {
                    r.push(Dynamic{
                        dhtype:  types::DynamicType::NEEDED,
                        content: DynamicContent::String(match strtab {
                            None => String::default(),
                            Some(s) => s[val as usize ..].split('\0').next().unwrap_or("").to_owned()
                        }),
                    });
                },
                Some(x) => {
                    r.push(Dynamic{
                        dhtype:  x,
                        content: DynamicContent::Address(val),
                    });
                },
            };
        }

        Ok(r)
    }
    pub fn to_writer<R>(&self, io: &mut R, eh: &Header) -> Result<(), Error> where R: Write {

        elf_write_uclass!(eh, io, self.dhtype.to_u64().unwrap())?;

        match self.content {
            DynamicContent::None => {elf_write_uclass!(eh, io, 0)?;},
            DynamicContent::String(ref s) => {elf_write_uclass!(eh, io, 1/*FIXME*/)?;},
            DynamicContent::Address(ref v) => {elf_write_uclass!(eh, io, *v)?;},
        }
        Ok(())
    }
}
