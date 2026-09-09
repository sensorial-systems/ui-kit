//! Example of composing specialized vector painting with the standard UI renderer.
use anyhow::Result;
use ui_kit_core::{Material, Rect};
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::{CommandRenderer, GpuContext, WgpuFrame};
use wgpu::util::DeviceExt;

pub struct GeometryRenderer {
    pipeline: wgpu::RenderPipeline,
}
impl GeometryRenderer {
    pub fn new(gpu: &GpuContext, format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gallery vector geometry"),
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
                label: Some("gallery vectors"),
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
                cache: None,
            });
        Self { pipeline }
    }
}
impl CommandRenderer for GeometryRenderer {
    fn render(
        &mut self,
        gpu: &GpuContext,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
    ) -> Result<bool> {
        let Material::Custom { name, parameters } = &command.style.material else {
            return Ok(false);
        };
        let colored = name == "gallery.colored-triangles";
        if name != "gallery.triangles" && !colored {
            return Ok(false);
        }
        anyhow::ensure!(
            parameters.len() % (if colored { 18 } else { 6 }) == 0
                && parameters.iter().all(|v| v.is_finite()),
            "invalid triangle coordinates"
        );
        let (w, h) = frame.target.size();
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: w as f32,
            height: h as f32,
        };
        let clip = command.clip.unwrap_or(bounds).intersect(bounds);
        if clip.width <= 0.0 || clip.height <= 0.0 || parameters.is_empty() {
            return Ok(true);
        }
        let mut vertices: Vec<[f32; 6]> = parameters
            .chunks_exact(if colored { 6 } else { 2 })
            .map(|xy| {
                [
                    xy[0],
                    xy[1],
                    if colored {
                        xy[2]
                    } else {
                        command.style.fill[0]
                    },
                    if colored {
                        xy[3]
                    } else {
                        command.style.fill[1]
                    },
                    if colored {
                        xy[4]
                    } else {
                        command.style.fill[2]
                    },
                    if colored {
                        xy[5]
                    } else {
                        command.style.fill[3]
                    },
                ]
            })
            .collect();
        antialias(&mut vertices);
        for vertex in &mut vertices {
            vertex[0] = vertex[0] / w as f32 * 2.0 - 1.0;
            vertex[1] = 1.0 - vertex[1] / h as f32 * 2.0;
            if frame.target.format.is_srgb()
                || frame.target.format == wgpu::TextureFormat::Rgba16Float
            {
                for c in &mut vertex[2..5] {
                    *c = if *c <= 0.04045 {
                        *c / 12.92
                    } else {
                        ((*c + 0.055) / 1.055).powf(2.4)
                    };
                }
            }
        }
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gallery triangles"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gallery vectors"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, buffer.slice(..));
        let x = clip.x.floor() as u32;
        let y = clip.y.floor() as u32;
        pass.set_scissor_rect(
            x,
            y,
            ((clip.x + clip.width).ceil() as u32).min(w) - x,
            ((clip.y + clip.height).ceil() as u32).min(h) - y,
        );
        pass.draw(0..vertices.len() as u32, 0..1);
        Ok(true)
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
