use std::fs::File;
use std::io::{self, prelude::*};

use eframe::egui;

use error::*;
use std::path::Path;

mod error;

fn main() -> ReadResult<()> {
    eframe::run_native(Box::new(App::new()?))
}

struct App {
    file: File,
    catalog: gob::Catalog,
    selected: Option<Selected>,
}

struct Selected {
    index: usize,
    name: String,
    length: u32,
    text_data: String,
    decoded: Decoded,
}

enum Decoded {
    Unknown,
    Fme { fme: fme::Fme, texture_id: egui::TextureId },
}

mod fme;

const BASE_PATH: &str = r"C:\Games\Steam\steamapps\common\Dark Forces\Game\";

impl App {
    fn new() -> ReadResult<Self> {
        let mut file = File::open(Path::new(BASE_PATH).join("SPRITES.GOB"))?;
        let catalog = gob::Catalog::read(&mut file)?;
        Ok(Self { file, catalog, selected: None })
    }

    fn escape_text_data(data: Vec<u8>) -> String {
        data
            .into_iter()
            .flat_map(|c| {
                let escape = match c {
                    b'\r' | b'\n' | b' ' => false,
                    b'\\' => true,
                    c => !c.is_ascii_graphic(),
                };
                let (a,b) = if !escape {
                    (Some(std::iter::once(char::from(c))), None)
                } else {
                    (None, Some(char::from(c).escape_default()))
                };
                a.into_iter().flatten().chain(b.into_iter().flatten())
            })
            .collect()
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
                let selected_index = self.selected.as_ref().map(|s| s.index);

                for (index, entry) in self.catalog.entries().enumerate() {
                    let selected = Some(index) == selected_index;

                    if ui.selectable_label(selected, entry.name()).clicked() {
                        let mut data = Vec::new();
                        entry.data(&mut self.file)
                            .read_to_end(&mut data)
                            .expect("can read file");

                        let decoded = if entry.name().ends_with(".FME") {
                            match fme::Fme::read(io::Cursor::new(&data)) {
                                Ok(fme) => {
                                    let colors = fme.data.iter()
                                        .map(|c| egui::Color32::from_gray(*c))
                                        .collect::<Vec<_>>();
                                    let texture_id = frame.tex_allocator()
                                        .alloc_srgba_premultiplied(
                                            (fme.size.0 as usize, fme.size.1 as usize),
                                            &colors,
                                        );
                                    Decoded::Fme {
                                        fme,
                                        texture_id,
                                    }
                                }
                                Err(..) => Decoded::Unknown,
                            }
                        } else {
                            Decoded::Unknown
                        };

                        let text_data = Self::escape_text_data(data);

                        if let Some(Selected { decoded, .. }) = &self.selected {
                            if let Decoded::Fme { texture_id, .. } = decoded {
                                frame.tex_allocator().free(*texture_id);
                            }
                        }

                        self.selected = Some(Selected {
                            index,
                            name: entry.name().to_string(),
                            length: entry.length(),
                            text_data,
                            decoded,
                        });
                    }
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
                    ui.horizontal(|ui| {
                        match &selected.decoded {
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

                        egui::ScrollArea::auto_sized().show(ui, |ui| {
                            ui.label(&selected.text_data);
                        });
                    });
                }
            }
        });
    }
}

mod gob;

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