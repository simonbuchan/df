use std::io;

use crate::common::*;

use crate::fme;

pub struct Wax {
    pub version: u32,
    pub num_sequences: u32,
    pub num_frames: u32,
    pub num_cells: u32,
    pub states: Vec<WaxState>,
    pub sequences: Vec<WaxSequence>,
    pub frames: Vec<WaxFrame>,
    pub cells: Vec<fme::Cell>,
}

pub struct WaxState {
    pub offset: u32,
    pub world_size: Vec2u32,
    pub frame_rate: u32,
    pub angle_sequence_indices: [usize; 32],
}

pub struct WaxSequence {
    pub offset: u32,
    pub frame_indices: Vec<usize>,
}

pub struct WaxFrame {
    pub offset: u32,
    pub frame: fme::Frame,
    pub cell_index: usize,
}

impl Wax {
    pub fn read(mut file: impl io::Read + io::Seek) -> ReadResult<Self> {
        let version = read_u32(&mut file)?;
        let num_sequences = read_u32(&mut file)?;
        let num_frames = read_u32(&mut file)?;
        let num_cells = read_u32(&mut file)?;
        read_u32(&mut file)?; // scale x, according to dftools (always 0 in Dork Forces)
        read_u32(&mut file)?; // scale y, "
        read_u32(&mut file)?; // extra light, "
        read_u32(&mut file)?; // padding, "

        let mut state_offsets = Vec::with_capacity(32);
        for _ in 0..32 {
            let offset = read_u32(&mut file)?;
            if offset == 0 {
                break;
            }
            state_offsets.push(offset);
        }

        let mut sequence_offsets = IndexMap::with_capacity(num_sequences as usize);
        let mut sequences = Vec::with_capacity(num_sequences as usize);

        let mut frame_offsets = IndexMap::with_capacity(num_frames as usize);
        let mut frames = Vec::with_capacity(num_frames as usize);

        let mut cell_offsets = IndexMap::with_capacity(num_cells as usize);
        let mut cells = Vec::with_capacity(num_cells as usize);

        let mut states = Vec::new();

        for offset in state_offsets {
            file.seek(io::SeekFrom::Start(offset as u64))?;
            let world_size = read_vec2_u32(&mut file)?;
            let frame_rate = read_u32(&mut file)?;
            read_u32(&mut file)?; // num_frames, according to dftools (always 0 in Dark Forces)
            read_u32(&mut file)?; // padding...
            read_u32(&mut file)?;
            read_u32(&mut file)?;

            let mut angle_sequence_indices = [0usize; 32];
            for angle in 0..32 {
                let sequence_offset = read_u32(&mut file)?;
                angle_sequence_indices[angle] = sequence_offsets.add_index(sequence_offset);
            }

            states.push(WaxState {
                offset,
                world_size,
                frame_rate,
                angle_sequence_indices,
            })
        }

        for offset in sequence_offsets.keys {
            file.seek(io::SeekFrom::Start(offset as u64 + 16))?;
            let mut frame_indices = Vec::with_capacity(32);
            for _ in 0..32 {
                let frame_offset = read_u32(&mut file)?;
                if frame_offset == 0 {
                    break;
                }
                frame_indices.push(frame_offsets.add_index(frame_offset));
            }
            sequences.push(WaxSequence {
                offset,
                frame_indices,
            })
        }

        for offset in frame_offsets.keys {
            file.seek(io::SeekFrom::Start(offset as u64))?;
            let frame = fme::Frame::read(&mut file)?;
            let cell_offset = read_u32(&mut file)?;

            let cell_index = cell_offsets.add_index(cell_offset);
            frames.push(WaxFrame {
                offset,
                frame,
                cell_index,
            });
        }

        for cell_offset in cell_offsets.keys {
            cells.push(fme::Cell::read(&mut file, cell_offset)?);
        }

        Ok(Self {
            version,
            num_sequences,
            num_frames,
            num_cells,
            states,
            sequences,
            frames,
            cells,
        })
    }
}

struct IndexMap<K> {
    pub keys: Vec<K>,
}

impl<K: Eq> IndexMap<K> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            keys: Vec::with_capacity(capacity),
        }
    }

    fn add_index(&mut self, key: K) -> usize {
        for index in 0..self.keys.len() {
            if self.keys[index] == key {
                return index;
            }
        }
        let index = self.keys.len();
        self.keys.push(key);
        index
    }
}
