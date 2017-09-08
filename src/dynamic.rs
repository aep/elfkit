use std::io::{Read};
use {Header, Error};
use types;
use num_traits::FromPrimitive;

#[derive(Debug, Clone)]
pub struct Dynamic {
    pub dhtype: types::DynamicType,
    pub val:    u64,
}

impl Dynamic {
    pub fn from_reader<R>(io: &mut R, tab: Option<&str>, eh: &Header) -> Result<Vec<Dynamic>, Error> where R: Read {
        let mut r = Vec::new();

        while let Ok(tag) = elf_read_uclass!(eh, io) {
            let val = elf_read_uclass!(eh, io)?;
            r.push(Dynamic{
                dhtype: match types::DynamicType::from_u64(tag) {
                    Some(v) => v,
                    None => return Err(Error::InvalidDynamicType(tag)),
                },
                val: val,
            });
        }

        Ok(r)
    }
}
