
use regex::Regex;
use ttf_parser::{Face, OutlineBuilder};
use lyon_geom::{QuadraticBezierSegment, CubicBezierSegment, Point, point};
use dxf::entities::*;
use dxf::{Drawing, Point as DxfPoint, LwPolylineVertex};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PartInfo {
    pub raw_line: String,
    pub part_number: String,
    pub tag_text: String,
    pub quantity: u32,
}

pub fn parse_pdf(bytes: &[u8]) -> Result<Vec<PartInfo>, String> {
    let text = pdf_extract::extract_text_from_mem(bytes).map_err(|e| e.to_string())?;
    let mut parts = Vec::new();
    let re = Regex::new(r"(?i)\b\d{5}-P\d{2}-[A-Z]{3}-(\d{3})(?:-R\d)?\b").unwrap();
    
    for line in text.lines() {
        if let Some(caps) = re.captures(line) {
            let part_number = caps.get(0).unwrap().as_str().to_string();
            let tag_text = caps.get(1).unwrap().as_str().to_string();
            
            let mut qty = 0;
            let parts_str: Vec<&str> = line.split_whitespace().collect();
            if let Some(last) = parts_str.last() {
                if let Ok(q) = last.parse::<u32>() {
                    qty = q;
                }
            }
            if qty > 0 {
                parts.push(PartInfo {
                    raw_line: line.to_string(),
                    part_number,
                    tag_text,
                    quantity: qty,
                });
            }
        }
    }
    Ok(parts)
}

struct GlyphBuilder {
    polygons: Vec<Vec<(f64, f64)>>,
    current_polygon: Vec<(f64, f64)>,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

impl OutlineBuilder for GlyphBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current_polygon.is_empty() {
            self.polygons.push(self.current_polygon.clone());
            self.current_polygon.clear();
        }
        self.current_polygon.push((x as f64 * self.scale + self.offset_x, y as f64 * self.scale + self.offset_y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.current_polygon.push((x as f64 * self.scale + self.offset_x, y as f64 * self.scale + self.offset_y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let start = self.current_polygon.last().copied().unwrap_or((0.0, 0.0));
        let bezier = QuadraticBezierSegment {
            from: point(start.0 as f32, start.1 as f32),
            ctrl: point(x1 * self.scale as f32 + self.offset_x as f32, y1 * self.scale as f32 + self.offset_y as f32),
            to: point(x * self.scale as f32 + self.offset_x as f32, y * self.scale as f32 + self.offset_y as f32),
        };
        bezier.for_each_flattened(0.01, &mut |line| {
            self.current_polygon.push((line.to.x as f64, line.to.y as f64));
        });
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let start = self.current_polygon.last().copied().unwrap_or((0.0, 0.0));
        let bezier = CubicBezierSegment {
            from: point(start.0 as f32, start.1 as f32),
            ctrl1: point(x1 * self.scale as f32 + self.offset_x as f32, y1 * self.scale as f32 + self.offset_y as f32),
            ctrl2: point(x2 * self.scale as f32 + self.offset_x as f32, y2 * self.scale as f32 + self.offset_y as f32),
            to: point(x * self.scale as f32 + self.offset_x as f32, y * self.scale as f32 + self.offset_y as f32),
        };
        bezier.for_each_flattened(0.01, &mut |line| {
            self.current_polygon.push((line.to.x as f64, line.to.y as f64));
        });
    }
    fn close(&mut self) {
        if !self.current_polygon.is_empty() {
            self.polygons.push(self.current_polygon.clone());
            self.current_polygon.clear();
        }
    }
}

pub fn generate_dxf(text: &str, output_path: &Path, font_data: &[u8]) -> Result<(), String> {
    let face = Face::parse(font_data, 0).map_err(|_| "Failed to parse font")?;
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2010;
    
    // Tag outline (50x25)
    let tag_outline = vec![
        LwPolylineVertex { x: 0.0, y: 0.0, ..Default::default() },
        LwPolylineVertex { x: 50.0, y: 0.0, ..Default::default() },
        LwPolylineVertex { x: 50.0, y: 25.0, ..Default::default() },
        LwPolylineVertex { x: 0.0, y: 25.0, ..Default::default() },
    ];
    let mut outline_entity = Entity::new(EntityType::LwPolyline(LwPolyline {
        vertices: tag_outline,
        flags: 1,
        ..Default::default()
    }));
    outline_entity.common.layer = "0".to_string();
    drawing.add_entity(outline_entity);
    
    // Hole at (8, 12.5) with radius 2.5
    let mut circle_entity = Entity::new(EntityType::Circle(Circle {
        center: DxfPoint::new(8.0, 12.5, 0.0),
        radius: 2.5,
        ..Default::default()
    }));
    circle_entity.common.layer = "0".to_string();
    drawing.add_entity(circle_entity);
    
    let units_per_em = face.units_per_em();
    let text_size = 10.0;
    let scale = text_size / units_per_em as f64;
    
    let mut offset_x = 20.0;
    let offset_y = 8.0;
    
    for c in text.chars() {
        if let Some(glyph_id) = face.glyph_index(c) {
            let mut builder = GlyphBuilder {
                polygons: Vec::new(),
                current_polygon: Vec::new(),
                scale,
                offset_x,
                offset_y,
            };
            if let Some(_bbox) = face.outline_glyph(glyph_id, &mut builder) {
                if !builder.current_polygon.is_empty() {
                    builder.polygons.push(builder.current_polygon);
                }
                for poly in builder.polygons {
                    if poly.len() < 2 { continue; }
                    let vertices: Vec<LwPolylineVertex> = poly.iter().map(|(x, y)| LwPolylineVertex {
                        x: *x,
                        y: *y,
                        ..Default::default()
                    }).collect();
                    let mut text_poly = Entity::new(EntityType::LwPolyline(LwPolyline {
                        vertices,
                        flags: 1,
                        ..Default::default()
                    }));
                    text_poly.common.layer = "0".to_string();
                    drawing.add_entity(text_poly);
                }
            }
            if let Some(advance) = face.glyph_hor_advance(glyph_id) {
                offset_x += advance as f64 * scale;
            }
        }
    }
    
    drawing.save_file(output_path).map_err(|e| e.to_string())?;
    Ok(())
}

