use std::io;

use crate::common::*;

pub struct Fme {
    pub frame: Frame,
    pub cell: Cell,
}

impl Fme {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
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
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
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
    pub fn read(mut file: impl io::Read + io::Seek, offset: u32) -> ReadResult<Self> {
        file.seek(io::SeekFrom::Start(offset as u64))?;

        let size = read_vec2_u32(&mut file)?;
        let compressed = read_u32(&mut file)? != 0;
        /*let data_size = */
        read_u32(&mut file)?;
        let data_offset = read_i32(&mut file)?;
        read_u32(&mut file)?; // padding

        assert_eq!(data_offset, 0);
        // file.seek(io::SeekFrom::Current(data_offset as i64))?;

        let columns = if !compressed {
            read_vec(&mut file, (size.x * size.y) as usize)?
        } else {
            rle0(&mut file, offset, size)?
        };

        let data = columns_to_rows(size, columns);

        Ok(Self { size, data })
    }
}
