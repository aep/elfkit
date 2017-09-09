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
    pub fn from_reader<R>(mut io: R, linked: Option<&SectionContent>, eh: &Header) -> Result<SectionContent, Error> where R: Read{

        let strtab = match linked {
            None => None,
            Some(&SectionContent::Raw(ref s)) => Some(s),
            _ => return Err(Error::LinkedSectionIsNotStrings),
        };

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
                            Some(s) => String::from_utf8_lossy(
                                s[val as usize ..].split(|c|*c==0).next().unwrap_or(&[0;0])).into_owned(),
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

        Ok(SectionContent::Dynamic(r))
    }
    pub fn to_writer<W>(&self, mut io: W, linked: Option<&mut SectionContent>, eh: &Header)
        -> Result<(), Error> where W: Write {
        elf_write_uclass!(eh, io, self.dhtype.to_u64().unwrap())?;

        match self.content {
            DynamicContent::None => {elf_write_uclass!(eh, io, 0)?;},
            DynamicContent::String(ref s) => {
                match linked {
                    Some(&mut SectionContent::Raw(ref mut dynstr)) => {
                        let off = dynstr.len() as u64;
                        dynstr.extend(s.bytes());
                        dynstr.extend(&[0;1]);
                        elf_write_uclass!(eh, io, off)?;
                    },
                    _ => elf_write_uclass!(eh, io, 0)?,
                }
            },
            DynamicContent::Address(ref v) => {elf_write_uclass!(eh, io, *v)?;},
        }
        Ok(())
    }
}
