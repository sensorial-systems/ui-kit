//! Drawing the look with wgpu: the world behind everything
//! ([`BACKDROP`](crate::BACKDROP)) and glass
//! ([`GLASS`](crate::GLASS)), in their turn among the frame's other
//! commands, so glass bends whatever was drawn before it.

mod glass;

use std::sync::Arc;

use anyhow::Result;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::{CommandRenderer, GpuContext, WgpuFrame};

use crate::atmosphere::{SharedAtmosphere, BACKDROP};
use crate::glass::GlassLook;

/// A layer of light added over the world: a 3D logo hanging in it, say.
pub struct LightLayer<'a> {
    /// What to add, stretched over the whole screen. Black adds nothing.
    pub view: &'a wgpu::TextureView,
    /// How strongly it is added, 0 to 1.
    pub opacity: f32,
    /// How far its colours split apart towards the screen's edges.
    pub dispersion: f32,
}

/// What renders the [`LightLayer`] the world is lit by, each frame.
pub trait Light {
    /// Render the layer for a target `size` pixels big at `scale` pixels
    /// per CSS pixel, into `encoder`, before the world is drawn; `None`
    /// when there is nothing to add this frame.
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        scale: f32,
    ) -> Option<LightLayer<'_>>;
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BackdropUniform {
    screen: [f32; 4],
    motion: [f32; 4],
    pointer: [f32; 4],
    tone: [f32; 4],
}

struct Backdrop {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
}

/// Draws the [`BACKDROP`] and glass commands, leaving everything else to
/// the renderers after it. Pair it with `ui_kit_wgpu::VectorRenderer`,
/// which the look's hairlines and icons are drawn with:
///
/// ```ignore
/// WgpuRenderer::with_custom(&gpu, format, (
///     SensorialRenderer::new(&gpu, atmosphere.clone()),
///     VectorRenderer::new(&gpu, format),
/// ))
/// ```
///
/// The target must be viewed in its own format and allow `COPY_SRC`, for
/// glass to read what is behind it.
pub struct SensorialRenderer {
    atmosphere: SharedAtmosphere,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    light: Option<Box<dyn Light>>,
    /// Black, for when there is no light to add.
    nothing: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    shader: wgpu::ShaderModule,
    backdrop: Option<Backdrop>,
    glass: glass::Glass,
}

impl SensorialRenderer {
    pub fn new(gpu: &GpuContext, atmosphere: SharedAtmosphere) -> Self {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        let nothing = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("no light"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("backdrop"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                size_of::<BackdropUniform>() as u64
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backdrop"),
            size: size_of::<BackdropUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backdrop"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop"),
            source: wgpu::ShaderSource::Wgsl(include_str!("backdrop.wgsl").into()),
        });
        let glass = glass::Glass::new(&device);
        Self {
            atmosphere,
            device,
            queue,
            light: None,
            nothing,
            layout,
            uniform,
            sampler,
            shader,
            backdrop: None,
            glass,
        }
    }

    /// Add `light` over the world, rendered afresh each frame.
    pub fn with_light(mut self, light: impl Light + 'static) -> Self {
        self.light = Some(Box::new(light));
        self
    }

    fn backdrop(&mut self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        if self.backdrop.as_ref().is_none_or(|b| b.format != format) {
            let layout = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("backdrop"),
                    bind_group_layouts: &[Some(&self.layout)],
                    immediate_size: 0,
                });
            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("backdrop"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &self.shader,
                        entry_point: Some("vs_fullscreen"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &self.shader,
                        entry_point: Some("fs_backdrop"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
            self.backdrop = Some(Backdrop { format, pipeline });
        }
        self.backdrop.as_ref().expect("made above").pipeline.clone()
    }

    fn draw_backdrop(&mut self, frame: &mut WgpuFrame<'_>) {
        let size = frame.target.size();
        let format = frame.target.format;
        let pipeline = self.backdrop(format);
        let atmosphere = *self.atmosphere.borrow();

        // The light layer first, into its own texture.
        let layer = self
            .light
            .as_mut()
            .and_then(|light| light.render(frame.encoder, size, atmosphere.scale))
            .filter(|layer| layer.opacity > 0.0);
        let (light_view, opacity, dispersion) = match &layer {
            Some(layer) => (layer.view, layer.opacity, layer.dispersion),
            None => (&self.nothing, 0.0, 0.0),
        };

        let linear = format.is_srgb() || format == wgpu::TextureFormat::Rgba16Float;
        let uniform = BackdropUniform {
            screen: [
                size.0 as f32,
                size.1 as f32,
                atmosphere.scale,
                if linear { 1.0 } else { 0.0 },
            ],
            motion: [
                atmosphere.time % 3600.0,
                atmosphere.scroll,
                atmosphere.velocity,
                atmosphere.lit,
            ],
            pointer: [
                atmosphere.pointer[0],
                atmosphere.pointer[1],
                opacity,
                dispersion,
            ],
            tone: [
                atmosphere.hue,
                atmosphere.flash,
                atmosphere.haze[0],
                atmosphere.haze[1],
            ],
        };
        self.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(light_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backdrop"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl CommandRenderer for SensorialRenderer {
    fn wants(&self, command: &DrawCommand) -> bool {
        command.kind == BACKDROP || GlassLook::of(&command.style.material).is_some()
    }

    fn render(
        &mut self,
        _context: &GpuContext,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
    ) -> Result<bool> {
        if command.kind == BACKDROP {
            self.draw_backdrop(frame);
            return Ok(true);
        }
        let Some(look) = GlassLook::of(&command.style.material) else {
            return Ok(false);
        };
        let atmosphere = *self.atmosphere.borrow();
        let cursor = atmosphere
            .cursor
            .map(|[x, y]| [x * atmosphere.scale, y * atmosphere.scale]);
        self.glass.draw(
            &self.device,
            frame,
            command,
            &look,
            atmosphere.scale,
            atmosphere.time,
            cursor,
        )?;
        Ok(true)
    }
}
