use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;

use eframe::egui;

use crate::error::ReadResult;
use std::convert::TryFrom;

mod common;
mod error;
mod fme;
mod gob;
mod wax;

fn main() -> ReadResult<()> {
    eframe::run_native(Box::new(App::new()?))
}

struct App {
    gobs: Vec<GobFile>,
    search: String,
    index: (usize, usize),
    selected: Option<Selected>,
}

struct GobFile {
    name: &'static str,
    file: File,
    catalog: gob::Catalog,
}

const BASE_PATH: &str = r"C:\Games\Steam\steamapps\common\Dark Forces\Game\";

impl App {
    fn new() -> ReadResult<Self> {
        let gobs = ["DARK.GOB", "SOUNDS.GOB", "SPRITES.GOB", "TEXTURES.GOB"]
            .iter()
            .map(|name| -> ReadResult<_> {
                let mut file = File::open(Path::new(BASE_PATH).join(name))?;
                let catalog = gob::Catalog::read(&mut file)?;
                Ok(GobFile {
                    name,
                    file,
                    catalog,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            gobs,
            search: String::new(),
            index: (0, 0),
            selected: None,
        })
    }

    fn escape_text_data(data: &[u8]) -> String {
        let unprintable = data
            .iter()
            .filter(|b| !b.is_ascii() || b.is_ascii_control() && !b.is_ascii_whitespace())
            .count();

        if unprintable > 10 {
            data.chunks(16)
                .enumerate()
                .map(|(i, chunk)| {
                    format!(
                        "{:4x}:{}\n",
                        i * 16,
                        chunk
                            .into_iter()
                            .map(|b| format!(" {:02x}", &b))
                            .collect::<String>(),
                    )
                })
                .collect()
        } else {
            data.iter()
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
            ui.group(|ui| {
                ui.heading("Entries");
                ui.text_edit_singleline(&mut self.search);
            });
            let search = self.search.to_ascii_uppercase();

            egui::ScrollArea::auto_sized().show(ui, |ui| {
                let current_index = self.index;
                let mut new_index = None;
                for (gob_index, gob) in self.gobs.iter_mut().enumerate() {
                    ui.collapsing(gob.name, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            for (entry_index, entry) in gob.catalog.entries().enumerate() {
                                let index = (gob_index, entry_index);
                                if !search.is_empty() && !entry.name().contains(&search) {
                                    continue;
                                }
                                if ui
                                    .selectable_label(current_index == index, entry.name())
                                    .clicked()
                                {
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
                    entry
                        .data(&mut gob.file)
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

        egui::TopPanel::top(1).show(ctx, |ui| match &self.selected {
            None => {
                ui.heading("No selected item");
            }
            Some(selected) => {
                ui.heading(format!(
                    "Selected: {:?} ({} bytes)",
                    selected.name, selected.length
                ));
            }
        });

        if let Some(selected) = &mut self.selected {
            selected.show(ctx);
        }
    }
}

struct Selected {
    name: String,
    length: u32,
    text_data: String,
    decoded: Decoded,
}

impl Selected {
    fn show(&mut self, ctx: &egui::CtxRef) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::left_to_right()
                    .with_cross_align(egui::Align::Min)
                    .with_cross_justify(true),
                |ui| {
                    ui.allocate_ui(ui.available_size() - egui::Vec2::new(410.0, 0.0), |ui| {
                        egui::ScrollArea::auto_sized().id_source(1).show(ui, |ui| {
                            self.decoded.show(ui);
                        });
                    });

                    egui::ScrollArea::auto_sized().id_source(2).show(ui, |ui| {
                        ui.add(egui::Label::from(&self.text_data).monospace().wrap(false));
                    });
                },
            );
        });
    }
}

struct DecodedCell {
    texture_id: egui::TextureId,
    size: egui::Vec2,
}

impl DecodedCell {
    fn load(fw: &mut eframe::epi::Frame, cell: &fme::Cell) -> Self {
        let colors = cell
            .data
            .iter()
            .map(|c| egui::Color32::from_gray(*c))
            .collect::<Vec<_>>();

        let texture_id = fw
            .tex_allocator()
            .alloc_srgba_premultiplied(cell.size.into(), &colors);

        let size = egui::Vec2::from(cell.size) * 4.0;

        Self { texture_id, size }
    }

    fn free(&self, fw: &mut eframe::epi::Frame) {
        fw.tex_allocator().free(self.texture_id);
    }

    fn show(&self, ui: &mut egui::Ui, flip: bool) -> egui::Response {
        let mut uv = egui::Rect::from_min_size(egui::Pos2::ZERO, [1.0, 1.0].into());
        if flip {
            std::mem::swap(&mut uv.min.x, &mut uv.max.x);
        }
        let mut mesh = egui::epaint::Mesh::with_texture(self.texture_id);
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::hover());
        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        ui.painter().add(egui::Shape::mesh(mesh));
        response
    }
}

enum Decoded {
    Unknown,
    Fme {
        fme: fme::Fme,
        cell: DecodedCell,
    },
    Wax {
        wax: wax::Wax,
        cells: Vec<DecodedCell>,
        selected_state: usize,
        selected_angle: usize,
    },
}

impl Decoded {
    fn free(&self, fw: &mut eframe::epi::Frame) {
        match self {
            Self::Fme { cell, .. } => {
                cell.free(fw);
            }
            Self::Wax { cells, .. } => {
                for cell in cells {
                    cell.free(fw);
                }
            }
            _ => {}
        }
    }

    fn read(fw: &mut eframe::epi::Frame, entry: &gob::Entry, data: &[u8]) -> Self {
        match entry.name().split('.').last() {
            Some("FME") => {
                let fme = match fme::Fme::read(&mut io::Cursor::new(data)) {
                    Err(error) => {
                        eprintln!("failed to load FME: {:?}", error);
                        return Self::Unknown;
                    }
                    Ok(fme) => fme,
                };
                let cell = DecodedCell::load(fw, &fme.cell);
                Self::Fme { fme, cell }
            }
            Some("WAX") => {
                let wax = match wax::Wax::read(&mut io::Cursor::new(data)) {
                    Err(error) => {
                        eprintln!("failed to load WAX: {:?}", error);
                        return Self::Unknown;
                    }
                    Ok(wax) => wax,
                };

                let cells = wax
                    .cells
                    .iter()
                    .map(|cell| DecodedCell::load(fw, cell))
                    .collect();

                Self::Wax {
                    wax,
                    cells,
                    selected_state: 1,
                    selected_angle: 0,
                }
            }
            _ => Self::Unknown,
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        fn row_code<T: ToString>(ui: &mut egui::Ui, label: &str, value: T) {
            ui.label(label);
            ui.code(value.to_string());
            ui.end_row();
        }

        fn row_vec2<T: ToString>(ui: &mut egui::Ui, label: &str, value: common::Vec2<T>) {
            ui.label(label);
            ui.code(value.x.to_string());
            ui.code(value.y.to_string());
            ui.end_row();
        }

        match self {
            Decoded::Unknown => {}
            Decoded::Fme { fme, cell } => {
                ui.vertical(|ui| {
                    egui::Grid::new(1).striped(true).show(ui, |ui| {
                        row_vec2(ui, "offset", fme.frame.offset);
                        row_code(ui, "flip", fme.frame.flip);
                        row_vec2(ui, "size", fme.cell.size);
                    });
                    cell.show(ui, fme.frame.flip);
                });
            }
            Decoded::Wax {
                wax,
                cells,
                selected_state,
                selected_angle,
            } => {
                ui.vertical(|ui| {
                    egui::Grid::new(1).striped(true).show(ui, |ui| {
                        row_code(ui, "sequences", wax.num_sequences);
                        row_code(ui, "frames", wax.num_frames);
                        row_code(ui, "cells", wax.num_cells);
                    });
                    ui.heading("state");
                    ui.add(
                        egui::Slider::new(selected_state, 1..=wax.states.len())
                            .clamp_to_range(true),
                    );
                    if let Some(state) = wax.states.get(selected_state.wrapping_sub(1)) {
                        egui::Grid::new(1).striped(true).show(ui, |ui| {
                            row_code(ui, "file offset", format!("{:x}", state.offset));
                            row_vec2(ui, "world size", state.world_size);
                            row_code(ui, "frame rate", state.frame_rate);
                        });
                        ui.heading("angle");
                        ui.add(egui::Slider::new(selected_angle, 0..=31).clamp_to_range(true));
                        let sequence_index = state.angle_sequence_indices[*selected_angle];
                        let sequence = &wax.sequences[sequence_index];
                        ui.heading(format!(
                            "Sequence {} ({:x})",
                            sequence_index, sequence.offset
                        ));
                        for &frame_index in &sequence.frame_indices {
                            ui.horizontal(|ui| {
                                let wax::WaxFrame {
                                    offset,
                                    frame,
                                    cell_index,
                                } = wax.frames[frame_index];
                                let cell = &wax.cells[cell_index];
                                egui::Grid::new(1).striped(true).show(ui, |ui| {
                                    row_code(ui, "file offset", format!("{:x}", offset));
                                    row_code(ui, "frame", frame_index);
                                    row_vec2(ui, "offset", frame.offset);
                                    row_code(ui, "flip", frame.flip);
                                    row_code(ui, "cell", cell_index);
                                    row_vec2(ui, "size", cell.size);
                                    ui.separator();
                                });
                                cells[cell_index].show(ui, frame.flip);
                            });
                        }
                    }
                    ui.heading("cells");
                    for (cell, decoded) in wax.cells.iter().zip(cells) {
                        ui.image(
                            decoded.texture_id,
                            egui::Vec2::try_from(cell.size).unwrap() * 4.0,
                        );
                    }
                });
            }
        }
    }
}
