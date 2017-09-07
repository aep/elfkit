use std::io::{Read};
use {Error, Elf};
use types;
use num_traits::FromPrimitive;

#[derive(Debug, Default)]
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
        e: &Elf,
        _name:   u32,
        info:   u8,
        other:  u8,
        shndx:  u16,
        value:  u64,
        size:   u64,
        ) -> Result<Symbol, Error> {
        let name  = e.strtab[_name as usize ..].split('\0').next().unwrap_or("").to_owned();

        let stype = match types::SymbolType::from_u8(info & 0xf) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
        };

        let bind = match types::SymbolBind::from_u8(info >> 4) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
        };

        let vis = match types::SymbolVis::from_u8(other & 0x3) {
            Some(v) => v,
            None => return Err(Error::UnsupportedFormat),
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
    pub fn from_reader<R>(io: &mut R, e: &Elf) -> Result<Vec<Symbol>, Error> where R: Read {
        let eh = &e.header;
        let mut r = Vec::new();

        let mut b = vec![0; match e.header.ident_class {
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

                    Symbol::from_val(e, _name, info, other, shndx, value, size)?
                },
                types::Class::Class32 => {
                    let value = elf_read_u32!(eh, br)?;
                    let size  = elf_read_u32!(eh, br)?;
                    let info  = b[12];
                    let other = b[13];
                    br = &b[14..];
                    let shndx = elf_read_u16!(eh, br)?;

                    Symbol::from_val(e, _name, info, other, shndx, value as u64, size as u64)?
                },
            })
        }

        Ok(r)
    }
}
