use std::io;

use super::*;

pub struct Fme {
    pub offset: (i32, i32),
    pub flip: bool,
    pub unit_size: (u32, u32),
    pub size: (u32, u32),
    pub data: Vec<u8>,
}

impl Fme {
    pub fn read(mut file: impl io::Read + io::Seek) -> io::Result<Self> {
        let x_offset = read_i32(&mut file)?;
        let y_offset = read_i32(&mut file)?;
        let flip = read_u32(&mut file)? != 0;
        let data_properties_offset = read_u32(&mut file)?;
        let unit_width = read_u32(&mut file)?;
        let unit_height = read_u32(&mut file)?;
        // read_u32(&mut file)?; // padding
        // read_u32(&mut file)?; // padding

        file.seek(io::SeekFrom::Start(data_properties_offset as u64))?;

        let width = read_u32(&mut file)?;
        let height = read_u32(&mut file)?;
        let compressed = read_u32(&mut file)? != 0;
        let data_size = read_u32(&mut file)?;
        let data_offset = read_i32(&mut file)?;
        read_u32(&mut file)?; // padding

        file.seek(io::SeekFrom::Current(data_offset as i64))?;

        let mut columns = Vec::with_capacity(data_size as usize);
        if !compressed {
            file.read_to_end(&mut columns)?;
        } else {
            let mut column_offsets = Vec::new();

            for _ in 0..width {
                column_offsets.push(data_properties_offset + read_u32(&mut file)?);
            }

            // rle0::decompress(file, height, column_offsets)
            let mut buffer = [0u8; 128];
            for offset in column_offsets {
                file.seek(io::SeekFrom::Start(offset as u64))?;
                let mut unpacked_bytes = 0;
                while unpacked_bytes < height {
                    let mut control_byte = 0u8;
                    file.read_exact(std::slice::from_mut(&mut control_byte))?;
                    if control_byte <= 128 {
                        columns.extend_from_slice(
                            read_buf(&mut file, &mut buffer[0..control_byte as usize])?,
                        );
                    } else {
                        control_byte -= 128;
                        for _ in 0..control_byte {
                            columns.push(0);
                        }
                    }
                    unpacked_bytes += control_byte as u32;
                }
            }
        }

        assert_eq!(columns.len(), (width * height) as usize);

        // data is in columns, bottom to top, not rows. Transpose it.
        let mut data = Vec::with_capacity(columns.len());
        for y in 0..height as usize {
            for x in 0..width as usize {
                data.push(columns[x * height as usize + height as usize - y - 1]);
            }
        }

        Ok(Self {
            offset: (x_offset, y_offset),
            flip,
            unit_size: (unit_width, unit_height),
            size: (width, height),
            data,
        })
    }
}
