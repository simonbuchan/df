use std::io;

use crate::common::*;

pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Catalog> {
    if read_buf(&mut file, &mut [0u8; 4])? != b"GOB\n" {
        return Err(ReadError::Signature);
    }

    let catalog_offset = read_u32(&mut file)?;
    file.seek(io::SeekFrom::Start(catalog_offset as u64))?;
    let num_entries = read_u32(&mut file)?;

    let mut entries = Vec::new();

    for _ in 0..num_entries {
        let offset = read_u32(&mut file)?;
        let length = read_u32(&mut file)?;
        let raw_name = read_buf(&mut file, [0u8; 13])?;
        let mut name = String::from_utf8(raw_name.to_vec()).expect("name to be ascii");
        if let Some(index) = name.find('\0') {
            name.truncate(index);
        }

        entries.push(CatalogEntry {
            name,
            offset,
            length,
        });
    }

    Ok(Catalog { entries })
}
