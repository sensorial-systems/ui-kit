//! What the look draws besides glass and type: soft shadows, flat fills,
//! the hairline of spectrum it underlines things with, and thin line icons.

use ui_kit_core::{Color, Material, Point, Rect, Style};
use ui_kit_immediate::{vector, Ui};

use crate::color::{faded, spectrum};

/// A filled rounded rect.
pub fn fill(ui: &mut Ui, rect: Rect, radius: f32, color: Color) {
    if color[3] <= 0.002 {
        return;
    }
    let style = Style {
        fill: color,
        gradient_end: None,
        stroke: [0.0; 4],
        stroke_width: 0.0,
        radius,
        corner_radii: None,
        material: Material::Solid,
        ..ui.style.clone()
    };
    ui.with_style(style, |ui| ui.panel(rect));
}

/// A soft shadow under `rect`, black at `alpha`.
pub fn shadow(ui: &mut Ui, rect: Rect, radius: f32, blur: f32, alpha: f32) {
    if alpha <= 0.002 {
        return;
    }
    ui.shadow_box(rect, radius, blur, [0.0, 0.0, 0.0, alpha]);
}

/// A hairline that runs through the spectrum from left to right, fading
/// in and out at its ends.
pub fn spectral_line(ui: &mut Ui, from: Point, length: f32, alpha: f32, width: f32) {
    if alpha <= 0.002 || length <= 0.5 {
        return;
    }
    let steps = 24;
    let mut vertices = Vec::with_capacity(steps * 6);
    for i in 0..steps {
        let (a, b) = (i as f32 / steps as f32, (i + 1) as f32 / steps as f32);
        let fade = |t: f32| (t * (1.0 - t) * 4.0).clamp(0.0, 1.0);
        let colour = |t: f32| {
            let [r, g, b, _] = spectrum(t);
            [r, g, b, alpha * fade(t)]
        };
        let (ca, cb) = (colour(a), colour(b));
        let (x0, x1) = (from.x + a * length, from.x + b * length);
        let (y0, y1) = (from.y - width * 0.5, from.y + width * 0.5);
        let v = |x: f32, y: f32, c: [f32; 4]| [x, y, c[0], c[1], c[2], c[3]];
        vertices.extend_from_slice(&[
            v(x0, y0, ca),
            v(x1, y0, cb),
            v(x0, y1, ca),
            v(x1, y0, cb),
            v(x1, y1, cb),
            v(x0, y1, ca),
        ]);
    }
    vector::fill_colored_triangles(ui, &vertices);
}

// ------------------------------------------------------------ icons

fn p(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// An arrow pointing right, in a `size` square at `at`.
pub fn arrow_right(ui: &mut Ui, at: Point, size: f32, color: Color, width: f32) {
    let (x, y, s) = (at.x, at.y, size);
    vector::polyline(
        ui,
        &[p(x + s * 0.12, y + s * 0.5), p(x + s * 0.86, y + s * 0.5)],
        width,
        color,
    );
    vector::polyline(
        ui,
        &[
            p(x + s * 0.56, y + s * 0.2),
            p(x + s * 0.88, y + s * 0.5),
            p(x + s * 0.56, y + s * 0.8),
        ],
        width,
        color,
    );
}

/// An arrow pointing down.
pub fn arrow_down(ui: &mut Ui, at: Point, size: f32, color: Color, width: f32) {
    let (x, y, s) = (at.x, at.y, size);
    vector::polyline(
        ui,
        &[p(x + s * 0.5, y + s * 0.12), p(x + s * 0.5, y + s * 0.86)],
        width,
        color,
    );
    vector::polyline(
        ui,
        &[
            p(x + s * 0.2, y + s * 0.56),
            p(x + s * 0.5, y + s * 0.88),
            p(x + s * 0.8, y + s * 0.56),
        ],
        width,
        color,
    );
}

/// An arrow pointing up and right: out of the page.
pub fn arrow_out(ui: &mut Ui, at: Point, size: f32, color: Color, width: f32) {
    let (x, y, s) = (at.x, at.y, size);
    vector::polyline(
        ui,
        &[p(x + s * 0.2, y + s * 0.8), p(x + s * 0.8, y + s * 0.2)],
        width,
        color,
    );
    vector::polyline(
        ui,
        &[
            p(x + s * 0.34, y + s * 0.2),
            p(x + s * 0.8, y + s * 0.2),
            p(x + s * 0.8, y + s * 0.66),
        ],
        width,
        color,
    );
}

/// A speaker, with its waves while `on` (0 to 1), crossed out when not.
pub fn speaker(ui: &mut Ui, at: Point, size: f32, color: Color, on: f32) {
    let (x, y, s) = (at.x, at.y, size);
    let body = [
        p(x + s * 0.1, y + s * 0.38),
        p(x + s * 0.28, y + s * 0.38),
        p(x + s * 0.28, y + s * 0.62),
        p(x + s * 0.1, y + s * 0.38),
        p(x + s * 0.28, y + s * 0.62),
        p(x + s * 0.1, y + s * 0.62),
        p(x + s * 0.28, y + s * 0.38),
        p(x + s * 0.52, y + s * 0.16),
        p(x + s * 0.52, y + s * 0.84),
        p(x + s * 0.28, y + s * 0.38),
        p(x + s * 0.52, y + s * 0.84),
        p(x + s * 0.28, y + s * 0.62),
    ];
    vector::fill_triangles(ui, &body, color);
    let arc = |radius: f32, alpha: f32| -> (Vec<Point>, Color) {
        let points = (0..=12)
            .map(|i| {
                let a = -0.8 + 1.6 * i as f32 / 12.0;
                p(
                    x + s * 0.52 + a.cos() * s * radius,
                    y + s * 0.5 + a.sin() * s * radius,
                )
            })
            .collect();
        (points, faded(color, alpha))
    };
    let width = (s * 0.08).max(1.2);
    if on > 0.01 {
        let (small, c) = arc(0.18, on);
        vector::polyline(ui, &small, width, c);
        let (large, c) = arc(0.34, on * on);
        vector::polyline(ui, &large, width, c);
    }
    if on < 0.99 {
        let c = faded(color, 1.0 - on);
        vector::polyline(
            ui,
            &[p(x + s * 0.64, y + s * 0.36), p(x + s * 0.9, y + s * 0.64)],
            width,
            c,
        );
        vector::polyline(
            ui,
            &[p(x + s * 0.9, y + s * 0.36), p(x + s * 0.64, y + s * 0.64)],
            width,
            c,
        );
    }
}
