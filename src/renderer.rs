use crate::recognizer::Point;
use tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

pub fn render_trail(pixmap: &mut Pixmap, points: &[Point], color: [u8; 4], width: f32) {
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    if points.len() < 2 {
        return;
    }

    let mut pb = PathBuilder::new();
    pb.move_to(points[0].x as f32, points[0].y as f32);
    for p in &points[1..] {
        pb.line_to(p.x as f32, p.y as f32);
    }
    let Some(path) = pb.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
    paint.anti_alias = true;

    let mut stroke = Stroke::default();
    stroke.width = width;
    stroke.line_cap = LineCap::Round;

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

/// Convert tiny-skia premultiplied RGBA pixel data to Wayland ARGB8888 format.
///
/// Wayland's `Argb8888` is BGRA in memory on little-endian systems.
/// tiny-skia stores premultiplied RGBA. We swap R and B channels.
pub fn rgba_to_argb(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2]; // B
        d[1] = s[1]; // G
        d[2] = s[0]; // R
        d[3] = s[3]; // A
    }
}
