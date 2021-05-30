use std::fs::File;
use std::io;
use std::path::Path;

use wgpu::util::DeviceExt;

use crate::context::Context;

pub type LoaderResult<T> = Result<T, LoaderError>;

#[derive(Debug)]
pub enum LoaderError {
    NotFound,
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
        let file = self.dark.entry(name).ok_or(LoaderError::NotFound)?;
        let pal = formats::pal::Pal::read(file)?;
        Ok(pal)
    }

    pub fn load_bm(
        &mut self,
        name: &str,
        pal: &formats::pal::Pal,
        context: &Context,
    ) -> LoaderResult<wgpu::Texture> {
        let file = self.textures.entry(name).ok_or(LoaderError::NotFound)?;
        let bm = formats::bm::Bm::read(file)?;
        let texels = bm
            .data
            .iter()
            .flat_map(|&i| {
                std::array::IntoIter::new({
                    // Todo: check bm transparency flag
                    if i == 0 {
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
            &wgpu::TextureDescriptor {
                label: Some(name),
                size: wgpu::Extent3d {
                    width: bm.size.x as u32,
                    height: bm.size.y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED
                    | wgpu::TextureUsage::COPY_SRC
                    | wgpu::TextureUsage::COPY_DST,
            },
            &texels,
        );

        Ok(texture)
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

    pub fn entry<'a>(&'a mut self, name: &str) -> Option<impl io::Read + io::Seek + 'a> {
        let entry = self
            .catalog
            .entries
            .iter()
            .find(|entry| entry.name == name)?;

        io::Seek::seek(&mut self.file, io::SeekFrom::Start(entry.offset as u64)).unwrap();
        // let mut data = vec![0u8; entry.length as usize];
        // std::io::Read::read_exact(&mut self.file, &mut data).unwrap();
        // Some(std::io::Cursor::new(data))
        Some(GobRead {
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
