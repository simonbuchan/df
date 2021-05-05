use std::fs::File;
use std::io::{self, prelude::*};

use eframe::egui;

use error::*;

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
    data: String,
}

impl App {
    fn new() -> ReadResult<Self> {
        let mut file = File::open(r"C:\Games\Steam\steamapps\common\Dark Forces\Game\DARK.GOB")?;
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

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut eframe::epi::Frame) {
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
                        let data = Self::escape_text_data(data);
                        self.selected = Some(Selected {
                            index,
                            name: entry.name().to_string(),
                            length: entry.length(),
                            data,
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
                    egui::ScrollArea::auto_sized().show(ui, |ui| {
                        ui.label(&selected.data);
                    });
                }
            }
        });
    }
}

mod gob;

fn read_u32(input: impl io::Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_buf(input, [0u8; 4])?))
}

fn read_buf<T: AsMut<[u8]>>(mut input: impl io::Read, mut buffer: T) -> io::Result<T> {
    input.read_exact(buffer.as_mut())?;
    Ok(buffer)
}