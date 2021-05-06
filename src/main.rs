use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;

use eframe::egui;

use error::*;

mod error;
mod fme;
mod gob;

fn main() -> ReadResult<()> {
    eframe::run_native(Box::new(App::new()?))
}

struct App {
    gobs: Vec<GobFile>,
    index: (usize, usize),
    selected: Option<Selected>,
}

struct GobFile {
    name: &'static str,
    file: File,
    catalog: gob::Catalog,
}

struct Selected {
    name: String,
    length: u32,
    text_data: String,
    decoded: Decoded,
}

impl Selected {
    fn layout(&self, ui: &mut egui::Ui) {
        ui.with_layout(
            match &self.decoded {
                Decoded::Unknown => egui::Layout::left_to_right(),
                _ => egui::Layout::right_to_left().with_cross_align(egui::Align::Min),
            },
            |ui| {
                egui::ScrollArea::auto_sized().show(ui, |ui| {
                    ui.add(egui::Label::from(&self.text_data).monospace().wrap(true));
                });

                self.decoded.layout(ui);
            },
        );
    }
}

enum Decoded {
    Unknown,
    Fme { fme: fme::Fme, texture_id: egui::TextureId },
}

impl Decoded {
    fn free(&self, frame: &mut eframe::epi::Frame) {
        if let Self::Fme { texture_id, .. } = self {
            frame.tex_allocator().free(*texture_id);
        }
    }

    fn read(
        frame: &mut eframe::epi::Frame,
        entry: &gob::Entry,
        data: &[u8],
    ) -> Self {
        match entry.name().split('.').last() {
            Some("FME") => {
                let fme = match fme::Fme::read(io::Cursor::new(data)) {
                    Err(..) => return Self::Unknown,
                    Ok(fme) => fme,
                };

                let colors = fme.data.iter()
                    .map(|c| egui::Color32::from_gray(*c))
                    .collect::<Vec<_>>();

                let texture_id = frame.tex_allocator()
                    .alloc_srgba_premultiplied(
                        (fme.size.0 as usize, fme.size.1 as usize),
                        &colors,
                    );
                Self::Fme { fme, texture_id }
            }
            _ => Self::Unknown,
        }
    }

    fn layout(&self, ui: &mut egui::Ui) {
        match self {
            Decoded::Unknown => {}
            Decoded::Fme { fme, texture_id } => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("offset {}x{}", fme.offset.0, fme.offset.1));
                        ui.label(format!("flip {}", fme.flip));
                        ui.label(format!("unit {}x{}", fme.unit_size.0, fme.unit_size.1));
                        ui.label(format!("size {}x{}", fme.size.0, fme.size.1));
                    });
                    ui.image(*texture_id, egui::Vec2::new(
                        fme.size.0 as f32 * 4.0,
                        fme.size.1 as f32 * 4.0,
                    ));
                });
            }
        }
    }
}

const BASE_PATH: &str = r"C:\Games\Steam\steamapps\common\Dark Forces\Game\";

impl App {
    fn new() -> ReadResult<Self> {
        let gobs = ["DARK.GOB", "SOUNDS.GOB", "SPRITES.GOB", "TEXTURES.GOB"]
            .iter()
            .map(|name| -> ReadResult<_> {
                let mut file = File::open(Path::new(BASE_PATH).join(name))?;
                let catalog = gob::Catalog::read(&mut file)?;
                Ok(GobFile { name, file, catalog })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { gobs, index: (0, 0), selected: None })
    }

    fn escape_text_data(data: &[u8]) -> String {
        let unprintable = data.iter()
            .filter(|b| !b.is_ascii() || b.is_ascii_control() && !b.is_ascii_whitespace())
            .count();

        if unprintable > 10 {
            data
                .chunks(16)
                .enumerate()
                .map(|(i, chunk)|
                    format!(
                        "{:4x}:{}\n",
                        i * 16,
                        chunk.into_iter()
                            .map(|b| format!(" {:02x}", &b))
                            .collect::<String>(),
                    )
                )
                .collect()
        } else {
            data
                .iter()
                .flat_map(|&c| {
                    let escape = match c {
                        b'\r' | b'\n' | b' ' => false,
                        b'\\' => true,
                        c => !c.is_ascii_graphic(),
                    };
                    let (a, b) = if !escape {
                        (Some(std::iter::once(char::from(c))), None)
                    } else {
                        (None, Some(char::from(c).escape_default()))
                    };
                    a.into_iter().flatten().chain(b.into_iter().flatten())
                })
                .collect()
        }
    }
}

impl eframe::epi::App for App {
    fn name(&self) -> &str {
        "Dark Forces Explorer"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut eframe::epi::Frame) {
        egui::SidePanel::left(1, 200.0).show(ctx, |ui| {
            ui.heading("Entries");
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                let current_index = self.index;
                let mut new_index = None;
                for (gob_index, gob) in self.gobs.iter_mut().enumerate() {
                    ui.collapsing(gob.name, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            for (entry_index, entry) in gob.catalog.entries().enumerate() {
                                let index = (gob_index, entry_index);
                                if ui.selectable_label(current_index == index, entry.name()).clicked() {
                                    new_index = Some(index);
                                }
                            }
                        });
                    });
                }

                if let Some((gob_index, entry_index)) = new_index {
                    let gob = &mut self.gobs[gob_index];
                    let entry = &gob.catalog[entry_index];

                    let mut data = Vec::new();
                    entry.data(&mut gob.file)
                        .read_to_end(&mut data)
                        .expect("can read file");

                    let decoded = Decoded::read(frame, entry, &data);
                    let text_data = Self::escape_text_data(&data);

                    if let Some(Selected { decoded, .. }) = &self.selected {
                        decoded.free(frame);
                    }

                    self.index = (gob_index, entry_index);
                    self.selected = Some(Selected {
                        name: entry.name().to_string(),
                        length: entry.length(),
                        text_data,
                        decoded,
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.selected {
                None => {
                    ui.heading("No selected item");
                }
                Some(selected) => {
                    ui.heading(format!("Selected: {:?} ({} bytes)", selected.name, selected.length));
                    selected.layout(ui);
                }
            }
        });
    }
}

fn read_i32(input: impl io::Read) -> io::Result<i32> {
    Ok(i32::from_le_bytes(read_buf(input, [0u8; 4])?))
}

fn read_u32(input: impl io::Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_buf(input, [0u8; 4])?))
}

fn read_buf<T: AsMut<[u8]>>(mut input: impl io::Read, mut buffer: T) -> io::Result<T> {
    input.read_exact(buffer.as_mut())?;
    Ok(buffer)
}