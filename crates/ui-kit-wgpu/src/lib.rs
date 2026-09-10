//! UI rendering on the application's wgpu device, into caller-owned textures.
//! No surface acquisition, device creation, queue submission, or blocking polling.
use anyhow::{ensure, Result};
mod context;
mod rich_text;
use context::TextState;
pub use context::{GpuContext, Texture2d};
pub use rich_text::RichTextLayout;
use std::sync::Arc;
use ui_kit_immediate::{
    composition::UiRenderer,
    immediate::{DrawCommand, Material, Rect},
};
use wgpu::util::DeviceExt;

/// A single-sample 2D color texture. All resources must belong to the shared device.
pub struct TextureTarget<'a> {
    pub texture: &'a wgpu::Texture,
    pub view: &'a wgpu::TextureView,
    /// The view format, which can differ from the underlying texture's sRGB setting.
    pub format: wgpu::TextureFormat,
}
impl<'a> TextureTarget<'a> {
    pub fn from_texture2d(texture: &'a Texture2d) -> Result<Self> {
        Ok(Self {
            texture: texture
                .texture
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing texture"))?,
            view: texture
                .view
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing texture view"))?,
            format: texture.format.into(),
        })
    }
    pub fn size(&self) -> (u32, u32) {
        (self.texture.width(), self.texture.height())
    }
}

/// Borrow the host's encoder; the host decides when to submit and present.
pub struct WgpuFrame<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target: TextureTarget<'a>,
    /// None preserves existing scene pixels. Some clears once before UI painting.
    pub clear: Option<wgpu::Color>,
}

/// Override individual commands without replacing input logic or the default renderer.
/// Return true when handled; false delegates to the next renderer.
pub trait CommandRenderer {
    fn render(
        &mut self,
        context: &GpuContext,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
    ) -> Result<bool>;
}
pub struct NoCustom;
impl CommandRenderer for NoCustom {
    fn render(&mut self, _: &GpuContext, _: &mut WgpuFrame<'_>, _: &DrawCommand) -> Result<bool> {
        Ok(false)
    }
}
impl<A: CommandRenderer, B: CommandRenderer> CommandRenderer for (A, B) {
    fn render(
        &mut self,
        context: &GpuContext,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
    ) -> Result<bool> {
        if self.0.render(context, frame, command)? {
            Ok(true)
        } else {
            self.1.render(context, frame, command)
        }
    }
}

pub struct WgpuRenderer<C = NoCustom> {
    context: GpuContext,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    text: TextState,
    dummy_backdrop: wgpu::Texture,
    pub custom: C,
}
impl WgpuRenderer {
    pub fn new(context: &GpuContext, format: wgpu::TextureFormat) -> Result<Self> {
        Self::with_custom(context, format, NoCustom)
    }
}
impl<C: CommandRenderer> WgpuRenderer<C> {
    pub fn with_custom(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        custom: C,
    ) -> Result<Self> {
        validate_format(format)?;
        let device = &context.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Prism UI"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shape.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI shape layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make_pipeline = |blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("UI shapes"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
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
        let pipeline = make_pipeline(Some(wgpu::BlendState::ALPHA_BLENDING));
        let glass_pipeline = make_pipeline(None);
        Ok(Self {
            context: context.clone(),
            format,
            pipeline,
            glass_pipeline,
            layout,
            text: TextState::new(device, &context.queue, format, target_is_linear(format)),
            dummy_backdrop: device.create_texture(&wgpu::TextureDescriptor {
                label: Some("UI solid material placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
            custom,
        })
    }
    pub fn context(&self) -> &GpuContext {
        &self.context
    }
    /// Supply a font on platforms without system fonts, or add application typography.
    pub fn load_font(&mut self, bytes: Vec<u8>) {
        self.text.font_system.db_mut().load_font_data(bytes);
    }

    /// Allocate a sampleable render target on the shared device without polling it.
    pub fn create_texture(&self, width: u32, height: u32) -> Result<Texture2d> {
        let max = self.context.device.limits().max_texture_dimension_2d;
        ensure!(
            width > 0 && height > 0 && width <= max && height <= max,
            "invalid UI texture size"
        );
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST;
        let texture = self
            .context
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("UI target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage,
                view_formats: &[],
            });
        let view = texture.create_view(&Default::default());
        Ok(Texture2d {
            texture: Some(Arc::new(texture)),
            view: Some(Arc::new(view)),
            size: (width, height),
            format: self.format,
            usage,
        })
    }
    fn shape(
        &self,
        frame: &mut WgpuFrame<'_>,
        command: &DrawCommand,
        rect: Rect,
        fill: [f32; 4],
        stroke: [f32; 4],
        radius: f32,
        glass: bool,
    ) -> Result<()> {
        let Some(clip) = scissor(
            Some(rect.intersect(command.clip.unwrap_or(rect))),
            frame.target.size(),
        ) else {
            return Ok(());
        };
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return Ok(());
        }
        let (width, height) = frame.target.size();
        // A distinct snapshot avoids reading the texture while it is a render attachment.
        let snapshot = if glass {
            self.context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("UI backdrop"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: frame.target.texture.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[self.format],
                })
        } else {
            self.dummy_backdrop.clone()
        };
        if glass {
            ensure!(
                frame
                    .target
                    .texture
                    .usage()
                    .contains(wgpu::TextureUsages::COPY_SRC),
                "glass needs COPY_SRC on the target texture"
            );
            frame.encoder.copy_texture_to_texture(
                frame.target.texture.as_image_copy(),
                snapshot.as_image_copy(),
                frame.target.texture.size(),
            );
        }
        let snapshot_view = snapshot.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format),
            ..Default::default()
        });
        let blur = if let Material::Glass { blur } = command.style.material {
            blur
        } else {
            0.0
        };
        let params = [
            [rect.x, rect.y, rect.width, rect.height],
            target_color(fill, self.format),
            target_color(stroke, self.format),
            [width as f32, height as f32, radius, blur],
            [
                if glass { 1.0 } else { 0.0 },
                if command.focused {
                    command.style.stroke_width.max(2.0)
                } else {
                    command.style.stroke_width
                },
                if command.style.gradient_end.is_some() {
                    1.0
                } else {
                    0.0
                },
                0.0,
            ],
            target_color(command.style.gradient_end.unwrap_or(fill), self.format),
        ];
        // Blur one axis into a separate attachment; never sample the active target.
        let horizontal = if glass && blur >= 0.5 {
            let intermediate = self
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("UI horizontal blur"),
                    size: snapshot.size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
            let view = intermediate.create_view(&Default::default());
            let mut horizontal_params = params;
            horizontal_params[0] = [0.0, 0.0, width as f32, height as f32];
            horizontal_params[4] = [0.0, 0.0, 0.0, 1.0];
            let buffer =
                self.context
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("UI horizontal blur parameters"),
                        contents: bytemuck::cast_slice(&horizontal_params),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
            let bindings = self
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("UI horizontal blur"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&snapshot_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&snapshot_view),
                        },
                    ],
                });
            let padding = blur.clamp(0.0, 32.0).ceil() * 2.0;
            let area = rect.intersect(command.clip.unwrap_or(rect));
            let region = scissor(
                Some(Rect {
                    x: area.x,
                    y: area.y - padding,
                    width: area.width,
                    height: area.height + padding * 2.0,
                }),
                frame.target.size(),
            );
            let mut pass = begin_pass(
                frame.encoder,
                &view,
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            );
            if let Some((x, y, w, h)) = region {
                pass.set_pipeline(&self.glass_pipeline);
                pass.set_bind_group(0, &bindings, &[]);
                pass.set_scissor_rect(x, y, w, h);
                pass.draw(0..6, 0..1);
            }
            drop(pass);
            Some(view)
        } else {
            None
        };
        let buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("UI shape parameters"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI shape"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            horizontal.as_ref().unwrap_or(&snapshot_view),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&snapshot_view),
                    },
                ],
            });
        let mut pass = begin_pass(frame.encoder, frame.target.view, wgpu::LoadOp::Load);
        pass.set_pipeline(if glass {
            &self.glass_pipeline
        } else {
            &self.pipeline
        });
        pass.set_bind_group(0, &bindings, &[]);
        pass.set_scissor_rect(clip.0, clip.1, clip.2, clip.3);
        pass.draw(0..6, 0..1);
        Ok(())
    }
    fn shadow(&self, frame: &mut WgpuFrame<'_>, command: &DrawCommand) -> Result<()> {
        let rect = command.rect;
        let blur = command.value.max(0.0);
        if rect.width <= 0.0 || rect.height <= 0.0 || blur <= 0.0 {
            return Ok(());
        }
        let (width, height) = frame.target.size();
        let shadow_rect = Rect {
            x: rect.x - blur,
            y: rect.y - blur,
            width: rect.width + 2.0 * blur,
            height: rect.height + 2.0 * blur,
        };
        let Some(clip) = scissor(
            Some(shadow_rect.intersect(command.clip.unwrap_or(shadow_rect))),
            frame.target.size(),
        ) else {
            return Ok(());
        };
        let snapshot_view = self
            .dummy_backdrop
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.format),
                ..Default::default()
            });
        let params = [
            [rect.x, rect.y, rect.width, rect.height],
            target_color(command.style.fill, self.format),
            [0.0; 4],
            [width as f32, height as f32, command.style.radius, blur],
            [0.0, 0.0, 0.0, 2.0],
            [0.0; 4],
        ];
        let buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("UI shadow parameters"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI shadow"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&snapshot_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&snapshot_view),
                    },
                ],
            });
        let mut pass = begin_pass(frame.encoder, frame.target.view, wgpu::LoadOp::Load);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.set_scissor_rect(clip.0, clip.1, clip.2, clip.3);
        pass.draw(0..6, 0..1);
        Ok(())
    }
    fn text(&mut self, frame: &mut WgpuFrame<'_>, command: &DrawCommand) -> Result<()> {
        if command.text.is_empty() && command.rich_text.is_none() {
            return Ok(());
        }
        let r = command.rect;
        let text_clip = intersect(command.clip.unwrap_or(r), r);
        let Some(clip) = scissor(Some(text_clip), frame.target.size()) else {
            return Ok(());
        };
        let mut buffer = glyphon::Buffer::new(
            &mut self.text.font_system,
            glyphon::Metrics::new(command.style.font_size, command.style.font_size * 1.25),
        );
        let is_centered = command.style.text_align == ui_kit_core::TextAlign::Center;
        let buffer_width = if is_centered {
            r.width.max(1.0)
        } else {
            (r.width - 2.0 * command.style.text_padding).max(1.0)
        };
        buffer.set_size(Some(buffer_width), Some(r.height.max(1.0)));
        if let Some(rich) = &command.rich_text {
            rich_text::fill(&mut buffer, &rich.document, command.style.font_size);
        } else {
            buffer.set_text(
                &command.text,
                &glyphon::Attrs::new().weight(glyphon::Weight(command.style.font_weight)),
                glyphon::Shaping::Advanced,
                None,
            );
        }
        if command.rich_text.is_none() {
            let align = match command.style.text_align {
                ui_kit_core::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                ui_kit_core::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                ui_kit_core::TextAlign::Right => glyphon::cosmic_text::Align::Right,
            };
            for line in &mut buffer.lines {
                line.set_align(Some(align));
            }
        }
        buffer.shape_until_scroll(&mut self.text.font_system, false);
        let text_height = buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0, f32::max);
        let top = if command.rich_text.is_some() {
            r.y
        } else {
            let mut runs = buffer.layout_runs();
            if let Some(first) = runs.next() {
                if runs.next().is_none() {
                    // Single-line controls (buttons, inputs, labels): center the primary
                    // visual mass (cap-height to baseline) inside the control height, so
                    // distance from top to capital letters equals distance from baseline to bottom.
                    let cap_height = (command.style.font_size * 0.714).round();
                    (r.y + (r.height + cap_height) * 0.5 - first.line_y).round()
                } else {
                    (r.y + (r.height - text_height).max(0.0) * 0.5).round()
                }
            } else {
                r.y
            }
        };
        if let Some(rich) = &command.rich_text {
            if let Some(position) = rich.cursor {
                let text = rich.document.text();
                let cursor = rich_text::cursor(&text, position);
                let anchor = rich_text::cursor(&text, rich.anchor);
                let (start, end) = if position < rich.anchor {
                    (cursor, anchor)
                } else {
                    (anchor, cursor)
                };
                let mut mark = command.clone();
                mark.clip = Some(text_clip);
                mark.style.material = Material::Solid;
                mark.style.gradient_end = None;
                mark.style.stroke_width = 0.0;
                mark.focused = false;
                if position != rich.anchor {
                    for run in buffer.layout_runs() {
                        for (x, width) in run.highlight(start, end) {
                            self.shape(
                                frame,
                                &mark,
                                Rect {
                                    x: r.x + command.style.text_padding + x,
                                    y: top + run.line_top,
                                    width,
                                    height: run.line_height,
                                },
                                [0.23, 0.51, 0.96, 0.3],
                                [0.0; 4],
                                0.0,
                                false,
                            )?;
                        }
                    }
                }
                if let Some((x, y)) = buffer.cursor_position(&cursor).or_else(|| {
                    if text.is_empty() {
                        Some((0.0, 0.0))
                    } else {
                        None
                    }
                }) {
                    self.shape(
                        frame,
                        &mark,
                        Rect {
                            x: r.x + command.style.text_padding + x,
                            y: top + y,
                            width: 1.5,
                            height: command.style.font_size * 1.25,
                        },
                        command.style.foreground,
                        [0.0; 4],
                        0.0,
                        false,
                    )?;
                }
            }
        }
        // Per-encode viewport/instance buffers permit multiple UI targets before submission.
        let mut viewport = glyphon::Viewport::new(&self.context.device, &self.text.cache);
        let (width, height) = frame.target.size();
        viewport.update(&self.context.queue, glyphon::Resolution { width, height });
        let mut renderer = glyphon::TextRenderer::new(
            &mut self.text.atlas,
            &self.context.device,
            Default::default(),
            None,
        );
        renderer.prepare(
            &self.context.device,
            &self.context.queue,
            &mut self.text.font_system,
            &mut self.text.atlas,
            &viewport,
            [glyphon::TextArea {
                buffer: &buffer,
                left: r.x
                    + if command.kind == "checkbox" {
                        44.0
                    } else if is_centered {
                        0.0
                    } else {
                        command.style.text_padding
                    },
                top,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: clip.0 as i32,
                    top: clip.1 as i32,
                    right: (clip.0 + clip.2) as i32,
                    bottom: (clip.1 + clip.3) as i32,
                },
                default_color: glyphon::Color::rgba(
                    (command.style.foreground[0] * 255.0) as u8,
                    (command.style.foreground[1] * 255.0) as u8,
                    (command.style.foreground[2] * 255.0) as u8,
                    (command.style.foreground[3] * 255.0) as u8,
                ),
                custom_glyphs: &[],
            }],
            &mut self.text.swash_cache,
        )?;
        let mut pass = begin_pass(frame.encoder, frame.target.view, wgpu::LoadOp::Load);
        renderer.render(&self.text.atlas, &viewport, &mut pass)?;
        Ok(())
    }
}

impl<C: CommandRenderer> UiRenderer<WgpuFrame<'_>> for WgpuRenderer<C> {
    type Error = anyhow::Error;
    fn render(&mut self, frame: &mut WgpuFrame<'_>, commands: &[DrawCommand]) -> Result<()> {
        ensure!(
            frame.target.format == self.format,
            "UI target view format must match renderer format"
        );
        ensure!(
            frame.target.texture.sample_count() == 1
                && frame.target.texture.depth_or_array_layers() == 1
                && frame.target.texture.dimension() == wgpu::TextureDimension::D2,
            "UI target must be a single-sample 2D texture"
        );
        ensure!(
            frame
                .target
                .texture
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT),
            "UI target needs RENDER_ATTACHMENT usage"
        );
        if let Some(color) = frame.clear {
            drop(begin_pass(
                frame.encoder,
                frame.target.view,
                wgpu::LoadOp::Clear(color),
            ))
        }
        for command in commands {
            let blur = if command.kind == "shadow" {
                command.value.max(0.0)
            } else {
                0.0
            };
            let bounds = Rect {
                x: command.rect.x - blur,
                y: command.rect.y - blur,
                width: command.rect.width + 2.0 * blur,
                height: command.rect.height + 2.0 * blur,
            };
            let visible = bounds
                .intersect(command.clip.unwrap_or(bounds))
                .intersect(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: frame.target.size().0 as f32,
                    height: frame.target.size().1 as f32,
                });
            if visible.width <= 0.0 || visible.height <= 0.0 {
                continue;
            }
            if self.custom.render(&self.context, frame, command)? {
                continue;
            }
            ensure!(
                !matches!(command.style.material, Material::Custom { .. }),
                "unhandled custom UI material"
            );
            ensure!(
                [
                    "panel",
                    "label",
                    "button",
                    "checkbox",
                    "text_input",
                    "slider",
                    "progress",
                    "shadow"
                ]
                .contains(&command.kind.as_str()),
                "unhandled UI command: {}",
                command.kind
            );
            if command.kind == "shadow" {
                self.shadow(frame, command)?;
                continue;
            }
            let r = command.rect;
            if command.kind != "label" {
                let mut fill = command.style.fill;
                if command.hovered {
                    for c in &mut fill[..3] {
                        *c = (*c + if command.active { 0.12 } else { 0.06 }).min(1.0)
                    }
                }
                let stroke = if command.focused {
                    [0.5, 0.9, 1.0, 1.0]
                } else {
                    command.style.stroke
                };
                self.shape(
                    frame,
                    command,
                    r,
                    fill,
                    stroke,
                    command.style.radius,
                    matches!(command.style.material, Material::Glass { .. }),
                )?;
            }
            let accent = command.style.accent;
            if command.kind == "checkbox" {
                let color = if command.value > 0.5 {
                    accent
                } else {
                    [0.2, 0.25, 0.3, 1.0]
                };
                self.shape(
                    frame,
                    command,
                    Rect {
                        x: r.x + 16.0,
                        y: r.y + (r.height - 18.0) * 0.5,
                        width: 18.0,
                        height: 18.0,
                    },
                    color,
                    color,
                    3.0,
                    false,
                )?;
            }
            if command.kind == "slider" || command.kind == "progress" {
                let track_height = 6.0f32.min(r.height);
                self.shape(
                    frame,
                    command,
                    Rect {
                        x: r.x,
                        y: r.y + (r.height - track_height) * 0.5,
                        width: r.width * command.value.clamp(0.0, 1.0),
                        height: track_height,
                    },
                    accent,
                    accent,
                    2.0,
                    false,
                )?;
            }
            self.text(frame, command)?;
        }
        Ok(())
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect {
        x,
        y,
        width: ((a.x + a.width).min(b.x + b.width) - x).max(0.0),
        height: ((a.y + a.height).min(b.y + b.height) - y).max(0.0),
    }
}

fn scissor(clip: Option<Rect>, size: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        width: size.0 as f32,
        height: size.1 as f32,
    };
    let r = intersect(clip.unwrap_or(bounds), bounds);
    if ![r.x, r.y, r.width, r.height].iter().all(|x| x.is_finite())
        || r.width <= 0.0
        || r.height <= 0.0
    {
        return None;
    }
    let x = r.x.floor() as u32;
    let y = r.y.floor() as u32;
    Some((
        x,
        y,
        ((r.x + r.width).ceil() as u32).min(size.0) - x,
        ((r.y + r.height).ceil() as u32).min(size.1) - y,
    ))
}

fn begin_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Prism UI"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

fn validate_format(format: wgpu::TextureFormat) -> Result<()> {
    ensure!(
        matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
                | wgpu::TextureFormat::Rgba16Float
        ),
        "unsupported UI texture format"
    );
    Ok(())
}

fn target_is_linear(format: wgpu::TextureFormat) -> bool {
    format.is_srgb() || format == wgpu::TextureFormat::Rgba16Float
}

fn target_color(mut color: [f32; 4], format: wgpu::TextureFormat) -> [f32; 4] {
    if target_is_linear(format) {
        for channel in &mut color[..3] {
            *channel = if *channel <= 0.04045 {
                *channel / 12.92
            } else {
                ((*channel + 0.055) / 1.055).powf(2.4)
            };
        }
    }
    color
}
