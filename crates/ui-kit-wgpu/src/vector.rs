//! Antialiased vector meshes: the `triangles` commands
//! `ui_kit_immediate::vector`, `svg` and `icon` paint.
//!
//! Compose it with the standard renderer:
//! `WgpuRenderer::with_custom(gpu, format, VectorRenderer::new(gpu, format))`.
use crate::{CommandRenderer, GpuContext, TargetInfo, WgpuFrame};
use anyhow::Result;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use ui_kit_core::{Material, Rect};
use ui_kit_immediate::vector::{COLORED_TRIANGLES, DOTS, TRIANGLES};
use ui_kit_immediate::DrawCommand;
use wgpu::util::DeviceExt;

/// Draws vector meshes and dot grids; hands every other command back to the
/// standard renderer.
pub struct VectorRenderer {
    pipeline: wgpu::RenderPipeline,
    dots: wgpu::RenderPipeline,
    /// What was queued since the last `prepare`.
    pending: Batch,
    /// What the last `prepare` uploaded, which `draw` draws from.
    prepared: Prepared,
    /// Meshes with their antialiasing fringe, by what they were made from:
    /// finding the fringe is most of a mesh's cost, and the same mesh is
    /// asked for frame after frame.
    fringes: HashMap<u64, Vec<Fringed>>,
    frame: u64,
}

/// Meshes and dot grids queued for one frame, their vertices in clip space.
#[derive(Default)]
struct Batch {
    triangles: Vec<[f32; 6]>,
    grids: Vec<DotVertex>,
    draws: Vec<Draw>,
}

#[derive(Default)]
struct Prepared {
    triangles: Option<wgpu::Buffer>,
    grids: Option<wgpu::Buffer>,
    draws: Vec<Draw>,
}

/// One queued command: which vertices, and the pixels it may touch.
struct Draw {
    grid: bool,
    vertices: Range<u32>,
    scissor: (u32, u32, u32, u32),
}

/// A mesh's triangles and fringe, from the corner of its rect. A plain
/// mesh's vertices keep only whether they are covered, in their alpha: its
/// colour is the command's.
struct Fringed {
    parameters: Vec<u32>,
    colored: bool,
    vertices: Vec<[f32; 6]>,
    used: u64,
}

/// How many frames a mesh's fringe is kept undrawn.
const KEEP_FRAMES: u64 = 90;

/// A dot grid's quad corner: where it is, then the grid, the same at every
/// corner: spacing, first dot, radius, colour.
type DotVertex = [f32; 11];

/// Linear from sRGB, for targets that blend in linear light.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
impl VectorRenderer {
    pub fn new(gpu: &GpuContext, format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ui-kit vector geometry"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct Vertex { @builtin(position) position:vec4<f32>, @location(0) color:vec4<f32> }
@vertex fn vs(@location(0) position:vec2<f32>, @location(1) color:vec4<f32>)->Vertex {
    var v:Vertex; v.position=vec4(position,0.,1.);v.color=color;return v;
}
@fragment fn fs(v:Vertex)->@location(0) vec4<f32> {return v.color;}
"#
                    .into(),
                ),
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ui-kit vectors"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x4],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: gpu.pipeline_cache(),
            });
        let dots_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ui-kit dot grid"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct Vertex {
    @builtin(position) position:vec4<f32>,
    @location(0) @interpolate(flat) spacing:vec2<f32>,
    @location(1) @interpolate(flat) first:vec2<f32>,
    @location(2) @interpolate(flat) radius:f32,
    @location(3) @interpolate(flat) color:vec4<f32>,
}
@vertex fn vs(
    @location(0) position:vec2<f32>, @location(1) spacing:vec2<f32>,
    @location(2) first:vec2<f32>, @location(3) radius:f32, @location(4) color:vec4<f32>,
)->Vertex {
    var v:Vertex; v.position=vec4(position,0.,1.); v.spacing=spacing; v.first=first;
    v.radius=radius; v.color=color; return v;
}
@fragment fn fs(v:Vertex)->@location(0) vec4<f32> {
    // Distance from this pixel's centre to the nearest dot's.
    let cell = (v.position.xy - v.first) / v.spacing;
    let offset = (cell - round(cell)) * v.spacing;
    let coverage = clamp(v.radius + 0.5 - length(offset), 0., 1.);
    return vec4(v.color.rgb, v.color.a * coverage);
}
"#
                    .into(),
                ),
            });
        let dots = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ui-kit dot grid"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &dots_shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<DotVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0=>Float32x2, 1=>Float32x2, 2=>Float32x2, 3=>Float32, 4=>Float32x4
                        ],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &dots_shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: gpu.pipeline_cache(),
            });
        Self {
            pipeline,
            dots,
            pending: Batch::default(),
            prepared: Prepared::default(),
            fringes: HashMap::new(),
            frame: 0,
        }
    }

    /// Queue a dot grid: one quad over the command's rect, the dots found
    /// per pixel.
    fn queue_dots(&mut self, target: TargetInfo, command: &DrawCommand, parameters: &[f32]) -> Result<u32> {
        anyhow::ensure!(
            parameters.len() == 5 && parameters.iter().all(|v| v.is_finite()),
            "invalid dot grid"
        );
        let (w, h) = target.size;
        let clip = command
            .clip
            .unwrap_or(screen(target))
            .intersect(screen(target))
            .intersect(command.rect);
        let Some(scissor) = scissor(clip, target) else {
            return Ok(self.push(true, 0, (0, 0, 0, 0)));
        };
        let mut color = command.style.fill;
        if is_linear(target.format) {
            for c in &mut color[..3] {
                *c = linear(*c);
            }
        }
        let corner = |x: f32, y: f32| -> DotVertex {
            [
                x / w as f32 * 2.0 - 1.0,
                1.0 - y / h as f32 * 2.0,
                parameters[0],
                parameters[1],
                command.rect.x + parameters[2],
                command.rect.y + parameters[3],
                parameters[4],
                color[0],
                color[1],
                color[2],
                color[3],
            ]
        };
        let r = command.rect;
        self.pending.grids.extend_from_slice(&[
            corner(r.x, r.y),
            corner(r.x + r.width, r.y),
            corner(r.x, r.y + r.height),
            corner(r.x, r.y + r.height),
            corner(r.x + r.width, r.y),
            corner(r.x + r.width, r.y + r.height),
        ]);
        Ok(self.push(true, 6, scissor))
    }

    /// Queue a mesh, its triangles and their fringe.
    fn queue_mesh(
        &mut self,
        target: TargetInfo,
        command: &DrawCommand,
        parameters: &[f32],
        colored: bool,
    ) -> Result<u32> {
        anyhow::ensure!(
            parameters.len() % (if colored { 18 } else { 6 }) == 0
                && parameters.iter().all(|v| v.is_finite()),
            "invalid triangle coordinates"
        );
        let clip = command.clip.unwrap_or(screen(target)).intersect(screen(target));
        let scissor = match scissor(clip, target) {
            Some(scissor) if !parameters.is_empty() => scissor,
            _ => return Ok(self.push(false, 0, (0, 0, 0, 0))),
        };
        let (w, h) = target.size;
        let linear_target = is_linear(target.format);
        let fill = command.style.fill;
        let (x, y) = (command.rect.x, command.rect.y);
        let start = self.pending.triangles.len();
        let mesh = fringed(&mut self.fringes, self.frame, parameters, colored);
        self.pending.triangles.extend(mesh.iter().map(|v| {
            let mut color = if colored {
                [v[2], v[3], v[4], v[5]]
            } else {
                [fill[0], fill[1], fill[2], if v[5] > 0.0 { fill[3] } else { 0.0 }]
            };
            if linear_target {
                for c in &mut color[..3] {
                    *c = linear(*c);
                }
            }
            [
                (v[0] + x) / w as f32 * 2.0 - 1.0,
                1.0 - (v[1] + y) / h as f32 * 2.0,
                color[0],
                color[1],
                color[2],
                color[3],
            ]
        }));
        let count = self.pending.triangles.len() - start;
        Ok(self.push(false, count as u32, scissor))
    }

    /// Record a draw of the last `count` vertices queued.
    fn push(&mut self, grid: bool, count: u32, scissor: (u32, u32, u32, u32)) -> u32 {
        let end = if grid {
            self.pending.grids.len()
        } else {
            self.pending.triangles.len()
        } as u32;
        self.pending.draws.push(Draw {
            grid,
            vertices: end - count..end,
            scissor,
        });
        self.pending.draws.len() as u32 - 1
    }
}

/// The mesh `parameters` describe with its fringe, from the corner of its
/// rect, found now if no recent frame asked for it.
fn fringed<'a>(
    fringes: &'a mut HashMap<u64, Vec<Fringed>>,
    frame: u64,
    parameters: &[f32],
    colored: bool,
) -> &'a [[f32; 6]] {
    let mut hasher = DefaultHasher::new();
    colored.hash(&mut hasher);
    for v in parameters {
        v.to_bits().hash(&mut hasher);
    }
    let bucket = fringes.entry(hasher.finish()).or_default();
    let same = |entry: &Fringed| {
        entry.colored == colored
            && entry.parameters.len() == parameters.len()
            && entry.parameters.iter().zip(parameters).all(|(a, b)| *a == b.to_bits())
    };
    let index = match bucket.iter().position(same) {
        Some(index) => index,
        None => {
            let mut vertices: Vec<[f32; 6]> = parameters
                .chunks_exact(if colored { 6 } else { 2 })
                .map(|v| {
                    if colored {
                        [v[0], v[1], v[2], v[3], v[4], v[5]]
                    } else {
                        [v[0], v[1], 1.0, 1.0, 1.0, 1.0]
                    }
                })
                .collect();
            antialias(&mut vertices);
            bucket.push(Fringed {
                parameters: parameters.iter().map(|v| v.to_bits()).collect(),
                colored,
                vertices,
                used: frame,
            });
            bucket.len() - 1
        }
    };
    bucket[index].used = frame;
    &bucket[index].vertices
}

fn screen(target: TargetInfo) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: target.size.0 as f32,
        height: target.size.1 as f32,
    }
}

/// Whether the target blends in linear light, so colours are given to it linear.
fn is_linear(format: wgpu::TextureFormat) -> bool {
    format.is_srgb() || format == wgpu::TextureFormat::Rgba16Float
}

/// The whole pixels `clip` touches, if any.
fn scissor(clip: Rect, target: TargetInfo) -> Option<(u32, u32, u32, u32)> {
    if clip.width <= 0.0 || clip.height <= 0.0 {
        return None;
    }
    let (w, h) = target.size;
    let x = clip.x.floor() as u32;
    let y = clip.y.floor() as u32;
    let right = ((clip.x + clip.width).ceil() as u32).min(w);
    let bottom = ((clip.y + clip.height).ceil() as u32).min(h);
    (right > x && bottom > y).then(|| (x, y, right - x, bottom - y))
}

impl CommandRenderer for VectorRenderer {
    fn wants(&self, command: &DrawCommand) -> bool {
        matches!(
            &command.style.material,
            Material::Custom { name, .. }
                if name == DOTS || name == TRIANGLES || name == COLORED_TRIANGLES
        )
    }

    fn queue(&mut self, _: &GpuContext, target: TargetInfo, command: &DrawCommand) -> Result<Option<u32>> {
        let Material::Custom { name, parameters } = &command.style.material else {
            return Ok(None);
        };
        if name == DOTS {
            return self.queue_dots(target, command, parameters).map(Some);
        }
        if name == TRIANGLES || name == COLORED_TRIANGLES {
            return self
                .queue_mesh(target, command, parameters, name == COLORED_TRIANGLES)
                .map(Some);
        }
        Ok(None)
    }

    fn prepare(&mut self, gpu: &GpuContext) -> Result<()> {
        let batch = std::mem::take(&mut self.pending);
        let buffer = |label, contents: &[u8]| {
            (!contents.is_empty()).then(|| {
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(label),
                        contents,
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            })
        };
        self.prepared = Prepared {
            triangles: buffer("ui-kit triangles", bytemuck::cast_slice(&batch.triangles)),
            grids: buffer("ui-kit dot grids", bytemuck::cast_slice(&batch.grids)),
            draws: batch.draws,
        };
        // Meshes no frame has drawn for a while are forgotten.
        self.frame += 1;
        if self.frame % 30 == 0 {
            let oldest = self.frame.saturating_sub(KEEP_FRAMES);
            self.fringes.retain(|_, bucket| {
                bucket.retain(|entry| entry.used >= oldest);
                !bucket.is_empty()
            });
        }
        Ok(())
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, id: u32) {
        let Some(draw) = self.prepared.draws.get(id as usize) else {
            return;
        };
        let buffer = if draw.grid {
            &self.prepared.grids
        } else {
            &self.prepared.triangles
        };
        let Some(buffer) = buffer else {
            return;
        };
        if draw.vertices.is_empty() {
            return;
        }
        let (x, y, w, h) = draw.scissor;
        pass.set_pipeline(if draw.grid { &self.dots } else { &self.pipeline });
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.set_scissor_rect(x, y, w, h);
        pass.draw(draw.vertices.clone(), 0..1);
    }

    /// Drawn in a pass of its own, for a host that renders commands one at
    /// a time.
    fn render(&mut self, gpu: &GpuContext, frame: &mut WgpuFrame<'_>, command: &DrawCommand) -> Result<bool> {
        let target = TargetInfo {
            size: frame.target.size(),
            format: frame.target.format,
        };
        let Some(id) = self.queue(gpu, target, command)? else {
            return Ok(false);
        };
        self.prepare(gpu)?;
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-kit vectors"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        self.draw(&mut pass, id);
        Ok(true)
    }

    /// A mesh or a grid is all in its command.
    fn is_static(&self, _: &DrawCommand) -> bool {
        true
    }
}

/// Add a one-physical-pixel coverage fringe to mesh boundary edges only.
/// Shared triangulation edges cancel, so pie fans and color meshes keep solid interiors.
fn antialias(vertices: &mut Vec<[f32; 6]>) {
    use std::collections::HashMap;
    type Key = (i64, i64);
    let key = |v: [f32; 6]| -> Key {
        (
            (v[0] as f64 * 4096.0).round() as i64,
            (v[1] as f64 * 4096.0).round() as i64,
        )
    };
    let mut edges: HashMap<(Key, Key), ([f32; 6], [f32; 6], f32, usize)> = HashMap::new();
    for triangle in vertices.chunks_exact(3) {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
        let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if cross.abs() < 0.00001 {
            continue;
        }
        for (a, b) in [(a, b), (b, c), (c, a)] {
            let (ka, kb) = (key(a), key(b));
            let edge = if ka < kb { (ka, kb) } else { (kb, ka) };
            edges
                .entry(edge)
                .and_modify(|e| e.3 += 1)
                .or_insert((a, b, cross.signum(), 1));
        }
    }
    for (_, (a, b, sign, count)) in edges {
        if count != 1 {
            continue;
        }
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let length = dx.hypot(dy);
        if length < 0.0001 {
            continue;
        }
        let (nx, ny) = (dy / length * sign, -dx / length * sign);
        let mut outer_a = a;
        let mut outer_b = b;
        outer_a[0] += nx;
        outer_a[1] += ny;
        outer_a[5] = 0.0;
        outer_b[0] += nx;
        outer_b[1] += ny;
        outer_b[5] = 0.0;
        vertices.extend_from_slice(&[a, b, outer_a, b, outer_b, outer_a]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coverage_fringe_excludes_internal_diagonal() {
        let v = |x, y| [x, y, 1., 1., 1., 1.];
        let mut vertices = vec![
            v(0., 0.),
            v(10., 0.),
            v(0., 10.),
            v(0., 10.),
            v(10., 0.),
            v(10., 10.),
        ];
        antialias(&mut vertices);
        assert_eq!(vertices.len(), 6 + 4 * 6);
        for v in &vertices[6..] {
            if v[5] == 0.0 {
                assert!(v[0] < 0.0 || v[0] > 10.0 || v[1] < 0.0 || v[1] > 10.0);
            }
        }
    }
}
