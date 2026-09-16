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
pub fn polyline(ui: &mut Ui, points: &[Point], width: f32, color: Color) {
    let triangles: Vec<Point> = points
        .windows(2)
        .filter_map(|pair| segment(pair[0], pair[1], width))
        .flatten()
        .collect();
    fill_triangles(ui, &triangles, color);
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
}
