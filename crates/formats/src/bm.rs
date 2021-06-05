use crate::common::*;
use std::io;

#[derive(Copy, Clone, Debug)]
pub enum Compression {
    None,
    Rle1,
    Rle0,
}

pub struct Bm {
    pub size: Vec2u16,
    pub idem_size: Vec2u16,
    pub flags: u8,
    pub log_size_y: bool,
    pub compression: Compression,
    pub data: Vec<u8>,
}

impl Bm {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
        if &read_buf(&mut file, [0u8; 4])? != b"BM \x1e" {
            return Err(ReadError::Signature);
        }

        let size = read_vec2_u16(&mut file)?;
        let idem_size = read_vec2_u16(&mut file)?;
        let flags = read_u8(&mut file)?;
        let log_size_y = read_u8(&mut file)? != 0;
        let compression = match read_u8(&mut file)? {
            0 => Compression::None,
            1 => Compression::Rle1,
            2 => Compression::Rle0,
            _ => return Err(ReadError::Decoding("invalid compression")),
        };
        read_u8(&mut file)?; // padding
        let data_size = read_u32(&mut file)?;
        file.seek(io::SeekFrom::Current(12))?;

        if size.x != 1 || size.y == 1 {
            let size_u32 = size.into_vec2::<u32>();
            let columns = match compression {
                Compression::None => {
                    let mut columns = vec![0u8; size.x as usize * size.y as usize];
                    file.read_exact(&mut columns)?;
                    columns
                }
                Compression::Rle1 => {
                    file.seek(io::SeekFrom::Start(32 + data_size as u64))?;
                    rle1(&mut file, 32, size_u32)?
                }
                Compression::Rle0 => {
                    file.seek(io::SeekFrom::Start(32 + data_size as u64))?;
                    rle0(&mut file, 32, size_u32)?
                }
            };

            let data = columns_to_rows(size_u32, columns);

            Ok(Bm {
                size,
                idem_size,
                flags,
                log_size_y,
                compression,
                data,
            })
        } else {
            eprintln!("multiple unimplemented");
            Ok(Bm {
                size: Vec2u16 { x: 1, y: 1 },
                idem_size,
                flags,
                log_size_y,
                compression,
                data: vec![1],
            })
        }
    }
}
