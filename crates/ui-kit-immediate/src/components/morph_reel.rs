//! A [`Scene`](ui_kit_core::morph::Scene) drawn as immediate commands, and
//! the reel that makes one from a time.
//!
//! Rectangles and text become panels and labels; paths and outlines are
//! tessellated into vector meshes (`ui_kit_wgpu::VectorRenderer` draws them).
//! The renderer has no blur for text, so a blurred layer is drawn a number of
//! times around a disc, each copy faint enough that they add up to the
//! layer's opacity where they all overlap and fall off towards its edges.
use crate::vector::fill_triangles;
use crate::TextOverflow;
use crate::Ui;
use lyon_tessellation::{
    math::point, path::Path, BuffersBuilder, FillOptions, FillTessellator, FillVertex, LineCap,
    LineJoin, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};
use ui_kit_core::morph::{Item, Reel, Scene};
use ui_kit_core::{Color, Point, Rect, Style, TextAlign};

/// The reel on a stage of its own: a rounded square of warm canvas.
pub struct MorphReel<'a> {
    reel: &'a Reel,
    radius: f32,
}

impl<'a> MorphReel<'a> {
    pub fn new(reel: &'a Reel) -> Self {
        Self { reel, radius: 16.0 }
    }

    /// The corner radius of the stage.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Draw the reel as it is `time` seconds in, as large a square as fits
    /// in `rect`, centred. Returns the square.
    pub fn show(&self, ui: &mut Ui, rect: Rect, time: f32) -> Rect {
        let scene = self.reel.seek(time);
        let (_, offset) = scene.fit(rect);
        let scene = scene.placed(rect);
        let stage = Rect::new(offset.x, offset.y, scene.size, scene.size);
        super::panel(ui, stage, scene.background, [0.0; 4], self.radius);
        ui.with_clip(stage, |ui| paint_scene(ui, &scene));
        stage
    }
}

/// Paint `scene` as it is placed; its background is the caller's.
pub fn paint_scene(ui: &mut Ui, scene: &Scene) {
    for layer in &scene.layers {
        if layer.opacity <= 0.001 {
            continue;
        }
        let copies = Copies {
            taps: taps(layer.blur),
            opacity: layer.opacity,
        };
        for item in &layer.items {
            paint_item(ui, item, &copies);
        }
    }
}

/// Offsets a layer blurred by `blur` is drawn at.
fn taps(blur: f32) -> Vec<Point> {
    if blur < 0.4 {
        return vec![Point::new(0.0, 0.0)];
    }
    let ring = |count: usize, radius: f32, turn: f32| {
        (0..count).map(move |i| {
            let a = (i as f32 + turn) / count as f32 * std::f32::consts::TAU;
            Point::new(a.cos() * radius, a.sin() * radius)
        })
    };
    let mut taps = vec![Point::new(0.0, 0.0)];
    if blur < 3.0 {
        taps.extend(ring(6, blur, 0.0));
    } else {
        taps.extend(ring(6, blur * 0.5, 0.5));
        taps.extend(ring(10, blur, 0.0));
    }
    taps
}

/// Where a layer's copies go, and how strong each is.
struct Copies {
    taps: Vec<Point>,
    opacity: f32,
}

impl Copies {
    /// `color` as each copy draws it: faint enough that all of them together
    /// reach the layer's opacity (just under, so an opaque layer still
    /// softens at its edges).
    fn each(&self, color: Color) -> Color {
        let target = (color[3] * self.opacity).clamp(0.0, 1.0);
        let alpha = if self.taps.len() == 1 {
            target
        } else {
            1.0 - (1.0 - target.min(0.97)).powf(1.0 / self.taps.len() as f32)
        };
        faded(color, alpha)
    }

    /// `color` at the layer's opacity, drawn once.
    fn once(&self, color: Color) -> Color {
        faded(color, color[3] * self.opacity)
    }
}

fn faded(color: Color, alpha: f32) -> Color {
    [color[0], color[1], color[2], alpha]
}

fn moved(rect: Rect, by: Point) -> Rect {
    Rect::new(rect.x + by.x, rect.y + by.y, rect.width, rect.height)
}

fn paint_item(ui: &mut Ui, item: &Item, copies: &Copies) {
    let taps = &copies.taps[..];
    match item {
        Item::Shadow {
            rect,
            radius,
            blur,
            color,
        } => {
            // A shadow is already soft: draw it once, at the layer's opacity.
            ui.shadow_box(*rect, *radius, *blur, copies.once(*color));
        }
        Item::Rect {
            rect,
            radius,
            fill,
            stroke,
        } => {
            let style = Style {
                fill: copies.each(*fill),
                stroke: stroke.map_or([0.0; 4], |s| copies.each(s.color)),
                stroke_width: stroke.map_or(0.0, |s| s.width),
                radius: *radius,
                ..Style::default()
            };
            ui.with_style(style, |ui| {
                for tap in taps {
                    ui.panel(moved(*rect, *tap));
                }
            });
        }
        Item::Text {
            at,
            text,
            size,
            weight,
            color,
            align,
            caret,
        } => {
            let size = &snap_size(*size);
            let style = Style {
                foreground: copies.each(*color),
                font_size: *size,
                font_weight: *weight,
                text_align: *align,
                text_padding: 0.0,
                overflow: TextOverflow::Clip,
                fill: [0.0; 4],
                stroke: [0.0; 4],
                ..Style::default()
            };
            let width = ui.text_width(text, &style);
            let room = (width + size * 4.0).max(size * 4.0);
            let x = match align {
                TextAlign::Left => at.x,
                TextAlign::Center => at.x - room * 0.5,
                TextAlign::Right => at.x - room,
            };
            let rect = Rect::new(x, at.y - size, room, size * 2.0);
            ui.with_style(style, |ui| {
                for tap in taps {
                    ui.label(moved(rect, *tap), text);
                }
            });
            if let Some(caret) = caret {
                let end = match align {
                    TextAlign::Left => at.x + width,
                    TextAlign::Center => at.x + width * 0.5,
                    TextAlign::Right => at.x,
                };
                let bar = Rect::new(
                    end + size * 0.08,
                    at.y - size * 0.62,
                    (size * 0.09).max(1.0),
                    size * 1.24,
                );
                let style = Style {
                    fill: copies.each(*caret),
                    stroke: [0.0; 4],
                    stroke_width: 0.0,
                    radius: bar.width * 0.5,
                    ..Style::default()
                };
                ui.with_style(style, |ui| {
                    for tap in taps {
                        ui.panel(moved(bar, *tap));
                    }
                });
            }
        }
        Item::Path {
            points,
            stroke,
            closed,
        } => {
            let mesh = stroke_mesh(points, stroke.width, *closed);
            paint_mesh(ui, &mesh, copies.each(stroke.color), taps);
        }
        Item::Polygon {
            points,
            fill,
            stroke,
        } => {
            if let Some(stroke) = stroke {
                let mesh = stroke_mesh(points, stroke.width, true);
                paint_mesh(ui, &mesh, copies.each(stroke.color), taps);
            }
            let mesh = fill_mesh(points);
            paint_mesh(ui, &mesh, copies.each(*fill), taps);
        }
    }
}

/// Text whose size animates is drawn at a limited set of sizes: glyphs are
/// cached per exact size, and a camera zoom would otherwise add new ones
/// every frame until the glyph atlas is full.
fn snap_size(size: f32) -> f32 {
    if size < 24.0 {
        (size * 2.0).round() * 0.5
    } else {
        size.round()
    }
}

fn paint_mesh(ui: &mut Ui, mesh: &[Point], color: Color, taps: &[Point]) {
    if mesh.is_empty() || color[3] <= 0.001 {
        return;
    }
    for tap in taps {
        let placed: Vec<Point> = mesh
            .iter()
            .map(|p| Point::new(p.x + tap.x, p.y + tap.y))
            .collect();
        fill_triangles(ui, &placed, color);
    }
}

fn outline(points: &[Point], closed: bool) -> Option<Path> {
    let (first, rest) = points.split_first()?;
    let mut builder = Path::builder();
    builder.begin(point(first.x, first.y));
    for p in rest {
        builder.line_to(point(p.x, p.y));
    }
    builder.end(closed);
    Some(builder.build())
}

fn flatten(buffers: VertexBuffers<[f32; 2], u32>) -> Vec<Point> {
    buffers
        .indices
        .iter()
        .map(|&i| {
            let [x, y] = buffers.vertices[i as usize];
            Point::new(x, y)
        })
        .collect()
}

/// Triangles covering a line `width` wide along `points`, with round ends
/// and joins.
pub fn stroke_mesh(points: &[Point], width: f32, closed: bool) -> Vec<Point> {
    let Some(path) = outline(points, closed) else {
        return vec![];
    };
    let options = StrokeOptions::tolerance(0.05)
        .with_line_width(width)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round);
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let _ = StrokeTessellator::new().tessellate_path(
        &path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position().to_array()),
    );
    flatten(buffers)
}

/// Triangles filling the outline through `points`, which may be concave.
pub fn fill_mesh(points: &[Point]) -> Vec<Point> {
    let Some(path) = outline(points, true) else {
        return vec![];
    };
    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let _ = FillTessellator::new().tessellate_path(
        &path,
        &FillOptions::tolerance(0.05),
        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position().to_array()),
    );
    flatten(buffers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::morph::{Layer, Stroke};

    fn frame(scene: &Scene) -> Vec<crate::DrawCommand> {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        paint_scene(&mut ui, scene);
        ui.end().to_vec()
    }

    fn scene(blur: f32) -> Scene {
        Scene {
            size: 100.0,
            background: [1.0; 4],
            layers: vec![Layer {
                opacity: 0.5,
                blur,
                items: vec![
                    Item::Rect {
                        rect: Rect::new(10.0, 10.0, 20.0, 20.0),
                        radius: 4.0,
                        fill: [0.0, 0.0, 0.0, 1.0],
                        stroke: None,
                    },
                    Item::Path {
                        points: vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
                        stroke: Stroke {
                            width: 2.0,
                            color: [0.0, 0.0, 0.0, 1.0],
                        },
                        closed: false,
                    },
                ],
            }],
        }
    }

    #[test]
    fn a_sharp_layer_draws_each_item_once_at_its_opacity() {
        let commands = frame(&scene(0.0));
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].kind, "panel");
        assert_eq!(commands[0].style.fill[3], 0.5);
        assert_eq!(commands[1].kind, crate::vector::KIND);
    }

    #[test]
    fn a_blurred_layer_is_many_faint_copies_that_add_up() {
        let commands = frame(&scene(6.0));
        let panels: Vec<_> = commands.iter().filter(|c| c.kind == "panel").collect();
        assert_eq!(panels.len(), 17);
        let alpha = panels[0].style.fill[3];
        let together = 1.0 - (1.0 - alpha).powi(panels.len() as i32);
        assert!(
            alpha < 0.1 && (together - 0.5).abs() < 0.01,
            "{alpha} {together}"
        );
        let spread = panels.iter().map(|c| c.rect.x).fold(0.0, f32::max) - 10.0;
        assert!((spread - 6.0).abs() < 0.01);
    }

    #[test]
    fn the_reel_paints_every_moment() {
        let reel = Reel::default();
        for beat in 0..28 {
            let scene = reel
                .seek(beat as f32 * 0.5 + 0.25)
                .placed(Rect::new(0.0, 0.0, 720.0, 720.0));
            assert!(!frame(&scene).is_empty(), "beat {beat} drew nothing");
        }
    }

    #[test]
    fn concave_outlines_fill_without_spilling() {
        let arrow = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 16.5),
            Point::new(3.9, 12.8),
            Point::new(6.6, 19.0),
            Point::new(9.2, 17.9),
            Point::new(6.6, 11.9),
            Point::new(11.9, 11.9),
        ];
        let mesh = fill_mesh(&arrow);
        assert_eq!(mesh.len() % 3, 0);
        let area: f32 = mesh
            .chunks(3)
            .map(|t| {
                ((t[1].x - t[0].x) * (t[2].y - t[0].y) - (t[2].x - t[0].x) * (t[1].y - t[0].y))
                    .abs()
                    * 0.5
            })
            .sum();
        let shoelace: f32 = arrow
            .iter()
            .zip(arrow.iter().cycle().skip(1))
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f32>()
            .abs()
            * 0.5;
        assert!((area - shoelace).abs() < 0.1, "{area} vs {shoelace}");
    }
}
