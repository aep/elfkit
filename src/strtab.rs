use std::io::{Read, Write};
use {Error, Header, SectionContent};

#[derive(Debug, Default, Clone)]
pub struct Strtab {
    data: Vec<u8>,
}

impl Strtab {
    pub fn len(&self, _: &Header) -> usize {
        self.data.len()
    }
    pub fn entsize(_: &Header) -> usize {
        1
    }
    pub fn from_reader(
        mut io: impl Read,
        _: Option<&SectionContent>,
        _: &Header,
    ) -> Result<SectionContent, Error> {
        let mut data = Vec::new();
        io.read_to_end(&mut data)?;
        Ok(SectionContent::Strtab(Strtab { data }))
    }

    pub fn to_writer(&self, mut io: impl Write, _: &Header) -> Result<usize, Error> {
        Ok(io.write(&self.data)?)
    }

    pub fn get(&self, i: usize) -> Vec<u8> {
        if i >= self.data.len() {
            println!("pointer {} into strtab extends beyond section size", i);
            return b"<corrupt>".to_vec();
        }
        self.data[i..]
            .split(|&c| c == 0)
            .next()
            .unwrap_or(&[0; 0])
            .to_vec()
    }

    pub fn insert(&mut self, ns: &[u8]) -> usize {
        // If ns is already in our data, it takes up (ns.len() + 1) bytes.
        if let Some(max_start) = self.data.len().checked_sub(ns.len() + 1) {
            for start in 0..=max_start {
                // We first check for ns, then check for the null terminator.
                if self.data[start..].starts_with(ns) {
                    if self.data[start + ns.len()] == 0 {
                        return start;
                    }
                }
            }
        }

        // No spot for ns, insert it (and the null terminator) at the end.
        let i = self.data.len();
        self.data.extend(ns);
        self.data.push(0);
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_then_get() {
        let ns1: &[u8] = b".text";
        let ns2: &[u8] = b".data";
        let mut strtab = Strtab { data: Vec::new() };

        assert_eq!(strtab.insert(&[]), 0);
        assert_eq!(strtab.insert(ns1), 1);
        assert_eq!(strtab.insert(ns2), 2 + ns1.len());

        assert_eq!(strtab.get(0), &[]);
        assert_eq!(strtab.get(1), ns1);
        assert_eq!(strtab.get(2 + ns1.len()), ns2);
    }

    #[test]
    fn test_insert_suffix() {
        let mut strtab = Strtab { data: Vec::new() };
        assert_eq!(strtab.insert(b".text"), 0);

        // inserting an existing suffix should not grow the data size
        let old_size = strtab.data.len();
        assert_eq!(strtab.insert(b"xt"), 3);
        assert_eq!(strtab.data.len(), old_size);
    }

    #[test]
    fn test_starting_data() {
        let ns = b".text";
        // Have the data initially be [NULL, ".text"]
        let mut data = vec![0];
        data.extend(ns);
        data.push(0);

        let mut strtab = Strtab { data };
        assert_eq!(strtab.get(1), ns);
        assert_eq!(strtab.insert(b".data"), 2 + ns.len());
    }

    #[test]
    fn test_only_data() {
        let ns: &[u8] = b".text";
        // Have the data initially just be ".text"
        let mut data = vec![];
        data.extend(ns);
        data.push(0);
        let mut strtab = Strtab { data };
        // The only value should be ".text"
        assert_eq!(strtab.get(0), ns);
        // Inserting the value again should change nothing.
        assert_eq!(strtab.insert(ns), 0);
        assert_eq!(strtab.get(0), ns);
    }

    #[test]
    fn test_insert_prefix() {
        // Have the data initially just be ".text"
        let mut data = vec![];
        data.extend(b".text");
        data.push(0);

        let mut strtab = Strtab { data };
        // Inserting a prefix should not save any space
        assert_eq!(strtab.insert(b".tex"), 6);
    }
}
