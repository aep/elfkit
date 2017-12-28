use std::io::{Read, Write};
use {Error, Header, SectionContent};
use types;
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone)]
pub enum DynamicContent {
    None,
    String((Vec<u8>,Option<u64>)),
    Address(u64),
    Flags1(types::DynamicFlags1),
}

impl Default for DynamicContent{
    fn default() -> Self {
        DynamicContent::None
    }
}

#[derive(Debug, Clone, Default)]
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
            any => return Err(Error::LinkedSectionIsNotStrtab{
                during: "reading dynamic",
                link: any.map(|v|v.clone()),
            }),
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
                },
                Some(types::DynamicType::RPATH) => {
                    r.push(Dynamic {
                        dhtype: types::DynamicType::RPATH,
                        content: DynamicContent::String(match strtab {
                            None => (Vec::default(),None),
                            Some(s) => (s.get(val as usize), Some(val)),
                        }),
                    });
                },
                Some(types::DynamicType::NEEDED) => {
                    r.push(Dynamic {
                        dhtype: types::DynamicType::NEEDED,
                        content: DynamicContent::String(match strtab {
                            None => (Vec::default(),None),
                            Some(s) => (s.get(val as usize), Some(val)),
                        }),
                    });
                },
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
                },
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
        eh: &Header,
    ) -> Result<(usize), Error>
    where
        W: Write,
    {
        elf_write_uclass!(eh, io, self.dhtype.to_u64().unwrap())?;

        match self.content {
            DynamicContent::None => {
                elf_write_uclass!(eh, io, 0)?;
            }
            DynamicContent::String(ref s) => match s.1 {
                Some(val) => elf_write_uclass!(eh, io, val)?,
                None      => return Err(Error::WritingNotSynced),
            },
            DynamicContent::Address(ref v) => {
                elf_write_uclass!(eh, io, *v)?;
            }
            DynamicContent::Flags1(ref v) => {
                elf_write_uclass!(eh, io, v.bits())?;
            }
        }
        Ok(Dynamic::entsize(eh))
    }

    pub fn sync(&mut self, linked: Option<&mut SectionContent>, _: &Header) -> Result<(), Error> {
        match self.content {
            DynamicContent::String(ref mut s) => match linked {
                Some(&mut SectionContent::Strtab(ref mut strtab)) => {
                    s.1 = Some(strtab.insert(&s.0) as u64);
                }
                any => return Err(Error::LinkedSectionIsNotStrtab{
                    during: "syncing dynamic",
                    link: any.map(|v|v.clone()),
                }),
            },
            DynamicContent::None => {}
            DynamicContent::Address(_) => {}
            DynamicContent::Flags1(_) => {}
        }
        Ok(())
    }
}
