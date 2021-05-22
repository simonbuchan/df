use std::convert::TryFrom;
use std::fs::{read_dir, File};
use std::io;
use std::path::Path;

use eframe::egui;

use crate::common::{Catalog, CatalogEntry, Vec2u32};
use crate::error::ReadResult;

mod common;
mod error;

mod bm;
mod fme;
mod gmd;
mod gob;
mod lfd;
mod pal;
mod wax;

fn main() -> ReadResult<()> {
    eframe::run_native(Box::new(App::new()?))
}

struct App {
    data_files: Vec<DataFile>,
    search: String,
    index: Option<(usize, usize)>,
    gob_palette: GobPalette,
    selected: Option<Selected>,
}

struct DataFile {
    name: String,
    file: File,
    catalog: Catalog,
}

impl DataFile {
    pub fn read(&self, entry: &CatalogEntry) -> Vec<u8> {
        let mut data = vec![0u8; entry.length as usize];
        // some crap so we can read without a &mut self, which causes lifetime conflict.
        #[cfg(windows)]
        let bytes_read =
            std::os::windows::fs::FileExt::seek_read(&self.file, &mut data, entry.offset as u64)
                .expect("can read file");
        #[cfg(unix)]
        let bytes_read =
            std::os::unix::fs::FileExt::read_at(&self.file, &mut data, entry.offset as u64);
        assert_eq!(bytes_read, entry.length as usize);
        data
    }
}

struct GobPalette {
    items: Vec<(String, pal::Pal)>,
    selected: usize,
}

impl GobPalette {}

impl GobPalette {
    fn setup(data_files: &mut [DataFile]) -> Self {
        let mut items = Vec::new();

        let mut selected = 0;

        for file in data_files {
            for entry in &file.catalog.entries {
                if entry.name.ends_with(".PAL") {
                    if entry.name == "SECBASE.PAL" {
                        selected = items.len();
                    }
                    let pal = pal::Pal::read(io::Cursor::new(file.read(entry))).unwrap();
                    items.push((entry.name.clone(), pal));
                }
            }
        }

        Self { items, selected }
    }

    fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.vertical(|ui| {
            ui.heading("Palette");
            for (index, (name, _)) in self.items.iter().enumerate() {
                let response = ui.selectable_value(&mut self.selected, index, name);
                if response.changed() {
                    changed = true;
                }
            }
        });
        changed
    }
}

const BASE_PATH: &str = r"C:\Games\Steam\steamapps\common\Dark Forces\Game\";

impl App {
    fn new() -> ReadResult<Self> {
        let mut data_files = Self::files(BASE_PATH)?;
        let gob_palette = GobPalette::setup(&mut data_files);
        Ok(Self {
            data_files,
            search: String::new(),
            index: None,
            gob_palette,
            selected: None,
        })
    }

    fn files(base_path: impl AsRef<Path>) -> ReadResult<Vec<DataFile>> {
        let base_path = base_path.as_ref();
        let mut result = Vec::new();
        for entry in read_dir(base_path)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("GOB") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut file = File::open(path)?;
                let catalog = gob::read(&mut file)?;
                result.push(DataFile {
                    name,
                    file,
                    catalog,
                });
            }
        }
        for entry in read_dir(base_path.join("LFD"))? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("LFD") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut file = File::open(path)?;
                let catalog = lfd::read(&mut file)?;
                result.push(DataFile {
                    name,
                    file,
                    catalog,
                });
            }
        }
        Ok(result)
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

                if let Some(selected) = &self.selected {
                    if selected.reload {
                        new_index = self.index;
                    }
                }

                for (data_file_index, data_file) in self.data_files.iter_mut().enumerate() {
                    ui.collapsing(&data_file.name, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            for (entry_index, entry) in data_file.catalog.entries.iter().enumerate()
                            {
                                let index = Some((data_file_index, entry_index));
                                if !search.is_empty() && !entry.name.contains(&search) {
                                    continue;
                                }
                                if ui
                                    .selectable_label(current_index == index, &entry.name)
                                    .clicked()
                                {
                                    new_index = index;
                                }
                            }
                        });
                    });
                }

                if let Some((gob_index, entry_index)) = new_index {
                    let data_file = &mut self.data_files[gob_index];
                    let entry = &data_file.catalog.entries[entry_index];

                    let data = data_file.read(entry);

                    let pal = &self.gob_palette.items[self.gob_palette.selected].1;

                    let decoded = match Decoded::read(frame, entry, &data, pal) {
                        Err(error) => {
                            println!("failed to read: {:?}", error);
                            Decoded::Unknown
                        }
                        Ok(decoded) => decoded,
                    };
                    let text_data = Self::escape_text_data(&data);

                    if let Some(Selected { decoded, .. }) = &self.selected {
                        decoded.free(frame);
                    }

                    self.index = Some((gob_index, entry_index));
                    self.selected = Some(Selected {
                        name: entry.name.to_string(),
                        offset: entry.offset,
                        length: entry.length,
                        text_data,
                        decoded,
                        reload: false,
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
                    "Selected: {:?} {:x} ({} bytes)",
                    selected.name, selected.offset, selected.length,
                ));
            }
        });

        if let Some(selected) = &mut self.selected {
            selected.show(ctx, &mut self.gob_palette);
        }
    }
}

struct Selected {
    name: String,
    offset: u32,
    length: u32,
    text_data: String,
    decoded: Decoded,
    reload: bool,
}

impl Selected {
    fn show(&mut self, ctx: &egui::CtxRef, palette: &mut GobPalette) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.decoded {
                Decoded::Unknown => {
                    self.show_raw_data(ui);
                }
                _ => {
                    self.show_content(ui, palette);
                }
            }

            // ui.allocate_ui_with_layout(
            //     ui.available_size(),
            //     egui::Layout::left_to_right()
            //         .with_cross_align(egui::Align::Min)
            //         .with_cross_justify(true),
            //     |ui| {
            //         ui.allocate_ui(ui.available_size() - egui::Vec2::new(410.0, 0.0), |ui| {
            //             self.show_content(ui, palette);
            //         });
            //     },
            // );
        });
    }

    fn show_content(&mut self, ui: &mut egui::Ui, palette: &mut GobPalette) {
        egui::ScrollArea::auto_sized().id_source(1).show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.decoded.want_pal() && palette.show(ui) {
                    self.reload = true;
                }

                self.decoded.show(ui);
            });
        });
    }

    fn show_raw_data(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::auto_sized().id_source(2).show(ui, |ui| {
            ui.add(egui::Label::from(&self.text_data).monospace().wrap(false));
        });
    }
}

struct DecodedImage {
    texture_id: egui::TextureId,
    size: egui::Vec2,
}

impl DecodedImage {
    fn load(fw: &mut eframe::epi::Frame, data: &[u8], size: Vec2u32, pal: &pal::Pal) -> Self {
        let colors = data
            .iter()
            .map(|&c| {
                if c == 0 {
                    egui::Color32::TRANSPARENT
                } else {
                    pal.entries[c as usize].into()
                }
            })
            .collect::<Vec<egui::Color32>>();

        let texture_id = fw
            .tex_allocator()
            .alloc_srgba_premultiplied(size.into(), &colors);

        let size = egui::Vec2::from(size) * 4.0;

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
    Gmd {
        _playing: Box<dyn Drop>,
    },
    Bm {
        bm: bm::Bm,
        image: DecodedImage,
    },
    Pal {
        texture_id: egui::TextureId,
    },
    Fme {
        fme: fme::Fme,
        image: DecodedImage,
    },
    Wax {
        wax: wax::Wax,
        images: Vec<DecodedImage>,
        selected_state: usize,
        selected_angle: usize,
    },
}

impl Decoded {
    fn free(&self, fw: &mut eframe::epi::Frame) {
        match self {
            Self::Bm { image, .. } => {
                image.free(fw);
            }
            Self::Pal { texture_id, .. } => {
                fw.tex_allocator().free(*texture_id);
            }
            Self::Fme { image, .. } => {
                image.free(fw);
            }
            Self::Wax { images, .. } => {
                for image in images {
                    image.free(fw);
                }
            }
            _ => {}
        }
    }

    fn want_pal(&self) -> bool {
        matches!(self, Self::Bm { .. } | Self::Wax { .. } | Self::Fme { .. })
    }

    fn read(
        fw: &mut eframe::epi::Frame,
        entry: &CatalogEntry,
        data: &[u8],
        pal: &pal::Pal,
    ) -> ReadResult<Self> {
        Ok(match entry.name.split('.').last() {
            Some("GMD") => Self::Gmd {
                _playing: Box::new(gmd::play_in_thread(data.to_vec())),
            },
            Some("BM") => {
                let bm = bm::Bm::read(&mut io::Cursor::new(data))?;
                let image = DecodedImage::load(fw, &bm.data, bm.size.into_vec2(), pal);
                Self::Bm { bm, image }
            }
            Some("PAL") => {
                let pal = pal::Pal::read(&mut io::Cursor::new(data))?;
                let mut pixels = [egui::Color32::TRANSPARENT; 256];
                for i in 1..256 {
                    pixels[i] = pal.entries[i].into();
                }
                let texture_id = fw
                    .tex_allocator()
                    .alloc_srgba_premultiplied((16, 16), &pixels);
                Self::Pal { texture_id }
            }
            Some("FME") => {
                let fme = fme::Fme::read(&mut io::Cursor::new(data))?;
                let image = DecodedImage::load(fw, &fme.cell.data, fme.cell.size, pal);
                Self::Fme { fme, image }
            }
            Some("WAX") => {
                let wax = wax::Wax::read(&mut io::Cursor::new(data))?;

                let images = wax
                    .cells
                    .iter()
                    .map(|cell| DecodedImage::load(fw, &cell.data, cell.size, pal))
                    .collect();

                Self::Wax {
                    wax,
                    images,
                    selected_state: 1,
                    selected_angle: 0,
                }
            }
            _ => Self::Unknown,
        })
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
            Decoded::Gmd { .. } => {}
            Decoded::Bm { bm, image } => {
                egui::Grid::new(1).striped(true).show(ui, |ui| {
                    row_vec2(ui, "size", bm.size);
                    row_vec2(ui, "idem size", bm.idem_size);
                    row_code(ui, "flags", format!("{:08b}", bm.flags));
                    row_code(ui, "compression", format!("{:?}", bm.compression));
                });
                image.show(ui, /*flip:*/ false);
            }
            Decoded::Pal { texture_id, .. } => {
                ui.image(*texture_id, (128.0, 128.0));
            }
            Decoded::Fme { fme, image } => {
                ui.vertical(|ui| {
                    egui::Grid::new(1).striped(true).show(ui, |ui| {
                        row_vec2(ui, "offset", fme.frame.offset);
                        row_code(ui, "flip", fme.frame.flip);
                        row_vec2(ui, "size", fme.cell.size);
                    });
                    image.show(ui, fme.frame.flip);
                });
            }
            Decoded::Wax {
                wax,
                images,
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
                                images[cell_index].show(ui, frame.flip);
                            });
                        }
                    }
                    ui.heading("cells");
                    for (cell, decoded) in wax.cells.iter().zip(images) {
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
