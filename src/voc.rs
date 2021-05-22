use crate::common::*;
use std::{fmt, io};

pub struct Voc {
    pub version: u16,
    pub chunks: Vec<Chunk>,
}

impl Voc {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
        if &read_buf(&mut file, [0u8; 0x16])? != b"Creative Voice File\x1a\x1a\0" {
            return Err(ReadError::Signature);
        }

        let version = read_u16(&mut file)?;
        let version_check = read_u16(&mut file)?;
        let expected_version_check = (!version).wrapping_add(0x1234);
        if expected_version_check != version_check {
            eprintln!(
                "VOC version check: {:04x} (expected {:04x})",
                version_check, expected_version_check
            );
            return Err(ReadError::Decoding("version check failed"));
        }

        let mut chunks = Vec::new();

        while let Some(chunk) = Chunk::read(&mut file)? {
            chunks.push(chunk);
        }

        Ok(Self { version, chunks })
    }
}

pub enum Chunk {
    SoundStart {
        sample_rate: SampleRate,
        codec: u8,
        data: Vec<u8>,
    },
    SoundContinue {
        data: Vec<u8>,
    },
    Silence {
        sample_count: u16,
        sample_rate: SampleRate,
    },
    Repeat {
        count: Option<u16>,
    },
    RepeatEnd,
    Unknown {
        ty: u8,
        len: u32,
    },
}

impl Chunk {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Option<Self>> {
        let ty = read_u8(&mut file)?;
        if ty == 0 {
            return Ok(None);
        }

        let len = u32::from_le_bytes([
            read_u8(&mut file)?,
            read_u8(&mut file)?,
            read_u8(&mut file)?,
            0,
        ]);

        let mut content = io::Cursor::new(read_vec(file, len as usize)?);

        Ok(Some(match ty {
            1 => {
                let sample_rate = SampleRate(read_u8(&mut content)?);
                let codec = read_u8(&mut content)?;
                let data = read_vec(&mut content, len as usize - 2)?;
                Self::SoundStart {
                    sample_rate,
                    codec,
                    data,
                }
            }
            2 => {
                let data = read_vec(&mut content, len as usize)?;
                Self::SoundContinue { data }
            }
            3 => {
                let sample_count = read_u16(&mut content)?;
                let sample_rate = SampleRate(read_u8(&mut content)?);
                Self::Silence {
                    sample_count,
                    sample_rate,
                }
            }
            6 => {
                let count = match read_u16(&mut content)? {
                    0xFFFF => None,
                    value => Some(value),
                };
                Self::Repeat { count }
            }
            7 => Self::RepeatEnd,
            _ => Self::Unknown { ty, len },
        }))
    }
}

//    encoded_sample_rate = 256 - (1_000_000 / true_sample_rate)
// => true_sample_rate = 1_000_000 / (256 - encoded_sample_rate)
// eg. 11025Hz, the only format that's supported by DF should be:
//   = 256 - (1_000_000 / 11025)
//   = 165 (rounded from 165.3...)
// and recovering:
//   = 1_000_000 / (256 - 165)
//   = 10_989 (off by ~36)
#[derive(Debug)]
pub struct SampleRate(u8);

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "~{}Hz", 1_000_000 / (256 - self.0 as u32))
    }
}
