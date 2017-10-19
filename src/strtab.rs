use std::io::{Read, Write};
use {Error, Header, SectionContent};
use std::collections::hash_map::{Entry, HashMap};

#[derive(Debug, Default, Clone)]
pub struct Strtab {
    hash: HashMap<Vec<u8>, usize>,
    data: Vec<u8>,
}

impl Strtab {
    pub fn len(&self, _: &Header) -> usize {
        self.data.len()
    }
    pub fn entsize(_: &Header) -> usize {
        1
    }
    pub fn from_reader<R>(
        mut io: R,
        _: Option<&SectionContent>,
        _: &Header,
    ) -> Result<SectionContent, Error>
    where
        R: Read,
    {
        let mut r = Strtab::default();

        io.read_to_end(&mut r.data)?;

        let mut n = Vec::new();
        let mut start = 0;
        for i in 0..r.data.len() {
            let c = r.data[i];
            if c == 0 {
                for x in 0..n.len() {
                    r.hash.insert(n[x..].to_vec(), start + x);
                }
                start = i + 1;
                n = Vec::new()
            } else {
                n.push(c);
            }
        }

        Ok(SectionContent::Strtab(r))
    }

    pub fn to_writer<W>(
        &self,
        mut io: W,
        _: Option<&mut SectionContent>,
        _: &Header,
    ) -> Result<(), Error>
    where
        W: Write,
    {
        io.write(&self.data)?;
        Ok(())
    }

    pub fn get(&self, i: usize) -> String {
        if i >= self.data.len() {
            println!("pointer {} into strtab extends beyond section size", i);
            return String::from("<corrupt>");
        }
        String::from_utf8_lossy(self.data[i..].split(|c| *c == 0).next().unwrap_or(&[0; 0]))
            .into_owned()
    }

    pub fn insert(&mut self, ns: Vec<u8>) -> usize {
        //special handling for null. for some reason rusts hashmap doesn't do that correctly
        if self.data.len() < 1 {
            self.data.push(0)
        }
        if ns.len() == 0 {
            return 0;
        }

        match self.hash.entry(ns.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let i = self.data.len();
                self.data.extend(&ns);
                self.data.extend(&[0; 1]);
                entry.insert(i);
                i
            }
        }
    }
}
