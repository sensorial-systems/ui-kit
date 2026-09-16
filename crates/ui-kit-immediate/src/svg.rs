//! SVG images drawn as vector meshes.
//!
//! [`Svg::parse`] reads an SVG document once and turns its fills and strokes
//! into triangles; painting it after that only places those triangles, so an
//! image can be drawn every frame at any size and stays sharp.
//!
//! Solid colours, opacity, transforms, strokes (width, caps, joins) and both
//! fill rules are supported. Gradients and patterns paint in their first
//! stop's colour; text, images, filters, masks and clip paths are ignored.
use crate::vector::{fill_colored_triangles, COLORED_TRIANGLES};
use crate::Ui;
use lyon_tessellation::{
    math::point, path::Path, BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex,
    LineCap, LineJoin, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};
use std::fmt;
use ui_kit_core::{Color, Rect};

/// Why an SVG could not be read.
#[derive(Debug)]
pub struct SvgError(String);

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid SVG: {}", self.0)
    }
}

impl std::error::Error for SvgError {}

/// Triangles in one colour, in the document's user units.
#[derive(Clone, Debug, PartialEq)]
struct Mesh {
    vertices: Vec<[f32; 2]>,
    color: Color,
}

/// A parsed SVG, ready to paint at any size.
#[derive(Clone, Debug, PartialEq)]
pub struct Svg {
    width: f32,
    height: f32,
    meshes: Vec<Mesh>,
}

impl Svg {
    /// Read an SVG document and tessellate everything it draws.
    pub fn parse(source: &str) -> Result<Self, SvgError> {
        let tree = usvg::Tree::from_str(source, &usvg::Options::default())
            .map_err(|error| SvgError(error.to_string()))?;
        let size = tree.size();
        // Curves are flattened to within a thousandth of the image.
        let tolerance = (size.width().max(size.height()) / 1000.0).max(0.001);
        let mut meshes = Vec::new();
        collect(tree.root(), 1.0, tolerance, &mut meshes);
        Ok(Self {
            width: size.width(),
            height: size.height(),
            meshes,
        })
    }

    /// The document's size in its own units.
    pub fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    /// Whether the document draws nothing.
    pub fn is_empty(&self) -> bool {
        self.meshes.iter().all(|mesh| mesh.vertices.is_empty())
    }

    /// Where the image lands in `rect`: as large as fits, keeping its aspect
    /// ratio, centred.
    pub fn fit(&self, rect: Rect) -> Rect {
        let scale = (rect.width / self.width.max(f32::EPSILON))
            .min(rect.height / self.height.max(f32::EPSILON))
            .max(0.0);
        rect.centered(self.width * scale, self.height * scale)
    }

    /// The triangles to paint the image into `rect`, coloured per vertex.
    /// `tint` replaces every colour, keeping each shape's opacity.
    pub fn vertices(&self, rect: Rect, tint: Option<Color>) -> Vec<[f32; 6]> {
        let area = self.fit(rect);
        let scale = area.width / self.width.max(f32::EPSILON);
        self.meshes
            .iter()
            .flat_map(|mesh| {
                let color = match tint {
                    Some(tint) => [tint[0], tint[1], tint[2], tint[3] * mesh.color[3]],
                    None => mesh.color,
                };
                mesh.vertices.iter().map(move |[x, y]| {
                    [
                        area.x + x * scale,
                        area.y + y * scale,
                        color[0],
                        color[1],
                        color[2],
                        color[3],
                    ]
                })
            })
            .collect()
    }

    /// Paint the image in its own colours, fitted into `rect`.
    pub fn paint(&self, ui: &mut Ui, rect: Rect) {
        fill_colored_triangles(ui, &self.vertices(rect, None));
    }

    /// Paint the image in one colour, as an icon: every shape takes `color`,
    /// keeping its own opacity.
    pub fn paint_tinted(&self, ui: &mut Ui, rect: Rect, color: Color) {
        fill_colored_triangles(ui, &self.vertices(rect, Some(color)));
    }
}

/// The material name SVG images are painted with.
pub const MATERIAL: &str = COLORED_TRIANGLES;

fn collect(group: &usvg::Group, opacity: f32, tolerance: f32, meshes: &mut Vec<Mesh>) {
    let opacity = opacity * group.opacity().get();
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => collect(group, opacity, tolerance, meshes),
            usvg::Node::Path(path) if path.is_visible() => {
                let Some(outline) = outline(path) else {
                    continue;
                };
                let fill = path.fill().map(|fill| {
                    let color = color(fill.paint(), fill.opacity().get() * opacity);
                    let rule = match fill.rule() {
                        usvg::FillRule::NonZero => FillRule::NonZero,
                        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                    };
                    let options = FillOptions::tolerance(tolerance).with_fill_rule(rule);
                    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
                    let _ = FillTessellator::new().tessellate_path(
                        &outline,
                        &options,
                        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
                            v.position().to_array()
                        }),
                    );
                    Mesh {
                        vertices: flatten(buffers),
                        color,
                    }
                });
                let stroke = path.stroke().map(|stroke| {
                    let color = color(stroke.paint(), stroke.opacity().get() * opacity);
                    let cap = match stroke.linecap() {
                        usvg::LineCap::Butt => LineCap::Butt,
                        usvg::LineCap::Round => LineCap::Round,
                        usvg::LineCap::Square => LineCap::Square,
                    };
                    let join = match stroke.linejoin() {
                        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
                        usvg::LineJoin::Round => LineJoin::Round,
                        usvg::LineJoin::Bevel => LineJoin::Bevel,
                    };
                    // The path is already in document units, so the stroke is
                    // scaled by the transform it was drawn under.
                    let ts = path.abs_transform();
                    let scale = ((ts.sx * ts.sx + ts.ky * ts.ky).sqrt()
                        + (ts.kx * ts.kx + ts.sy * ts.sy).sqrt())
                        * 0.5;
                    let options = StrokeOptions::tolerance(tolerance)
                        .with_line_width(stroke.width().get() * scale)
                        .with_line_cap(cap)
                        .with_line_join(join)
                        .with_miter_limit(stroke.miterlimit().get().max(1.0));
                    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
                    let _ = StrokeTessellator::new().tessellate_path(
                        &outline,
                        &options,
                        &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| {
                            v.position().to_array()
                        }),
                    );
                    Mesh {
                        vertices: flatten(buffers),
                        color,
                    }
                });
                let (first, second) = match path.paint_order() {
                    usvg::PaintOrder::FillAndStroke => (fill, stroke),
                    usvg::PaintOrder::StrokeAndFill => (stroke, fill),
                };
                meshes.extend(first.into_iter().chain(second));
            }
            _ => {}
        }
    }
}

/// The path's outline in document units.
fn outline(path: &usvg::Path) -> Option<Path> {
    let data = path.data().clone().transform(path.abs_transform())?;
    let mut builder = Path::builder();
    let mut open = false;
    for segment in data.segments() {
        match segment {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                if open {
                    builder.end(false);
                }
                builder.begin(point(p.x, p.y));
                open = true;
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                builder.line_to(point(p.x, p.y));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(c, p) => {
                builder.quadratic_bezier_to(point(c.x, c.y), point(p.x, p.y));
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(c1, c2, p) => {
                builder.cubic_bezier_to(point(c1.x, c1.y), point(c2.x, c2.y), point(p.x, p.y));
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                if open {
                    builder.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        builder.end(false);
    }
    Some(builder.build())
}

fn flatten(buffers: VertexBuffers<[f32; 2], u32>) -> Vec<[f32; 2]> {
    buffers
        .indices
        .iter()
        .map(|&i| buffers.vertices[i as usize])
        .collect()
}

fn color(paint: &usvg::Paint, opacity: f32) -> Color {
    let (c, stop_opacity) = match paint {
        usvg::Paint::Color(c) => (*c, 1.0),
        usvg::Paint::LinearGradient(g) => first_stop(g.stops()),
        usvg::Paint::RadialGradient(g) => first_stop(g.stops()),
        usvg::Paint::Pattern(_) => (usvg::Color::black(), 1.0),
    };
    [
        c.red as f32 / 255.0,
        c.green as f32 / 255.0,
        c.blue as f32 / 255.0,
        (opacity * stop_opacity).clamp(0.0, 1.0),
    ]
}

fn first_stop(stops: &[usvg::Stop]) -> (usvg::Color, f32) {
    stops.first().map_or((usvg::Color::black(), 1.0), |stop| {
        (stop.color(), stop.opacity().get())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::Material;

    #[test]
    fn a_square_fills_its_box_and_fits_centred() {
        let svg = Svg::parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#ff0000" fill-opacity="0.5"/></svg>"##,
        )
        .unwrap();
        assert_eq!(svg.size(), (10.0, 10.0));
        assert_eq!(
            svg.fit(Rect::new(0.0, 0.0, 40.0, 20.0)),
            Rect::new(10.0, 0.0, 20.0, 20.0)
        );
        let vertices = svg.vertices(Rect::new(0.0, 0.0, 40.0, 20.0), None);
        assert!(!vertices.is_empty());
        assert!(vertices
            .iter()
            .all(|v| (10.0..=30.0).contains(&v[0]) && (0.0..=20.0).contains(&v[1])));
        assert_eq!(vertices[0][2..], [1.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn strokes_are_tessellated_and_tinted() {
        let svg = Svg::parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round"/></svg>"##,
        )
        .unwrap();
        assert!(!svg.is_empty());
        let mut ui = Ui::default();
        ui.begin(Input::default());
        svg.paint_tinted(
            &mut ui,
            Rect::new(0.0, 0.0, 24.0, 24.0),
            [0.0, 0.5, 1.0, 1.0],
        );
        let commands = ui.end();
        let Material::Custom { name, parameters } = &commands[0].style.material else {
            panic!("not a mesh");
        };
        assert_eq!(name, MATERIAL);
        assert_eq!(parameters.len() % 18, 0);
        assert_eq!(parameters[2..6], [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn malformed_documents_are_rejected() {
        assert!(Svg::parse("<svg").is_err());
    }
}
