use std::fs::File;
use std::io;
use std::path::Path;

use wgpu::util::DeviceExt;

pub use level::Level;

use crate::context::Context;

pub type LoaderResult<T> = Result<T, LoaderError>;

mod level;

#[derive(Debug)]
pub enum LoaderError {
    NotFound(String),
    IO(io::Error),
    Read(formats::common::ReadError),
}

impl From<std::io::Error> for LoaderError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<formats::common::ReadError> for LoaderError {
    fn from(value: formats::common::ReadError) -> Self {
        Self::Read(value)
    }
}

pub struct Loader {
    dark: Gob,
    textures: Gob,
}

impl Loader {
    pub fn open(base_path: impl AsRef<Path>) -> LoaderResult<Self> {
        let base_path = base_path.as_ref();
        Ok(Self {
            dark: Gob::open(base_path.join("DARK.GOB"))?,
            textures: Gob::open(base_path.join("TEXTURES.GOB"))?,
        })
    }

    pub fn load_pal(&mut self, name: &str) -> LoaderResult<formats::pal::Pal> {
        let file = self.dark.entry(name)?;
        let pal = formats::pal::Pal::read(file)?;
        Ok(pal)
    }

    pub fn load_bm(
        &mut self,
        name: &str,
        pal: &formats::pal::Pal,
        context: &Context,
    ) -> LoaderResult<(cgmath::Vector2<u32>, wgpu::Texture)> {
        let file = self.textures.entry(name)?;

        let bm = formats::bm::Bm::read(file)?;

        let texels = bm
            .data
            .iter()
            .flat_map(|&i| {
                std::array::IntoIter::new({
                    if bm.flags & 8 == 0 && i == 0 {
                        [0, 0, 0, 0]
                    } else {
                        let (r, g, b) = pal.entries[i as usize].to_rgb();
                        [r, g, b, 0xFF]
                    }
                })
            })
            .collect::<Vec<u8>>();

        let texture = context.device.create_texture_with_data(
            &context.queue,
            &Self::texture_descriptor(name, bm.size.x as u32, bm.size.y as u32),
            &texels,
        );

        Ok((cgmath::Vector2::from(bm.size).cast().unwrap(), texture))
    }

    pub fn load_bm_or_default(
        &mut self,
        name: &str,
        pal: &formats::pal::Pal,
        context: &Context,
    ) -> (cgmath::Vector2<u32>, wgpu::Texture) {
        self.load_bm(name, pal, context).unwrap_or_else(|_| {
            (
                cgmath::vec2(1, 1),
                context.device.create_texture_with_data(
                    &context.queue,
                    &Loader::texture_descriptor("default_texture", 1, 1),
                    &vec![255, 0, 255, 255],
                ),
            )
        })
    }

    fn texture_descriptor(name: &str, width: u32, height: u32) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: Some(name),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        }
    }

    pub fn load_lev(&mut self, name: &str, context: &Context) -> LoaderResult<level::Level> {
        level::Level::load(self, name, context)
    }
}

pub struct Gob {
    file: File,
    catalog: formats::common::Catalog,
}

impl Gob {
    pub fn open(path: impl AsRef<Path>) -> LoaderResult<Self> {
        let mut file = File::open(path)?;
        let catalog = formats::gob::read(&mut file)?;
        Ok(Self { file, catalog })
    }

    pub fn entry<'a>(&'a mut self, name: &str) -> LoaderResult<impl io::Read + io::Seek + 'a> {
        let entry = self
            .catalog
            .entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| LoaderError::NotFound(name.to_string()))?;

        io::Seek::seek(&mut self.file, io::SeekFrom::Start(entry.offset as u64))?;

        // let mut data = vec![0u8; entry.length as usize];
        // std::io::Read::read_exact(&mut self.file, &mut data).unwrap();
        // Some(std::io::Cursor::new(data))
        Ok(GobRead {
            file: &mut self.file,
            pointer: 0,
            entry_offset: entry.offset,
            entry_length: entry.length,
        })
    }
}

struct GobRead<'a> {
    file: &'a mut File,
    pointer: u32,
    entry_offset: u32,
    entry_length: u32,
}

impl<'a> io::Read for GobRead<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pointer >= self.entry_length {
            return Ok(0);
        }
        let len = buf
            .len()
            .min(self.entry_length.saturating_sub(self.pointer) as usize);
        // dbg! { buf.len(), len };
        let read_len = self.file.read(&mut buf[..len])?;
        // dbg! { read_len };
        self.pointer += read_len as u32;
        Ok(read_len)
    }
}

impl<'a> io::Seek for GobRead<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let pointer = match pos {
            io::SeekFrom::Start(pointer) => self
                .file
                .seek(io::SeekFrom::Start(self.entry_offset as u64 + pointer))?,
            io::SeekFrom::Current(offset) => {
                if self.pointer as i64 + offset < 0 {
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                self.file.seek(io::SeekFrom::Current(offset))?
            }
            io::SeekFrom::End(offset) => self.file.seek(io::SeekFrom::Start(
                (self.entry_offset as i64 + self.entry_length as i64 + offset) as u64,
            ))?,
        };
        let pointer = pointer - self.entry_offset as u64;
        self.pointer = pointer as u32;
        Ok(pointer)
    }
}
