//! Vector drawing: triangle meshes, lines and polylines, painted as commands
//! a vector renderer (`ui_kit_wgpu::VectorRenderer`) turns into antialiased
//! geometry.
//!
//! A mesh travels as a [`Material::Custom`] on a `triangles` command. Its
//! vertices are relative to the command's rect, so moving or scaling the
//! command moves the mesh with it.
use crate::{Response, Ui};
use ui_kit_core::{Color, Material, Point, Rect, Style};

/// A mesh in one colour, the command's `style.fill`: `[x, y]` per vertex,
/// three vertices per triangle.
pub const TRIANGLES: &str = "ui-kit.triangles";
/// A mesh coloured per vertex: `[x, y, r, g, b, a]` per vertex, three
/// vertices per triangle.
pub const COLORED_TRIANGLES: &str = "ui-kit.colored-triangles";
/// A grid of dots in the command's `style.fill`:
/// `[spacing_x, spacing_y, first_x, first_y, radius]`, the first dot's
/// centre relative to the command's rect. Drawn by a shader, so a grid of
/// any density is one command.
pub const DOTS: &str = "ui-kit.dots";
/// The command kind vector meshes are painted as.
pub const KIND: &str = "triangles";

/// The smallest rect holding `points`.
fn bounds(points: impl Iterator<Item = [f32; 2]>) -> Option<Rect> {
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    for [x, y] in points {
        min = [min[0].min(x), min[1].min(y)];
        max = [max[0].max(x), max[1].max(y)];
    }
    (min[0] <= max[0] && min[1] <= max[1])
        .then(|| Rect::new(min[0], min[1], max[0] - min[0], max[1] - min[1]))
}

fn paint(ui: &mut Ui, name: &str, rect: Rect, parameters: Vec<f32>, fill: Color) {
    let style = Style {
        fill,
        stroke: [0.0; 4],
        stroke_width: 0.0,
        material: Material::Custom {
            name: name.into(),
            parameters,
        },
        ..ui.style.clone()
    };
    ui.with_style(style, |ui| {
        ui.paint(KIND, "", rect, 0.0, Response::default(), false)
    });
}

/// Fill the triangles `points` makes, three at a time, in `color`.
pub fn fill_triangles(ui: &mut Ui, points: &[Point], color: Color) {
    let Some(rect) = bounds(points.iter().map(|p| [p.x, p.y])) else {
        return;
    };
    let parameters = points
        .iter()
        .flat_map(|p| [p.x - rect.x, p.y - rect.y])
        .collect();
    paint(ui, TRIANGLES, rect, parameters, color);
}

/// Fill triangles coloured per vertex: `[x, y, r, g, b, a]`, three at a time.
pub fn fill_colored_triangles(ui: &mut Ui, vertices: &[[f32; 6]]) {
    let Some(rect) = bounds(vertices.iter().map(|v| [v[0], v[1]])) else {
        return;
    };
    let parameters = vertices
        .iter()
        .flat_map(|v| [v[0] - rect.x, v[1] - rect.y, v[2], v[3], v[4], v[5]])
        .collect();
    paint(ui, COLORED_TRIANGLES, rect, parameters, [1.0; 4]);
}

/// The two triangles of a segment `width` thick.
fn segment(from: Point, to: Point, width: f32) -> Option<[Point; 6]> {
    let length = (to.x - from.x).hypot(to.y - from.y);
    if length < 0.001 {
        return None;
    }
    let dx = -(to.y - from.y) / length * width * 0.5;
    let dy = (to.x - from.x) / length * width * 0.5;
    let a = Point::new(from.x + dx, from.y + dy);
    let b = Point::new(from.x - dx, from.y - dy);
    let c = Point::new(to.x + dx, to.y + dy);
    let d = Point::new(to.x - dx, to.y - dy);
    Some([a, b, c, b, d, c])
}

/// A straight line `width` thick.
pub fn line(ui: &mut Ui, from: Point, to: Point, width: f32, color: Color) {
    if let Some(triangles) = segment(from, to, width) {
        fill_triangles(ui, &triangles, color);
    }
}

/// Connected straight lines through `points`, `width` thick, as one mesh.
///
/// Its edges are smoothed the way a browser strokes a path: a pixel-wide
/// ramp centred on each edge, so the line covers as much as its width and
/// no more, and a line thinner than a pixel fades instead of thickening.
pub fn polyline(ui: &mut Ui, points: &[Point], width: f32, color: Color) {
    let points: Vec<Point> = points
        .iter()
        .copied()
        .fold(Vec::new(), |mut kept: Vec<Point>, p| {
            if kept.last().is_none_or(|q| (p.x - q.x).hypot(p.y - q.y) > 0.001) {
                kept.push(p);
            }
            kept
        });
    if points.len() < 2 || width <= 0.0 {
        return;
    }
    // A pixel, where the line is drawn.
    let pixel = 1.0 / ui.transform().map_or(1.0, |t| t.scale).max(0.0001);
    let thickness = width / pixel;
    let (core, outer, alpha) = if thickness >= 1.0 {
        ((thickness - 1.0) * 0.5 * pixel, (thickness + 1.0) * 0.5 * pixel, color[3])
    } else {
        (0.0, pixel, color[3] * thickness)
    };
    // Each point's offset across the line: the mean of its segments'
    // normals, lengthened to keep the width through a bend.
    let normal = |from: Point, to: Point| {
        let length = (to.x - from.x).hypot(to.y - from.y);
        Point::new(-(to.y - from.y) / length, (to.x - from.x) / length)
    };
    let last = points.len() - 1;
    let offsets: Vec<Point> = (0..points.len())
        .map(|i| {
            let before = (i > 0).then(|| normal(points[i - 1], points[i]));
            let after = (i < last).then(|| normal(points[i], points[i + 1]));
            match (before, after) {
                (Some(a), Some(b)) => {
                    let mean = Point::new(a.x + b.x, a.y + b.y);
                    let length = mean.x.hypot(mean.y);
                    if length < 0.001 {
                        return a;
                    }
                    let unit = Point::new(mean.x / length, mean.y / length);
                    let miter = (1.0 / (unit.x * a.x + unit.y * a.y)).min(2.0);
                    Point::new(unit.x * miter, unit.y * miter)
                }
                (Some(n), None) | (None, Some(n)) => n,
                (None, None) => Point::new(0.0, 0.0),
            }
        })
        .collect();
    let [r, g, b, _] = color;
    // Four rows along the line: the outer edge, clear; the core's edges,
    // solid; the other outer edge, clear.
    let rows: Vec<[[f32; 6]; 4]> = points
        .iter()
        .zip(&offsets)
        .map(|(p, n)| {
            let at = |d: f32, a: f32| [p.x + n.x * d, p.y + n.y * d, r, g, b, a];
            [at(outer, 0.0), at(core, alpha), at(-core, alpha), at(-outer, 0.0)]
        })
        .collect();
    let mut vertices = Vec::with_capacity(last * 18);
    for pair in rows.windows(2) {
        let (this, next) = (pair[0], pair[1]);
        for k in 0..3 {
            vertices.extend_from_slice(&[this[k], this[k + 1], next[k], this[k + 1], next[k + 1], next[k]]);
        }
    }
    fill_colored_triangles(ui, &vertices);
}

/// A filled circle of `radius` around `center`.
pub fn circle(ui: &mut Ui, center: Point, radius: f32, color: Color) {
    let steps = ((radius * 1.5).ceil() as usize).clamp(12, 96);
    let at = |i: usize| {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        )
    };
    let triangles: Vec<Point> = (0..steps)
        .flat_map(|i| [center, at(i), at(i + 1)])
        .collect();
    fill_triangles(ui, &triangles, color);
}

/// Fill `rect` with dots of `radius` in `color`, `spacing` apart, one of
/// them centred on `first`: a dotted background, as CSS draws a repeated
/// `radial-gradient`.
pub fn dot_grid(ui: &mut Ui, rect: Rect, spacing: Point, first: Point, radius: f32, color: Color) {
    if rect.width <= 0.0 || rect.height <= 0.0 || spacing.x <= 0.0 || spacing.y <= 0.0 {
        return;
    }
    let parameters = vec![spacing.x, spacing.y, first.x - rect.x, first.y - rect.y, radius];
    paint(ui, DOTS, rect, parameters, color);
}

/// The middle of `rect`.
pub fn center(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;

    #[test]
    fn meshes_travel_relative_to_their_command() {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        line(
            &mut ui,
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
            2.0,
            [1.0; 4],
        );
        let commands = ui.end();
        let command = &commands[0];
        assert_eq!(command.kind, KIND);
        assert_eq!(command.rect, Rect::new(10.0, 19.0, 20.0, 2.0));
        let Material::Custom { name, parameters } = &command.style.material else {
            panic!("not a mesh");
        };
        assert_eq!(name, TRIANGLES);
        assert_eq!(parameters.len(), 12);
        assert!(parameters.iter().all(|v| (0.0..=20.0).contains(v)));
    }

    /// The alpha of each vertex of a horizontal polyline, by its height
    /// above the line.
    fn ramp(width: f32, scale: f32) -> Vec<(f32, f32)> {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        let transform = crate::Transform::new(Point::new(0.0, 0.0), scale);
        ui.with_transform(transform, |ui| {
            polyline(ui, &[Point::new(0.0, 10.0), Point::new(20.0, 10.0)], width, [1.0; 4]);
        });
        let command = ui.end()[0].clone();
        let Material::Custom { name, parameters } = &command.style.material else {
            panic!("not a mesh");
        };
        assert_eq!(name, COLORED_TRIANGLES);
        let mut ramp: Vec<(f32, f32)> = parameters
            .chunks_exact(6)
            .map(|v| (v[1] + command.rect.y - 10.0, v[5]))
            .collect();
        ramp.sort_by(|a, b| a.0.total_cmp(&b.0));
        ramp.dedup();
        ramp
    }

    #[test]
    fn polylines_cover_their_width() {
        // A pixel-wide ramp centred on each edge of a 3px line.
        assert_eq!(ramp(3.0, 1.0), vec![(-2.0, 0.0), (-1.0, 1.0), (1.0, 1.0), (2.0, 0.0)]);
        // Zoomed out to half a pixel, it fades rather than thickens.
        assert_eq!(ramp(1.0, 0.5), vec![(-2.0, 0.0), (0.0, 0.5), (2.0, 0.0)]);
    }
}
