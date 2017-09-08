use std::io::{Read, Write};
use {Elf, Error, SectionContent};
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
    pub fn from_reader<R>(io: &mut R, elf: &Elf) -> Result<Vec<Dynamic>, Error> where R: Read {
        let mut r = Vec::new();

        //FIXME look up by link rather than name
        let strtab: Option<&str> = elf.get_section_by_name(".dynstr").map(|s| {
            match s.content {
                SectionContent::Strings(ref s) => s.as_ref(),
                _ => unreachable!()
            }
        });

        while let Ok(tag) = elf_read_uclass!(elf.header, io) {
            let val = elf_read_uclass!(elf.header, io)?;

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
    pub fn to_writer<R>(&self, io: &mut R, elf: &Elf) -> Result<(), Error> where R: Write {

        elf_write_uclass!(elf.header, io, self.dhtype.to_u64().unwrap())?;

        match self.content {
            DynamicContent::None => {elf_write_uclass!(elf.header, io, 0)?;},
            DynamicContent::String(ref s) => {elf_write_uclass!(elf.header, io, 1/*FIXME*/)?;},
            DynamicContent::Address(ref v) => {elf_write_uclass!(elf.header, io, *v)?;},
        }
        Ok(())
    }
}
