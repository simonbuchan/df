use std::io;

use crate::*;
use std::ops::Index;

pub struct Catalog {
    entries: Vec<Entry>,
}

impl Catalog {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
        if read_buf(&mut file, &mut [0u8; 4])? != b"GOB\n" {
            return Err(ReadError::Signature)
        }

        let catalog_offset = read_u32(&mut file)?;
        file.seek(io::SeekFrom::Start(catalog_offset as u64))?;
        let num_entries = read_u32(&mut file)?;

        let mut entries = Vec::new();

        for _ in 0..num_entries {
            let offset = read_u32(&mut file)?;
            let length = read_u32(&mut file)?;
            let raw_name = read_buf(&mut file, [0u8; 13])?;
            let mut name = String::from_utf8(raw_name.to_vec())
                .expect("name to be ascii");
            if let Some(index) = name.find('\0') {
                name.truncate(index);
            }

            entries.push(Entry { offset, length, name });
        }

        Ok(Self { entries })
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        self.entries.iter()
    }
}

impl Index<usize> for Catalog {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IntoIterator for Catalog {
    type Item = Entry;
    type IntoIter = CatalogIter;

    fn into_iter(self) -> Self::IntoIter {
        CatalogIter(self.entries.into_iter())
    }
}

pub struct CatalogIter(std::vec::IntoIter<Entry>);

impl Iterator for CatalogIter {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

pub struct Entry {
    offset: u32,
    length: u32,
    name: String,
}

impl Entry {
    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data<'file>(&self, gob_file: &'file mut File) -> impl io::Read + 'file {
        use std::io::Seek as _;
        gob_file.seek(io::SeekFrom::Start(self.offset as u64)).unwrap();
        io::Read::take(gob_file, self.length as u64)
    }
}
