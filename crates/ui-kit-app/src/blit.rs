//! Copying the UI's texture onto a surface that cannot be copied into.
//!
//! The UI is drawn into a texture of its own, so that glass can read back
//! what is behind it, and then put on the surface. Where the surface takes
//! copies that is one `copy_texture_to_texture`; where it does not (a
//! browser's canvas) it is this: one triangle covering the surface, reading
//! the texture pixel for pixel.
//!
//! It is also how a window that shows through gets round corners where the
//! system does not round it: what is outside them is cleared as it is copied.

const SHADER: &str = r#"
struct Corners {
    size: vec2<f32>,
    radius: f32,
    unused: f32,
};

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<uniform> corners: Corners;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let color = textureLoad(source, vec2<i32>(position.xy), 0);
    if (corners.radius <= 0.0) {
        return color;
    }
    // How far outside the rounded rectangle of the surface this pixel is;
    // the colour is premultiplied, so all of it fades with the coverage.
    let half = corners.size * 0.5;
    let q = abs(position.xy - half) - (half - vec2<f32>(corners.radius));
    let outside = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - corners.radius;
    return color * clamp(0.5 - outside, 0.0, 1.0);
}
"#;

pub(crate) struct Blit {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// The surface's size and the corners' radius, physical pixels.
    corners: wgpu::Buffer,
}

impl Blit {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI blit"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI blit"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI blit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let corners = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI blit corners"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, layout, corners }
    }

    /// Draw `source` over the whole of `target`, which has its size, `width`
    /// by `height`, with its corners cut round by `radius`, all physical
    /// pixels: square at 0.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        (width, height, radius): (u32, u32, f32),
    ) {
        let corners = [width as f32, height as f32, radius, 0.0];
        let bytes: Vec<u8> = corners.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&self.corners, 0, &bytes);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI blit"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.corners.as_entire_binding(),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
