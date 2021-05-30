use std::{fmt, io};

use crate::common::*;

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
#[derive(Copy, Clone, Debug)]
pub struct SampleRate(u8);

// impl SampleRate {
//     pub fn sample_count_duration(&self, count: usize) -> std::time::Duration {
//         std::time::Duration::from_nanos((256 - self.0 as u64) * count as u64)
//     }
// }
//
impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "~{}Hz", 1_000_000 / (256 - self.0 as u32))
    }
}

pub struct Player {
    graph: bindings::Windows::Media::Audio::AudioGraph,
}
impl Drop for Player {
    fn drop(&mut self) {
        if let Err(error) = self.graph.Close() {
            eprintln!("AudioGraph::Close failed: {}", error);
        }
    }
}

pub fn play(
    voc: &Voc,
    // commands: std::sync::mpsc::Receiver<MediaCommand>,
) -> bindings::Result<Player> {
    use bindings::{
        Interface,
        Windows::Foundation::TypedEventHandler,
        Windows::Media::{Audio::*, MediaProperties::*, Render::*, *},
        Windows::Win32::System::WinRT::IMemoryBufferByteAccess,
    };

    let settings = AudioGraphSettings::Create(AudioRenderCategory::GameEffects)?;
    let graph = AudioGraph::CreateAsync(settings)?.get()?.Graph()?;

    let input_node =
        graph.CreateFrameInputNodeWithFormat(AudioEncodingProperties::CreatePcm(11_025, 1, 8)?)?;

    let output_node = graph
        .CreateDeviceOutputNodeAsync()?
        .get()?
        .DeviceOutputNode()?;
    input_node.AddOutgoingConnection(output_node)?;

    fn create_frame(data: &[u8]) -> bindings::Result<AudioFrame> {
        let frame = AudioFrame::Create(data.len() as u32)?;
        let buffer = frame.LockBuffer(AudioBufferAccessMode::Write)?;
        let reference = buffer.CreateReference()?;
        write_buffer(reference.cast()?, data)?;
        reference.Close()?;
        buffer.Close()?;
        Ok(frame)
    }

    fn write_buffer(access: IMemoryBufferByteAccess, data: &[u8]) -> bindings::Result<()> {
        unsafe {
            let mut bytes = std::ptr::null_mut();
            let mut len = 0;
            access.GetBuffer(&mut bytes, &mut len).ok()?;
            bytes.copy_from(data.as_ptr(), data.len().min(len as usize));
        }
        Ok(())
    }

    let mut repeat = false;
    let mut repeat_frames = Vec::new();

    for chunk in &voc.chunks {
        match chunk {
            Chunk::SoundStart { data, .. } | Chunk::SoundContinue { data } => {
                let frame = create_frame(data)?;
                if repeat {
                    repeat_frames.push(frame.clone());
                    // Avoids small gaps by ensuring there's always a frame being played. E.g.
                    // A(B#), where B should repeat, should end up with a queue like:
                    // ABB
                    //  BB
                    //   B
                    //   BB
                    //    B
                    //    BB
                    // ...
                    // Probably a smarter way to do this?
                    // This is assuming only one repeated chunk, of course.
                    input_node.AddFrame(frame.clone())?;
                }
                input_node.AddFrame(frame)?;
            }
            Chunk::Repeat { .. } => {
                repeat = true;
            }
            Chunk::RepeatEnd => {
                repeat = false;
            }
            _ => {}
        }
    }

    graph.Start()?;

    input_node.AudioFrameCompleted(TypedEventHandler::<
        AudioFrameInputNode,
        AudioFrameCompletedEventArgs,
    >::new({
        let input_node = input_node.clone();
        move |_node, args| {
            if let Some(args) = args {
                let frame = args.Frame()?;
                if repeat_frames.contains(&frame) {
                    input_node.AddFrame(frame)?;
                }
            }
            Ok(())
        }
    }))?;

    Ok(Player { graph })
}
