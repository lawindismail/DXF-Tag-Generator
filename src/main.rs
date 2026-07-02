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
    search_query: String,
    show_math_dialog: bool,
    math_op: usize, // 0 = Add, 1 = Subtract, 2 = Multiply, 3 = Divide
    math_val: String,
    last_clicked_index: Option<usize>,
    drag_is_selecting: Option<bool>,
}

impl eframe::App for TagApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        
        let mut do_math = false;
        
        // Math popup dialog
        if self.show_math_dialog {
            egui::Window::new("Batch Modify Quantity")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_label("Operation")
                            .selected_text(match self.math_op {
                                0 => "Add (+)",
                                1 => "Subtract (-)",
                                2 => "Multiply (*)",
                                3 => "Divide (/)",
                                _ => "",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.math_op, 0, "Add (+)");
                                ui.selectable_value(&mut self.math_op, 1, "Subtract (-)");
                                ui.selectable_value(&mut self.math_op, 2, "Multiply (*)");
                                ui.selectable_value(&mut self.math_op, 3, "Divide (/)");
                            });
                        
                        ui.add(egui::TextEdit::singleline(&mut self.math_val).desired_width(50.0));
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            do_math = true;
                            self.show_math_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_math_dialog = false;
                        }
                    });
                });
        }
        
        if do_math {
            if let Ok(val) = self.math_val.parse::<f64>() {
                for part in &mut self.parts {
                    if part.selected {
                        let cur = part.quantity as f64;
                        let new_val = match self.math_op {
                            0 => cur + val,
                            1 => cur - val,
                            2 => cur * val,
                            3 => cur / val,
                            _ => cur,
                        };
                        part.quantity = new_val.max(1.0).round() as u32; // ensure at least 1
                    }
                }
            }
        }

        // Detect global drag release
        if ctx.input(|i| i.pointer.any_released()) {
            self.drag_is_selecting = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DXF Tag Generator v1.0.1");
            
            ui.horizontal(|ui| {
                if ui.button("Select DSC PDF").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("PDF", &["pdf", "PDF"]).pick_file() {
                        self.pdf_path = Some(path.clone());
                        if let Ok(bytes) = fs::read(&path) {
                            match parse_pdf(&bytes) {
                                Ok(parts) => {
                                    self.parts = parts;
                                    self.status_msg = format!("Loaded {} parts", self.parts.len());
                                    self.last_clicked_index = None;
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
            
            ui.horizontal(|ui| {
                ui.label("Search Part Number:");
                ui.text_edit_singleline(&mut self.search_query);
                
                if ui.button("Select All Filters").clicked() {
                    let q = self.search_query.to_lowercase();
                    for part in &mut self.parts {
                        if q.is_empty() || part.part_number.to_lowercase().contains(&q) {
                            part.selected = true;
                        }
                    }
                }
                
                if ui.button("Deselect All").clicked() {
                    for part in &mut self.parts {
                        part.selected = false;
                    }
                    self.last_clicked_index = None;
                }
            });
            
            ui.add_space(10.0);
            
            // Handle Ctrl+A
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) {
                let q = self.search_query.to_lowercase();
                for part in &mut self.parts {
                    if q.is_empty() || part.part_number.to_lowercase().contains(&q) {
                        part.selected = true;
                    }
                }
            }
            
            use egui_extras::{TableBuilder, Column};
            
            let modifiers = ctx.input(|i| i.modifiers);
            let pointer_down = ctx.input(|i| i.pointer.primary_down());
            
            enum Action {
                ShiftClick(usize),
                CtrlClick(usize),
                Click(usize),
                DragStart(usize),
                Drag(usize),
            }
            let mut actions = Vec::new();
            
            // Table
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click_and_drag())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(40.0).at_least(30.0)) // Select
                .column(Column::initial(200.0).at_least(100.0)) // Part No
                .column(Column::initial(150.0)) // Tag Text
                .column(Column::remainder()) // Qty
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.strong("Sel"); });
                    header.col(|ui| { ui.strong("Part Number"); });
                    header.col(|ui| { ui.strong("Tag Text"); });
                    header.col(|ui| { ui.strong("Quantity"); });
                })
                .body(|mut body| {
                    let q = self.search_query.to_lowercase();
                    let mut show_math = false;
                    
                    let mut visible_indices = Vec::new();
                    for (i, part) in self.parts.iter().enumerate() {
                        if q.is_empty() || part.part_number.to_lowercase().contains(&q) {
                            visible_indices.push(i);
                        }
                    }
                    
                    for &i in &visible_indices {
                        let part = &mut self.parts[i];
                        let is_selected = part.selected;
                        
                        body.row(25.0, |mut row| {
                            row.col(|ui| { 
                                if is_selected {
                                    let mut rect = ui.max_rect();
                                    rect.max.x += 1000.0;
                                    rect.min.x -= 100.0;
                                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_premultiplied(0, 100, 200, 50));
                                }
                                ui.checkbox(&mut part.selected, ""); 
                            });
                            row.col(|ui| { 
                                ui.label(&part.part_number); 
                            });
                            row.col(|ui| { 
                                let modified = part.tag_text != part.original_tag_text;
                                let color = if modified {
                                    egui::Color32::from_rgb(200, 150, 0)
                                } else {
                                    ui.visuals().text_color()
                                };
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut part.tag_text)
                                        .text_color(color)
                                        .desired_width(120.0)
                                );
                                r.context_menu(|ui| {
                                    if ui.button("Restore Original").clicked() {
                                        part.tag_text = part.original_tag_text.clone();
                                        ui.close_menu();
                                    }
                                });
                            });
                            row.col(|ui| { 
                                let r = ui.add(egui::DragValue::new(&mut part.quantity).speed(1.0).range(1..=10000));
                                r.context_menu(|ui| {
                                    if ui.button("Modify Quantity of Selected").clicked() {
                                        show_math = true;
                                        ui.close_menu();
                                    }
                                });
                            });
                            
                            let response = row.response();
                            
                            if response.clicked() {
                                if modifiers.shift {
                                    actions.push(Action::ShiftClick(i));
                                } else if modifiers.ctrl || modifiers.command {
                                    actions.push(Action::CtrlClick(i));
                                } else {
                                    actions.push(Action::Click(i));
                                }
                            }
                            
                            if response.drag_started() {
                                actions.push(Action::DragStart(i));
                            }
                            
                            if response.dragged() || response.drag_started() || (response.hovered() && pointer_down) {
                                actions.push(Action::Drag(i));
                            }
                        });
                    }
                    
                    if show_math {
                        self.show_math_dialog = true;
                    }
                });
            
            // Apply actions
            for action in actions {
                match action {
                    Action::ShiftClick(i) => {
                        if let Some(last) = self.last_clicked_index {
                            let start = last.min(i);
                            let end = last.max(i);
                            for idx in start..=end {
                                self.parts[idx].selected = true;
                            }
                        } else {
                            self.parts[i].selected = true;
                        }
                        self.last_clicked_index = Some(i);
                    }
                    Action::CtrlClick(i) => {
                        self.parts[i].selected = !self.parts[i].selected;
                        self.last_clicked_index = Some(i);
                    }
                    Action::Click(i) => {
                        for p in &mut self.parts {
                            p.selected = false;
                        }
                        self.parts[i].selected = true;
                        self.last_clicked_index = Some(i);
                    }
                    Action::DragStart(i) => {
                        if self.drag_is_selecting.is_none() {
                            self.drag_is_selecting = Some(!self.parts[i].selected);
                        }
                    }
                    Action::Drag(i) => {
                        if let Some(target_state) = self.drag_is_selecting {
                            self.parts[i].selected = target_state;
                            self.last_clicked_index = Some(i);
                        }
                    }
                }
            }
            
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
            
            ui.horizontal(|ui| {
                if ui.button("Generate Selected Tags").clicked() {
                    if let Some(out_dir) = &self.output_dir {
                        let mut success = 0;
                        for part in &self.parts {
                            if part.selected {
                                let qty_dir = out_dir.join(format!("qty_{}", part.quantity));
                                let _ = fs::create_dir_all(&qty_dir);
                                let file_path = qty_dir.join(format!("{}.dxf", part.tag_text));
                                if generate_dxf(&part.tag_text, &file_path, FONT_DATA).is_ok() {
                                    success += 1;
                                }
                            }
                        }
                        self.status_msg = format!("Generated {} selected tags successfully!", success);
                    } else {
                        self.status_msg = "Please select an output directory first.".to_string();
                    }
                }
                
                if ui.button("Generate All Tags").clicked() {
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
            });
            
            ui.add_space(10.0);
            ui.label(&self.status_msg);
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 600.0])
            .with_title("DXF Tag Generator v1.0.1"),
        ..Default::default()
    };
    
    eframe::run_native(
        "DXF Tag Generator",
        options,
        Box::new(|_cc| Ok(Box::<TagApp>::default())),
    )
}

