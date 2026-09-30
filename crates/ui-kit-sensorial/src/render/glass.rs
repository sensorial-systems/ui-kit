//! Drawing glass: what is behind a panel copied out and halved again and
//! again, each level blurring more, then the panel drawn from the copy and
//! one of the levels.

use anyhow::Result;
use ui_kit_core::Rect;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::WgpuFrame;
use wgpu::util::DeviceExt;

use crate::glass::GlassLook;

/// How many times what is behind a panel can be halved: up to a blur of
/// some 64 pixels.
const LEVELS: usize = 5;
/// The copy and the levels are made this many pixels wide and high at a
/// time, so every level is exactly half the one before.
const ALIGN: u32 = 1 << LEVELS;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DownUniform {
    texel: [f32; 4],
    region: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlassUniform {
    rect: [f32; 4],
    shape: [f32; 4],
    look: [f32; 4],
    light: [f32; 4],
    region: [f32; 4],
    screen: [f32; 4],
    tint: [f32; 4],
    extra: [f32; 4],
}

struct Targets {
    /// The target's size, and the size of the copy: rounded up to [`ALIGN`].
    size: (u32, u32),
    aligned: (u32, u32),
    format: wgpu::TextureFormat,
    snapshot: wgpu::Texture,
    snapshot_view: wgpu::TextureView,
    /// Half the copy's size, a quarter, ...
    levels: Vec<wgpu::TextureView>,
}

struct Pipelines {
    format: wgpu::TextureFormat,
    down: wgpu::RenderPipeline,
    glass: wgpu::RenderPipeline,
}

pub struct Glass {
    shader: wgpu::ShaderModule,
    down_layout: wgpu::BindGroupLayout,
    glass_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: Option<Pipelines>,
    targets: Option<Targets>,
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32, size: usize) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size as u64),
        },
        count: None,
    }
}

/// Pixels `(x, y, width, height)` of `rect` on a `size` target, rounded out;
/// `None` when nothing of it is on the target.
fn pixels(rect: Rect, size: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let x0 = rect.x.floor().max(0.0);
    let y0 = rect.y.floor().max(0.0);
    let x1 = (rect.x + rect.width).ceil().min(size.0 as f32);
    let y1 = (rect.y + rect.height).ceil().min(size.1 as f32);
    if !(x1 > x0 && y1 > y0) || ![x0, y0, x1, y1].iter().all(|v| v.is_finite()) {
        return None;
    }
    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

fn pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    label: &str,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
    })
}

impl Glass {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glass"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
        });
        let down_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass blur"),
            entries: &[
                texture_entry(0),
                sampler_entry(1),
                uniform_entry(2, size_of::<DownUniform>()),
            ],
        });
        let glass_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                sampler_entry(2),
                uniform_entry(3, size_of::<GlassUniform>()),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glass"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            shader,
            down_layout,
            glass_layout,
            sampler,
            pipelines: None,
            targets: None,
        }
    }

    fn pipelines(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self.pipelines.as_ref().is_some_and(|p| p.format == format) {
            return;
        }
        let make = |layout: &wgpu::BindGroupLayout,
                    entry: &str,
                    blend: Option<wgpu::BlendState>| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("glass"),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let down = make(&self.down_layout, "fs_down", None);
        let glass = make(
            &self.glass_layout,
            "fs_glass",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        self.pipelines = Some(Pipelines {
            format,
            down,
            glass,
        });
    }

    fn targets(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
        texture_format: wgpu::TextureFormat,
        view_format: wgpu::TextureFormat,
    ) {
        if self
            .targets
            .as_ref()
            .is_some_and(|t| t.size == size && t.format == texture_format)
        {
            return;
        }
        let aligned = (
            size.0.next_multiple_of(ALIGN),
            size.1.next_multiple_of(ALIGN),
        );
        let make = |label, (width, height): (u32, u32), format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let snapshot = make(
            "glass backdrop",
            aligned,
            texture_format,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let snapshot_view = snapshot.create_view(&Default::default());
        let levels = (1..=LEVELS)
            .map(|level| {
                let size = (aligned.0 >> level, aligned.1 >> level);
                make(
                    "glass blur",
                    size,
                    view_format,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                )
                .create_view(&Default::default())
            })
            .collect();
        self.targets = Some(Targets {
            size,
            aligned,
            format: texture_format,
            snapshot,
            snapshot_view,
            levels,
        });
    }

    /// Draw `command`, a panel of `look`, over what `frame` holds so far.
    /// A highlight that follows the pointer is at `cursor`, in pixels, or
    /// where light from above and to the left would fall without one.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
        look: &GlassLook,
        scale: f32,
        time: f32,
        cursor: Option<[f32; 2]>,
    ) -> Result<()> {
        let size = frame.target.size();
        let rect = command.rect;
        let visible = rect.intersect(command.clip.unwrap_or(rect));
        let Some(scissor) = pixels(visible, size) else {
            return Ok(());
        };
        if look.opacity <= 0.001 {
            return Ok(());
        }
        let view_format = frame.target.format;
        let texture_format = frame.target.texture.format();
        anyhow::ensure!(
            texture_format.remove_srgb_suffix() == view_format.remove_srgb_suffix(),
            "glass needs a target viewed in its own format"
        );
        anyhow::ensure!(
            frame
                .target
                .texture
                .usage()
                .contains(wgpu::TextureUsages::COPY_SRC),
            "glass needs COPY_SRC on the target"
        );
        self.pipelines(device, view_format);
        self.targets(device, size, texture_format, view_format);
        let pipelines = self.pipelines.as_ref().expect("made above");
        let targets = self.targets.as_ref().expect("made above");

        let blur = (look.blur * scale).max(0.0);
        // Each level doubles how far the blur reaches: some two of its
        // texels, from the thirteen taps.
        let level = if blur < 1.0 {
            1
        } else {
            ((blur / 2.0).log2().ceil() as usize).clamp(1, LEVELS)
        };
        let reach = (1u32 << (level + 2)) as f32;
        let refraction = look.refraction * scale;
        let padding = reach
            + refraction * (1.3 + look.dispersion)
            + visible.width.max(visible.height) * look.lens
            + 4.0;
        let read = Rect::new(
            visible.x - padding,
            visible.y - padding,
            visible.width + padding * 2.0,
            visible.height + padding * 2.0,
        );
        let Some((rx, ry, rw, rh)) = pixels(read, size) else {
            return Ok(());
        };
        frame.encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d { x: rx, y: ry, z: 0 },
                ..frame.target.texture.as_image_copy()
            },
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d { x: rx, y: ry, z: 0 },
                ..targets.snapshot.as_image_copy()
            },
            wgpu::Extent3d {
                width: rw,
                height: rh,
                depth_or_array_layers: 1,
            },
        );
        let region = [rx as f32, ry as f32, (rx + rw) as f32, (ry + rh) as f32];

        // Halve it, level by level, only where the panel reads.
        let aligned = (targets.aligned.0 as f32, targets.aligned.1 as f32);
        for step in 1..=level {
            let source_size = (
                aligned.0 / (1 << (step - 1)) as f32,
                aligned.1 / (1 << (step - 1)) as f32,
            );
            let size_here = (
                aligned.0 / (1 << step) as f32,
                aligned.1 / (1 << step) as f32,
            );
            let divisor = (1 << step) as f32;
            let uniform = DownUniform {
                texel: [
                    1.0 / source_size.0,
                    1.0 / source_size.1,
                    1.0 / size_here.0,
                    1.0 / size_here.1,
                ],
                region: [
                    (region[0] + 0.5 * divisor * 0.5) / aligned.0,
                    (region[1] + 0.5 * divisor * 0.5) / aligned.1,
                    (region[2] - 0.5 * divisor * 0.5) / aligned.0,
                    (region[3] - 0.5 * divisor * 0.5) / aligned.1,
                ],
            };
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glass blur"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let source = if step == 1 {
                &targets.snapshot_view
            } else {
                &targets.levels[step - 2]
            };
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("glass blur"),
                layout: &self.down_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            });
            let level_size = ((targets.aligned.0 >> step), (targets.aligned.1 >> step));
            let area = Rect::new(
                region[0] / divisor - 2.0,
                region[1] / divisor - 2.0,
                (region[2] - region[0]) / divisor + 4.0,
                (region[3] - region[1]) / divisor + 4.0,
            );
            let Some((x, y, w, h)) = pixels(area, level_size) else {
                continue;
            };
            let mut pass = pass(frame.encoder, &targets.levels[step - 1], "glass blur");
            pass.set_pipeline(&pipelines.down);
            pass.set_bind_group(0, &group, &[]);
            pass.set_scissor_rect(x, y, w, h);
            pass.draw(0..3, 0..1);
        }

        // The panel.
        let light = if look.highlight.iter().any(|v| v.is_nan()) {
            cursor.unwrap_or([rect.x + rect.width * 0.3, rect.y - rect.height * 0.5])
        } else {
            [
                rect.x + rect.width * look.highlight[0],
                rect.y + rect.height * look.highlight[1],
            ]
        };
        let divisor = (1 << level) as f32;
        let linear = view_format.is_srgb() || view_format == wgpu::TextureFormat::Rgba16Float;
        let uniform = GlassUniform {
            rect: [rect.x, rect.y, rect.width, rect.height],
            shape: [
                command.style.radius,
                look.bevel * scale,
                refraction,
                look.dispersion,
            ],
            look: [
                if blur < 1.0 { 0.0 } else { 1.0 },
                look.tint_amount,
                look.rim,
                look.opacity.clamp(0.0, 1.0),
            ],
            light: [
                light[0],
                light[1],
                look.highlight_strength,
                look.highlight_radius * scale,
            ],
            region,
            screen: [
                aligned.0,
                aligned.1,
                divisor / aligned.0,
                divisor / aligned.1,
            ],
            tint: [look.tint[0], look.tint[1], look.tint[2], look.spectral],
            extra: [
                look.lens,
                time % 3600.0,
                if linear { 1.0 } else { 0.0 },
                look.brightness,
            ],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glass"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glass"),
            layout: &self.glass_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.snapshot_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.levels[level - 1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        let mut pass = pass(frame.encoder, frame.target.view, "glass");
        pass.set_pipeline(&pipelines.glass);
        pass.set_bind_group(0, &group, &[]);
        pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
