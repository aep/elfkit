use std::io::{Read, Write};
use {Error, Header, SectionContent};
use types;
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone)]
pub enum DynamicContent {
    None,
    String(String),
    Address(u64),
    Flags1(types::DynamicFlags1),
}

#[derive(Debug, Clone)]
pub struct Dynamic {
    pub dhtype: types::DynamicType,
    pub content: DynamicContent,
}

impl Dynamic {
    pub fn entsize(eh: &Header) -> usize {
        match eh.ident_class {
            types::Class::Class64 => 16,
            types::Class::Class32 => 8,
        }
    }

    pub fn from_reader<R>(
        mut io: R,
        linked: Option<&SectionContent>,
        eh: &Header,
    ) -> Result<SectionContent, Error>
    where
        R: Read,
    {
        let strtab = match linked {
            None => None,
            Some(&SectionContent::Strtab(ref s)) => Some(s),
            _ => return Err(Error::LinkedSectionIsNotStrtab("reading dynamic")),
        };

        let mut r = Vec::new();

        while let Ok(tag) = elf_read_uclass!(eh, io) {
            let val = elf_read_uclass!(eh, io)?;

            match types::DynamicType::from_u64(tag) {
                None => return Err(Error::InvalidDynamicType(tag)),
                Some(types::DynamicType::NULL) => {
                    r.push(Dynamic {
                        dhtype: types::DynamicType::NULL,
                        content: DynamicContent::None,
                    });
                    break;
                }
                Some(types::DynamicType::NEEDED) => {
                    r.push(Dynamic {
                        dhtype: types::DynamicType::NEEDED,
                        content: DynamicContent::String(match strtab {
                            None => String::default(),
                            Some(s) => s.get(val as usize),
                        }),
                    });
                }
                Some(types::DynamicType::FLAGS_1) => {
                    r.push(Dynamic {
                        dhtype: types::DynamicType::FLAGS_1,
                        content: DynamicContent::Flags1(
                            match types::DynamicFlags1::from_bits(val) {
                                Some(v) => v,
                                None => return Err(Error::InvalidDynamicFlags1(val)),
                            },
                        ),
                    });
                }
                Some(x) => {
                    r.push(Dynamic {
                        dhtype: x,
                        content: DynamicContent::Address(val),
                    });
                }
            };
        }

        Ok(SectionContent::Dynamic(r))
    }
    pub fn to_writer<W>(
        &self,
        mut io: W,
        linked: Option<&mut SectionContent>,
        eh: &Header,
    ) -> Result<(), Error>
    where
        W: Write,
    {
        elf_write_uclass!(eh, io, self.dhtype.to_u64().unwrap())?;

        match self.content {
            DynamicContent::None => {
                elf_write_uclass!(eh, io, 0)?;
            }
            DynamicContent::String(ref s) => match linked {
                Some(&mut SectionContent::Strtab(ref mut dynstr)) => {
                    let off = dynstr.insert(s.bytes().collect()) as u64;
                    elf_write_uclass!(eh, io, off)?;
                }
                _ => elf_write_uclass!(eh, io, 0)?,
            },
            DynamicContent::Address(ref v) => {
                elf_write_uclass!(eh, io, *v)?;
            }
            DynamicContent::Flags1(ref v) => {
                elf_write_uclass!(eh, io, v.bits())?;
            }
        }
        Ok(())
    }

    pub fn sync(&self, linked: Option<&mut SectionContent>, eh: &Header) -> Result<(), Error> {
        match self.content {
            DynamicContent::String(ref s) => match linked {
                Some(&mut SectionContent::Strtab(ref mut strtab)) => {
                    let off = strtab.insert(s.bytes().collect()) as u32;
                }
                _ => return Err(Error::LinkedSectionIsNotStrtab("syncing dynamic")),
            },
            DynamicContent::None => {}
            DynamicContent::Address(_) => {}
            DynamicContent::Flags1(_) => {}
        }
        Ok(())
    }
}
