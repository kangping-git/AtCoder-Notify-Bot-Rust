use std::sync::Arc;

use resvg::render;
use resvg::usvg::{Options, Tree};
use tiny_skia::Pixmap;
use usvg::fontdb::Database;

pub fn svg_to_png(svg_data: &str, w: u32, h: u32, scale_x: f32, scale_y: f32) -> Vec<u8> {
    let mut opt = Options::default();
    let mut fonts = Database::new();
    fonts.load_system_fonts();
    opt.fontdb = Arc::new(fonts);
    let rtree = Tree::from_str(svg_data, &opt).unwrap();

    let mut pixmap = Pixmap::new(w, h).ok_or("Failed to create pixmap").unwrap();

    render(&rtree, tiny_skia::Transform::from_scale(scale_x, scale_y), &mut pixmap.as_mut());

    pixmap.encode_png().unwrap()
}
