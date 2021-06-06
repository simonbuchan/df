pub use crate::error::*;
use std::io;

pub fn read_buf<T: AsMut<[u8]>>(mut input: impl io::Read, mut buffer: T) -> io::Result<T> {
    input.read_exact(buffer.as_mut())?;
    Ok(buffer)
}

pub fn read_vec(input: impl io::Read, len: usize) -> io::Result<Vec<u8>> {
    read_buf(input, vec![0u8; len])
}

pub fn read_u8(input: impl io::Read) -> io::Result<u8> {
    read_buf(input, [0u8; 1]).map(u8::from_le_bytes)
}

pub fn read_u16(input: impl io::Read) -> io::Result<u16> {
    read_buf(input, [0u8; 2]).map(u16::from_le_bytes)
}

pub fn read_i32(input: impl io::Read) -> io::Result<i32> {
    read_buf(input, [0u8; 4]).map(i32::from_le_bytes)
}

pub fn read_u32(input: impl io::Read) -> io::Result<u32> {
    read_buf(input, [0u8; 4]).map(u32::from_le_bytes)
}

pub fn read_vec2<R: io::Read, T>(
    mut input: R,
    read: impl Fn(&mut R) -> io::Result<T>,
) -> io::Result<mint::Vector2<T>> {
    let x = read(&mut input)?;
    let y = read(&mut input)?;
    Ok(mint::Vector2 { x, y })
}

pub type Vec2<T> = mint::Vector2<T>;

macro_rules! vec2_subtype {
    ($p: ty, $read_vec: ident, $read_p: ident) => {
        pub fn $read_vec(input: impl io::Read) -> io::Result<mint::Vector2<$p>> {
            read_vec2(input, |r| $read_p(r))
        }
    };
}

vec2_subtype!(u16, read_vec2_u16, read_u16);
vec2_subtype!(i32, read_vec2_i32, read_i32);
vec2_subtype!(u32, read_vec2_u32, read_u32);

pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

pub struct CatalogEntry {
    pub name: String,
    pub offset: u32,
    pub length: u32,
}

pub fn rle0(
    mut file: impl io::Read + io::Seek,
    offset: u32,
    size: mint::Vector2<u32>,
) -> ReadResult<Vec<u8>> {
    let mut columns = Vec::with_capacity((size.x * size.y) as usize);
    let mut column_offsets = Vec::new();

    for _ in 0..size.x {
        column_offsets.push(offset + read_u32(&mut file)?);
    }

    let mut buffer = [0u8; 128];
    for offset in column_offsets {
        file.seek(io::SeekFrom::Start(offset as u64))?;
        let mut unpacked_bytes = 0;
        while unpacked_bytes < size.y {
            let mut control_byte = 0u8;
            file.read_exact(std::slice::from_mut(&mut control_byte))?;
            if control_byte <= 128 {
                let column = read_buf(&mut file, &mut buffer[0..control_byte as usize])?;
                columns.extend_from_slice(column);
            } else {
                control_byte -= 128;
                for _ in 0..control_byte {
                    columns.push(0);
                }
            }
            unpacked_bytes += control_byte as u32;
        }
    }

    if columns.len() != (size.x * size.y) as usize {
        return Err(ReadError::Decoding("RLE0 decoded size did not match"));
    }

    Ok(columns)
}

pub fn rle1(
    mut file: impl io::Read + io::Seek,
    offset: u32,
    size: mint::Vector2<u32>,
) -> ReadResult<Vec<u8>> {
    let mut columns = Vec::with_capacity((size.x * size.y) as usize);
    let mut column_offsets = Vec::new();

    for _ in 0..size.x {
        column_offsets.push(offset + read_u32(&mut file)?);
    }

    let mut buffer = [0u8; 128];
    for offset in column_offsets {
        file.seek(io::SeekFrom::Start(offset as u64))?;
        let mut unpacked_bytes = 0;
        while unpacked_bytes < size.y {
            let mut control_byte = 0u8;
            file.read_exact(std::slice::from_mut(&mut control_byte))?;
            if control_byte < 128 {
                let column = read_buf(&mut file, &mut buffer[0..control_byte as usize])?;
                columns.extend_from_slice(column);
            } else {
                let data_byte = read_u8(&mut file)?;
                control_byte -= 128; // including 0 bytes
                for _ in 0..control_byte {
                    columns.push(data_byte);
                }
            }
            unpacked_bytes += control_byte as u32;
        }
    }

    if columns.len() != (size.x * size.y) as usize {
        return Err(ReadError::Decoding("RLE1 decoded size did not match"));
    }

    Ok(columns)
}

pub fn columns_to_rows(size: mint::Vector2<u32>, columns: Vec<u8>) -> Vec<u8> {
    assert_eq!((size.x * size.y) as usize, columns.len());
    // data is in columns, bottom to top, not rows. Transpose it.
    let mut data = Vec::with_capacity(columns.len());
    for y in 0..size.y as usize {
        for x in 0..size.x as usize {
            data.push(columns[x * size.y as usize + size.y as usize - y - 1]);
        }
    }
    data
}
