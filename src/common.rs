pub use crate::error::*;
use std::io;

#[derive(Copy, Clone, Default, Debug)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

pub type Vec2i32 = Vec2<i32>;
pub type Vec2u32 = Vec2<u32>;

impl From<Vec2u32> for (usize, usize) {
    fn from(value: Vec2u32) -> Self {
        (value.x as usize, value.y as usize)
    }
}

impl From<Vec2i32> for eframe::egui::Vec2 {
    fn from(value: Vec2i32) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}

impl From<Vec2u32> for eframe::egui::Vec2 {
    fn from(value: Vec2u32) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
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
) -> io::Result<Vec2<T>> {
    let x = read(&mut input)?;
    let y = read(&mut input)?;
    Ok(Vec2 { x, y })
}

pub fn read_vec2_i32(input: impl io::Read) -> io::Result<Vec2i32> {
    read_vec2(input, |r| read_i32(r))
}

pub fn read_vec2_u32(input: impl io::Read) -> io::Result<Vec2u32> {
    read_vec2(input, |r| read_u32(r))
}

pub fn read_buf<T: AsMut<[u8]>>(mut input: impl io::Read, mut buffer: T) -> io::Result<T> {
    input.read_exact(buffer.as_mut())?;
    Ok(buffer)
}
