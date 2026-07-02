
# DXF Tag Generator

A lightning-fast, native desktop application written in Rust designed to automatically parse part numbers from PDF documents and generate explicit, perfectly flattened DXF polygon stencils ready for laser cutting.

## Features
- **PDF Extraction:** Automatically extracts table contents and parses valid part numbers and quantities.
- **True Stencil Polygons:** Completely bypasses standard DXF text entities and disjointed splines. The application flattens the modern `Saira Stencil One` font into explicit, dense line segments (`LWPOLYLINE`), guaranteeing that your CAD or nesting software will never attempt to "heal" the gaps.
- **Automated Organization:** Automatically categorizes and saves the DXF files into subfolders based on quantity (e.g., `qty_1`, `qty_2`).
- **Native GUI:** A clean, dark-mode spreadsheet interface built with `egui`.

## Usage
1. Open the application.
2. Click **Select DSC PDF** and locate your parts list PDF.
3. Review the extracted parts and quantities in the spreadsheet view.
4. Click **Select Output Directory** to choose where the DXF folders should be saved.
5. Click **Generate Tags**!

## Development
To compile the application from source, you must have Rust and Cargo installed.

```bash
cargo build --release
```
The compiled executable will be located in `target/release/dxf-tag-generator.exe`.

