use std::io::{Read, Write};
use {Error, Header, SectionContent};
use std::collections::hash_map::{Entry, HashMap};

#[derive(Debug, Default, Clone)]
pub struct Strtab {
    hash: Option<HashMap<Vec<u8>, usize>>,
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
        let mut data = Vec::new();
        io.read_to_end(&mut data)?;
        Ok(SectionContent::Strtab(Strtab{
            hash: None,
            data: data,
        }))
    }

    pub fn to_writer<W>(
        &self,
        mut io: W,
        _: &Header,
    ) -> Result<(usize), Error>
    where
        W: Write,
    {
        io.write(&self.data)?;
        Ok(self.data.len())
    }

    pub fn get(&self, i: usize) -> Vec<u8> {
        if i >= self.data.len() {
            println!("pointer {} into strtab extends beyond section size", i);
            return b"<corrupt>".to_vec();
        }
        self.data[i..].split(|c| *c == 0).next().unwrap_or(&[0; 0]).to_vec()
    }

    pub fn insert(&mut self, ns: &[u8]) -> usize {

        //special handling for null. for some reason rusts hashmap doesn't do that correctly
        if self.data.len() < 1 {
            self.data.push(0)
        }
        if ns.len() == 0 {
            return 0;
        }

        //TODO this is less efficient than just scanning data, so it's kinda pointless
        if self.hash == None {
            let mut hash = HashMap::new();

            let mut n = Vec::new();
            let mut start = 0;
            for i in 0..self.data.len() {
                let c = self.data[i];
                if c == 0 {
                    for x in 0..self.data.len() {
                        hash.insert(n[x..].to_vec(), start + x);
                    }
                    start = i + 1;
                    n = Vec::new()
                } else {
                    n.push(c);
                }
            }
            self.hash = Some(hash);
        }

        match self.hash.as_mut().unwrap().entry(ns.to_vec()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let i = self.data.len();
                self.data.extend(ns);
                self.data.extend(&[0; 1]);
                entry.insert(i);
                i
            }
        }
    }
}
