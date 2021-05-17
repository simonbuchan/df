use crate::common::read_buf;
use std::io;

pub struct Pal {
    pub entries: [Entry; 256],
}

impl Pal {
    pub fn read(file: impl io::Read) -> io::Result<Self> {
        let bytes = read_buf(file, [0u8; 256 * 3])?;
        Ok(Self {
            entries: unsafe { std::mem::transmute(bytes) },
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Entry {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Entry {
    pub const BLACK: Entry = Entry { r: 0, g: 0, b: 0 };
}

impl From<Entry> for eframe::egui::Color32 {
    fn from(value: Entry) -> Self {
        Self::from_rgb(
            channel_6_to_8_bit(value.r),
            channel_6_to_8_bit(value.g),
            channel_6_to_8_bit(value.b),
        )
    }
}

fn channel_6_to_8_bit(value: u8) -> u8 {
    value << 2 | value >> 4
}
