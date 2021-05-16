use std::io;

use crate::common::*;
use crate::error::ReadResult;

pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Catalog> {
    let mut offset = 0u32;

    let mut entries = Vec::new();

    loop {
        let mut ty = [0u8; 4];
        match file.read(&mut ty)? {
            0 => break,
            4 => {}
            _ => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
        }

        let raw_name = read_buf(&mut file, [0u8; 8])?;

        let length = read_u32(&mut file)?;

        let ty = String::from_utf8(ty.to_vec()).unwrap();

        let mut name =
            String::from_utf8(raw_name.split(|&c| c == 0u8).next().unwrap().to_vec()).unwrap();

        name.push('.');
        name.push_str(&ty);

        entries.push(CatalogEntry {
            name,
            length,
            offset,
        });

        file.seek(io::SeekFrom::Current(length as i64))?;

        offset += 4 + 8 + 4 + length;
    }

    Ok(Catalog { entries })
}
