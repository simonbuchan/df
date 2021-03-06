use std::fs::{read_dir, File};
use std::io;
use std::path::Path;

use eframe::egui;

use formats::common::*;
use formats::*;

fn main() -> ReadResult<()> {
    eframe::run_native(Box::new(App::new()?), eframe::NativeOptions::default())
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

impl App {
    fn new() -> ReadResult<Self> {
        let steam_path = registry::Hive::CurrentUser
            .open(r"Software\Valve\Steam", registry::Security::Read)
            .expect("want a Steam installation")
            .value("SteamPath")
            .unwrap();
        let steam_path = match steam_path {
            registry::Data::String(steam_path) => steam_path.to_os_string(),
            _ => panic!("unexpected type: {:?}", steam_path),
        };

        let game_path =
            std::path::Path::new(&steam_path).join(r"steamapps\common\Dark Forces\Game");

        let mut data_files = Self::files(game_path)?;
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
            match std::str::from_utf8(data) {
                Ok(str) => str
                    .lines()
                    .enumerate()
                    .map(|(index, line)| format!("{:5}: {}\n", index + 1, line))
                    .collect(),
                Err(_) => data
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
                    .collect(),
            }
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
    fn load(
        fw: &mut eframe::epi::Frame,
        data: &[u8],
        size: mint::Vector2<u32>,
        pal: &pal::Pal,
    ) -> Self {
        let colors = data
            .iter()
            .map(|&c| {
                if c == 0 {
                    egui::Color32::TRANSPARENT
                } else {
                    let (r, g, b) = pal.entries[c as usize].to_rgb();
                    egui::Color32::from_rgb(r, g, b)
                }
            })
            .collect::<Vec<egui::Color32>>();

        let texture_id = fw
            .tex_allocator()
            .alloc_srgba_premultiplied((size.x as usize, size.y as usize), &colors);

        let size = egui::Vec2::new(size.x as f32, size.y as f32) * 4.0;

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
    Lev(DecodedLev),
    Voc {
        voc: voc::Voc,
        _player: voc::Player,
    },
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
            // Levels
            Some("LEV") => {
                let lev = lev::Lev::read(&mut io::Cursor::new(data))?;
                Self::Lev(DecodedLev::new(lev))
            }

            // Audio
            Some("VOC") => {
                let voc = voc::Voc::read(&mut io::Cursor::new(data))?;
                let player = voc::play(&voc).unwrap();
                Self::Voc {
                    voc,
                    _player: player,
                }
            }
            Some("GMD") => {
                let playing = Box::new(gmd::play_in_thread(data.to_vec()));
                Self::Gmd { _playing: playing }
            }

            // Images
            Some("BM") => {
                let bm = bm::Bm::read(&mut io::Cursor::new(data))?;
                let size = mint::Vector2 {
                    x: bm.size.x as u32,
                    y: bm.size.y as u32,
                };
                let image = DecodedImage::load(fw, &bm.data, size, pal);
                Self::Bm { bm, image }
            }
            Some("PAL") => {
                let pal = pal::Pal::read(&mut io::Cursor::new(data))?;
                let mut pixels = [egui::Color32::TRANSPARENT; 256];
                for i in 1..256 {
                    let (r, g, b) = pal.entries[i].to_rgb();
                    pixels[i] = egui::Color32::from_rgb(r, g, b);
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

        fn row_vec2<T: ToString>(ui: &mut egui::Ui, label: &str, value: mint::Vector2<T>) {
            ui.label(label);
            ui.code(value.x.to_string());
            ui.code(value.y.to_string());
            ui.end_row();
        }

        match self {
            Decoded::Unknown => {}
            Decoded::Lev(decoded) => {
                decoded.show(ui);
            }
            Decoded::Voc { voc, .. } => {
                ui.vertical(|ui| {
                    egui::Grid::new(1).show(ui, |ui| {
                        row_code(ui, "version", {
                            let [major, minor] = voc.version.to_be_bytes();
                            format!("{}.{}", major, minor)
                        });
                    });
                    for chunk in &voc.chunks {
                        ui.group(|ui| {
                            egui::Grid::new(2).show(ui, |ui| match chunk {
                                voc::Chunk::SoundStart {
                                    sample_rate,
                                    codec,
                                    data,
                                } => {
                                    row_code(ui, "chunk", "sound start");
                                    row_code(ui, "sample rate", sample_rate);
                                    row_code(ui, "codec", codec);
                                    row_code(ui, "data len", data.len());
                                }
                                voc::Chunk::SoundContinue { data } => {
                                    row_code(ui, "chunk", "sound continue");
                                    row_code(ui, "data len", data.len());
                                }
                                voc::Chunk::Silence {
                                    sample_rate,
                                    sample_count,
                                } => {
                                    row_code(ui, "chunk", "silence");
                                    row_code(ui, "sample rate", sample_rate);
                                    row_code(ui, "sample count", sample_count);
                                }
                                voc::Chunk::Repeat { count } => {
                                    row_code(ui, "chunk", "repeat");
                                    match count {
                                        Some(value) => row_code(ui, "count", value),
                                        None => row_code(ui, "count", "inf"),
                                    }
                                }
                                voc::Chunk::RepeatEnd => {
                                    row_code(ui, "chunk", "repeat end");
                                }
                                voc::Chunk::Unknown { ty, len } => {
                                    row_code(ui, "chunk", ty);
                                    row_code(ui, "len", len);
                                }
                            });
                        });
                    }
                });
            }
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
                            egui::Vec2::new(cell.size.x as f32, cell.size.y as f32) * 4.0,
                        );
                    }
                });
            }
        }
    }
}

struct DecodedLev {
    lev: lev::Lev,
    bounds: egui::Rect,
    layers: Vec<i32>,
    scroll: egui::Vec2,
    zoom: f32,
    layer_index: usize,
    sector_index: usize,
}

impl DecodedLev {
    fn new(lev: lev::Lev) -> DecodedLev {
        let mut bounds = egui::Rect::NOTHING;
        let mut layers = Vec::new();
        for sector in &lev.sectors {
            if let Err(index) = layers.binary_search(&sector.layer) {
                layers.insert(index, sector.layer);
            }

            for mint::Point2 { x, y } in &sector.vertices {
                bounds.extend_with(egui::Pos2::new(*x, *y));
            }
        }

        Self {
            lev,
            bounds,
            layers,
            scroll: egui::Vec2::ZERO,
            zoom: 0.0,
            layer_index: 0,
            sector_index: 0,
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.show_map(ui);
            self.show_sector(ui);
        });
    }

    fn show_map(&mut self, ui: &mut egui::Ui) {
        let mut rect = ui.clip_rect();
        rect.max.x -= 300.0;
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        let hover_pos = response.hover_pos();
        let clicked = response.clicked();
        let drag_delta = response.drag_delta();
        let painter = egui::Painter::new(response.ctx, ui.layer_id(), response.rect.shrink(10.0));

        if ui.input().key_pressed(egui::Key::Space) {
            self.layer_index += 1;
            self.layer_index %= self.layers.len();
        }
        let layer = self.layers[self.layer_index];

        self.scroll += drag_delta;

        fn zoom_to_scale(zoom: f32) -> f32 {
            (zoom / 20.0).exp()
        }

        let mut offset = {
            let clip_rect = painter.clip_rect();

            let x = clip_rect.left() - self.bounds.left() + self.scroll.x;
            let y = clip_rect.top() + self.bounds.bottom() + self.scroll.y;
            egui::Pos2 { x, y }
        };

        let mut scale = zoom_to_scale(self.zoom);

        if let Some(pos) = hover_pos {
            let zoom_delta = ui.input().scroll_delta.y;
            if zoom_delta != 0.0 {
                self.zoom += zoom_delta;
                let new_scale = zoom_to_scale(self.zoom);

                // Adjust to keep pointer in same map position. This means the following
                // must not change:
                //   (pos.x - offset.x) / scale
                //   (offset.y - pos.y) / scale
                let mut delta = offset - pos;
                delta *= new_scale / scale;
                delta += pos - offset;

                // update stored
                self.scroll += delta;
                // and for this frame
                offset += delta;

                scale = new_scale;
            }
        }

        let hover_map_pos = hover_pos.map(|pos| egui::Pos2 {
            x: (pos.x - offset.x) / scale,
            y: (offset.y - pos.y) / scale,
        });

        let mut hover_sector_indices = Vec::new();

        fn draw_sector(
            painter: &egui::Painter,
            sector: &lev::Sector,
            offset: egui::Pos2,
            scale: f32,
            edge_width: f32,
            active: bool,
        ) {
            // draw walls (including walk walls, e.g. cliffs, steps, windows, ...)
            for wall in &sector.walls {
                let left = sector.vertices[wall.left_vertex];
                let right = sector.vertices[wall.right_vertex];

                let left = offset + egui::Vec2::new(left.x, -left.y) * scale;
                let right = offset + egui::Vec2::new(right.x, -right.y) * scale;

                let stroke_color = if active {
                    egui::Color32::YELLOW
                } else if wall.walk_sector.is_none() {
                    egui::Color32::RED
                } else {
                    egui::Color32::LIGHT_GRAY
                };
                let stroke = egui::Stroke::new(edge_width, stroke_color);

                painter.line_segment([left, right], stroke);
            }
        }

        for (ix, sector) in self.lev.sectors.iter().enumerate() {
            // current sector should be drawn last
            if ix != self.sector_index {
                let edge_width = if sector.layer == layer { 1.0 } else { 0.2 };
                draw_sector(&painter, sector, offset, scale, edge_width, false);
            }

            // Test if mouse is hovering this sector using the ray-cast approach from:
            // http://alienryderflex.com/polygon/
            // Fails with unclosed polygons, like sector 3 on SECBASE.LEV
            if let Some(pos) = hover_map_pos {
                let mut inside = false;
                for wall in &sector.walls {
                    let left = sector.vertices[wall.left_vertex];
                    let right = sector.vertices[wall.right_vertex];
                    if (left.y < pos.y && pos.y <= right.y || right.y < pos.y && pos.y <= left.y)
                        && (left.x + (pos.y - left.y) / (right.y - left.y) * (right.x - left.x)
                            < pos.x)
                    {
                        inside = !inside;
                    }
                }

                if inside {
                    hover_sector_indices.push(ix);
                }
            }
        }

        draw_sector(
            &painter,
            &self.lev.sectors[self.sector_index],
            offset,
            scale,
            1.0,
            true,
        );

        let origin_stroke = egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE);
        painter.line_segment(
            [
                offset - egui::Vec2::new(-5.0, 0.0),
                offset - egui::Vec2::new(5.0, 0.0),
            ],
            origin_stroke,
        );
        painter.line_segment(
            [
                offset - egui::Vec2::new(0.0, -5.0),
                offset - egui::Vec2::new(0.0, 5.0),
            ],
            origin_stroke,
        );

        if let Some(pos) = hover_map_pos {
            let mut text_bounds = painter.text(
                painter.clip_rect().left_top(),
                egui::Align2::LEFT_TOP,
                format_args!("{:.2}, {:.2} / layer {}", pos.x, pos.y, layer),
                egui::TextStyle::Body,
                egui::Color32::WHITE,
            );
            for &sector_index in &hover_sector_indices {
                let sector = &self.lev.sectors[sector_index];
                text_bounds = painter.text(
                    text_bounds.left_bottom(),
                    egui::Align2::LEFT_TOP,
                    format_args!(
                        "sector {}: {:.2}..{:.2} {:?}",
                        sector.id,
                        sector.floor_altitude,
                        sector.ceiling_altitude,
                        sector.name.as_deref().unwrap_or_default(),
                    ),
                    egui::TextStyle::Body,
                    if self.sector_index == sector_index {
                        egui::Color32::YELLOW
                    } else if layer == sector.layer {
                        egui::Color32::LIGHT_GRAY
                    } else {
                        egui::Color32::GRAY
                    },
                );
            }

            if clicked && !hover_sector_indices.is_empty() {
                // Select the next (or first) hovered sector on click.
                let last_hover_index = hover_sector_indices
                    .iter()
                    .position(|&index| index == self.sector_index);
                let next_hover_index = match last_hover_index {
                    None => 0,
                    Some(x) => (x + 1) % hover_sector_indices.len(),
                };
                let next_sector_index = hover_sector_indices[next_hover_index];
                self.sector_index = next_sector_index;
                let next_layer = self.lev.sectors[next_sector_index].layer;
                self.layer_index = self.layers.iter().position(|&l| l == next_layer).unwrap();
            }
        }
    }

    fn show_sector(&mut self, ui: &mut egui::Ui) {
        fn tex_row(ui: &mut egui::Ui, name: &str, lev: &lev::Lev, texture: &lev::Texture) {
            if let Some(tex) = texture.index {
                ui.label(name);
                ui.code(&lev.texture_names[tex]);
                ui.code(format!("{}", texture.offset.x));
                ui.code(format!("{}", texture.offset.y));
                ui.end_row();
            }
        }

        ui.vertical_centered_justified(|ui| {
            egui::Grid::new("level").show(ui, |ui| {
                ui.label("parallax");
                ui.code(format!("{}", self.lev.parallax.x));
                ui.code(format!("{}", self.lev.parallax.y));
                ui.end_row();
            });

            ui.add(
                egui::Slider::new(&mut self.sector_index, 0..=self.lev.sectors.len() - 1)
                    .text("sector"),
            );
            let sector = &self.lev.sectors[self.sector_index];
            egui::Grid::new("sector").show(ui, |ui| {
                ui.label("id");
                ui.code(format!("{}", sector.id));
                ui.end_row();
                if let Some(ref name) = sector.name {
                    ui.label("name");
                    ui.code(name);
                    ui.end_row();
                }
                tex_row(ui, "ceiling", &self.lev, &sector.ceiling_texture);
                tex_row(ui, "floor", &self.lev, &sector.floor_texture);
                ui.label("ambient");
                ui.code(format!("{}", sector.ambient));
                ui.end_row();
                ui.label("flags");
                ui.code(format!("{:x}", sector.flags.0));
                ui.code(format!("{:x}", sector.flags.1));
                ui.code(format!("{:x}", sector.flags.2));
                ui.end_row();
            });

            egui::containers::ScrollArea::auto_sized().show(ui, |ui| {
                for (ix, wall) in sector.walls.iter().enumerate() {
                    ui.group(|ui| {
                        egui::Grid::new(ix).show(ui, |ui| {
                            ui.label("Wall");
                            ui.code(format!("{}", ix));
                            ui.end_row();
                            ui.label("left");
                            ui.code(format!("{}", wall.left_vertex));
                            let left = sector.vertices[wall.left_vertex];
                            ui.code(format!("{}", left.x));
                            ui.code(format!("{}", left.y));
                            ui.end_row();
                            ui.label("right");
                            ui.code(format!("{}", wall.right_vertex));
                            let right = sector.vertices[wall.right_vertex];
                            ui.code(format!("{}", right.x));
                            ui.code(format!("{}", right.y));
                            ui.end_row();
                            ui.label("light");
                            ui.code(format!("{}", wall.light));
                            ui.end_row();
                            ui.label("textures");
                            ui.end_row();
                            tex_row(ui, "mid", &self.lev, &wall.middle_texture);
                            tex_row(ui, "top", &self.lev, &wall.top_texture);
                            tex_row(ui, "bot", &self.lev, &wall.bottom_texture);
                            ui.label("flags");
                            ui.code(format!("{:x}", wall.flags.0));
                            ui.code(format!("{:x}", wall.flags.1));
                            ui.code(format!("{:x}", wall.flags.2));
                            ui.end_row();
                        });
                    });
                }
            });
        });
    }
}
