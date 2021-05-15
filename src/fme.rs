use std::io;

use crate::common::*;

pub struct Fme {
    pub frame: Frame,
    pub cell: Cell,
}

impl Fme {
    pub fn read(mut file: impl io::Read + io::Seek) -> io::Result<Self> {
        let frame = Frame::read(&mut file)?;
        let cell_offset = read_u32(&mut file)?;
        let cell = Cell::read(&mut file, cell_offset)?;
        Ok(Self { frame, cell })
    }
}

#[derive(Copy, Clone)]
pub struct Frame {
    pub offset: Vec2i32,
    pub flip: bool,
}

impl Frame {
    pub fn read(mut file: impl io::Read + io::Seek) -> io::Result<Self> {
        let offset = read_vec2_i32(&mut file)?;
        let flip = read_u32(&mut file)? != 0;
        Ok(Self { offset, flip })
    }
}

pub struct Cell {
    pub size: Vec2u32,
    pub data: Vec<u8>,
}

impl Cell {
    pub fn read(mut file: impl io::Read + io::Seek, offset: u32) -> io::Result<Self> {
        file.seek(io::SeekFrom::Start(offset as u64))?;

        let size = read_vec2_u32(&mut file)?;
        let compressed = read_u32(&mut file)? != 0;
        /*let data_size = */
        read_u32(&mut file)?;
        let data_offset = read_i32(&mut file)?;
        read_u32(&mut file)?; // padding

        assert_eq!(data_offset, 0);
        // file.seek(io::SeekFrom::Current(data_offset as i64))?;

        let unpacked_data_size = size.x as usize * size.y as usize;
        let mut columns = Vec::with_capacity(unpacked_data_size);
        if !compressed {
            columns.resize(unpacked_data_size, 0u8);
            file.read_exact(&mut columns)?;
        } else {
            let mut column_offsets = Vec::new();

            for _ in 0..size.x {
                column_offsets.push(offset + read_u32(&mut file)?);
            }

            // rle0::decompress(file, height, column_offsets)
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

            assert_eq!(columns.len(), (size.x * size.y) as usize);
        }

        // data is in columns, bottom to top, not rows. Transpose it.
        let mut data = Vec::with_capacity(columns.len());
        for y in 0..size.y as usize {
            for x in 0..size.x as usize {
                data.push(columns[x * size.y as usize + size.y as usize - y - 1]);
            }
        }

        Ok(Self { size, data })
    }
}
