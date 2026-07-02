
mod engine;
fn main() {
    let font_data = std::fs::read("SairaStencilOne-Regular.ttf").unwrap();
    engine::generate_dxf("123", std::path::Path::new("test_tags.dxf"), &font_data).unwrap();
    println!("Done");
}

