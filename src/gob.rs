use std::io;

use crate::*;

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

            entries.push(Entry { offset, length, raw_name });
        }

        Ok(Self { entries })
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> + '_ {
        self.entries.iter()
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
    raw_name: [u8; 13],
}

impl Entry {
    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn name(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.raw_name.to_vec())
            .map(|mut s| match s.find('\0') { None => s, Some(i) => { s.truncate(i); s } })
    }

    pub fn data<'file>(&self, gob_file: &'file mut File) -> impl io::Read + 'file {
        use std::io::Seek as _;
        gob_file.seek(io::SeekFrom::Start(self.offset as u64)).unwrap();
        io::Read::take(gob_file, self.length as u64)
    }
}
