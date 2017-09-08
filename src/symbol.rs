use std::io::{Read};
use {Error, Header, types};
use num_traits::FromPrimitive;

#[derive(Debug, Default, Clone)]
pub struct Symbol {
    pub shndx:  u16,
    pub value:  u64,
    pub size:   u64,

    pub name:   String,
    pub stype:  types::SymbolType,
    pub bind:   types::SymbolBind,
    pub vis:    types::SymbolVis,
}

impl Symbol {
    pub fn from_val(
        tab: Option<&str>,
        _name:   u32,
        info:   u8,
        other:  u8,
        shndx:  u16,
        value:  u64,
        size:   u64,
        ) -> Result<Symbol, Error> {

        let name  = match tab {
            Some(s) => s[_name as usize ..].split('\0').next().unwrap_or("").to_owned(),
            None    => String::default(),
        };

        let reb = info & 0xf;
        let stype = match types::SymbolType::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolType(reb)),
        };

        let reb = info >> 4;
        let bind = match types::SymbolBind::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolBind(reb)),
        };

        let reb = info & 0x3;
        let vis = match types::SymbolVis::from_u8(reb) {
            Some(v) => v,
            None => return Err(Error::InvalidSymbolVis(reb)),
        };

        Ok(Symbol {
            shndx: shndx,
            value: value,
            size:  size,

            name: name,
            stype: stype,
            bind:  bind,
            vis:   vis,
        })
    }
    pub fn from_reader<R>(io: &mut R, tab: Option<&str>, eh: &Header) -> Result<Vec<Symbol>, Error> where R: Read {
        let mut r = Vec::new();

        let mut b = vec![0; match eh.ident_class {
            types::Class::Class64 => 24,
            types::Class::Class32 => 16,
        }];
        while io.read(&mut b)? > 0 {
            let mut br = &b[..];
            let _name  = elf_read_u32!(eh, br)?;

            r.push(match eh.ident_class {
                types::Class::Class64 => {
                    let info  = b[4];
                    let other = b[5];
                    br = &b[6..];
                    let shndx = elf_read_u16!(eh, br)?;
                    let value = elf_read_u64!(eh, br)?;
                    let size  = elf_read_u64!(eh, br)?;

                    Symbol::from_val(tab, _name, info, other, shndx, value, size)?
                },
                types::Class::Class32 => {
                    let value = elf_read_u32!(eh, br)?;
                    let size  = elf_read_u32!(eh, br)?;
                    let info  = b[12];
                    let other = b[13];
                    br = &b[14..];
                    let shndx = elf_read_u16!(eh, br)?;

                    Symbol::from_val(tab, _name, info, other, shndx, value as u64, size as u64)?
                },
            })
        }

        Ok(r)
    }
}
