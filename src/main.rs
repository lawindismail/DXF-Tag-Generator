#![windows_subsystem = "windows"]

mod engine;

use engine::{parse_pdf, generate_dxf, PartInfo};
use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;
use std::fs;

const FONT_DATA: &[u8] = include_bytes!("../SairaStencilOne-Regular.ttf");

#[derive(Default)]
struct TagApp {
    pdf_path: Option<PathBuf>,
    parts: Vec<PartInfo>,
    output_dir: Option<PathBuf>,
    status_msg: String,
}

impl eframe::App for TagApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DXF Tag Generator");
            
            ui.horizontal(|ui| {
                if ui.button("Select DSC PDF").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("PDF", &["pdf", "PDF"]).pick_file() {
                        self.pdf_path = Some(path.clone());
                        if let Ok(bytes) = fs::read(&path) {
                            match parse_pdf(&bytes) {
                                Ok(parts) => {
                                    self.parts = parts;
                                    self.status_msg = format!("Loaded {} parts", self.parts.len());
                                },
                                Err(e) => {
                                    self.status_msg = format!("Error parsing PDF: {}", e);
                                }
                            }
                        }
                    }
                }
                
                if let Some(path) = &self.pdf_path {
                    ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                }
            });
            
            ui.add_space(10.0);
            
            use egui_extras::{TableBuilder, Column};
            
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(200.0).at_least(100.0))
                .column(Column::initial(100.0))
                .column(Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.strong("Part Number"); });
                    header.col(|ui| { ui.strong("Tag Text"); });
                    header.col(|ui| { ui.strong("Quantity"); });
                })
                .body(|mut body| {
                    for part in &self.parts {
                        body.row(18.0, |mut row| {
                            row.col(|ui| { ui.label(&part.part_number); });
                            row.col(|ui| { ui.label(&part.tag_text); });
                            row.col(|ui| { ui.label(part.quantity.to_string()); });
                        });
                    }
                });
            
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                if ui.button("Select Output Directory").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.output_dir = Some(path);
                    }
                }
                
                if let Some(path) = &self.output_dir {
                    ui.label(path.to_string_lossy());
                }
            });
            
            ui.add_space(10.0);
            
            if ui.button("Generate Tags").clicked() {
                if let Some(out_dir) = &self.output_dir {
                    let mut success = 0;
                    for part in &self.parts {
                        let qty_dir = out_dir.join(format!("qty_{}", part.quantity));
                        let _ = fs::create_dir_all(&qty_dir);
                        let file_path = qty_dir.join(format!("{}.dxf", part.tag_text));
                        if generate_dxf(&part.tag_text, &file_path, FONT_DATA).is_ok() {
                            success += 1;
                        }
                    }
                    self.status_msg = format!("Generated {} tags successfully!", success);
                } else {
                    self.status_msg = "Please select an output directory first.".to_string();
                }
            }
            
            ui.add_space(10.0);
            ui.label(&self.status_msg);
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_title("DXF Tag Generator"),
        ..Default::default()
    };
    
    eframe::run_native(
        "DXF Tag Generator",
        options,
        Box::new(|_cc| Ok(Box::<TagApp>::default())),
    )
}

