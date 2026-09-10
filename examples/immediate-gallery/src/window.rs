use anyhow::Result;
use immediate_gallery::geometry::GeometryRenderer;
use immediate_gallery::Gallery;
use std::sync::Arc;
use ui_kit_app::{App, AppConfig, FrameContext};
use ui_kit_core::Point;
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::{DrawCommand, Input};
use ui_kit_wgpu::{GpuContext, TextureTarget, WgpuFrame, WgpuRenderer};

struct GalleryApp {
    gallery: Gallery,
}

impl App for GalleryApp {
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand> {
        self.gallery.shift = ctx.input.extend_selection;
        self.gallery
            .frame(ctx.input.clone(), ctx.width, ctx.height, ctx.elapsed)
    }

    fn on_scroll(&mut self, delta: f32) {
        self.gallery.scroll_by(delta);
    }

    fn on_focus_lost(&mut self) {
        self.gallery.cancel_drag();
    }
}

pub fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--snapshot") {
        return snapshot(
            args.get(index + 1)
                .ok_or_else(|| anyhow::anyhow!("--snapshot requires a PNG path"))?,
            &args,
        );
    }

    let gallery = Gallery::default();
    let bg = gallery.background();
    let config = AppConfig::new("UI Kit Component Gallery — Immediate / wgpu")
        .with_size(1200.0, 850.0)
        .with_background([
            bg.r as f32,
            bg.g as f32,
            bg.b as f32,
            bg.a as f32,
        ]);

    ui_kit_app::run_with_custom(GalleryApp { gallery }, config, |gpu, format| {
        GeometryRenderer::new(gpu, format)
    })
}

fn snapshot(path: &str, args: &[String]) -> Result<()> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
    let mut renderer = WgpuRenderer::with_custom(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        GeometryRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm),
    )?;
    let mut gallery = Gallery::default();
    gallery.dark = args.iter().any(|a| a == "--dark");
    gallery.frame(Input::default(), 1200.0, 850.0, 0.0);
    if let Some(i) = args.iter().position(|a| a == "--scroll") {
        gallery.scroll_by(
            args.get(i + 1)
                .ok_or_else(|| anyhow::anyhow!("missing scroll value"))?
                .parse()?,
        );
    }
    if let Some(i) = args.iter().position(|a| a == "--section") {
        let section = args
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("missing section name"))?;
        gallery.scroll = *gallery
            .section_positions
            .get(section)
            .ok_or_else(|| anyhow::anyhow!("unknown section: {section}"))?;
    }
    // Drive the same input path as the window for reproducible component captures.
    for pair in args.windows(2).filter(|pair| pair[0] == "--click") {
        let commands = gallery.frame(Input::default(), 1200.0, 850.0, 0.0);
        let rect = commands
            .iter()
            .rev()
            .find(|c| c.text == pair[1])
            .or_else(|| commands.iter().rev().find(|c| c.text.starts_with(&pair[1])))
            .ok_or_else(|| anyhow::anyhow!("control not found: {}", pair[1]))?
            .rect;
        if rect.y < 0.0 || rect.y + rect.height > 850.0 {
            gallery.scroll_by(rect.y - 200.0);
        }
        let commands = gallery.frame(Input::default(), 1200.0, 850.0, 0.0);
        let rect = commands
            .iter()
            .rev()
            .find(|c| c.text == pair[1])
            .or_else(|| commands.iter().rev().find(|c| c.text.starts_with(&pair[1])))
            .unwrap()
            .rect;
        let pointer = Some(Point::new(
            rect.x + rect.width * 0.5,
            rect.y + rect.height * 0.5,
        ));
        gallery.frame(
            Input {
                pointer,
                down: true,
                ..Default::default()
            },
            1200.0,
            850.0,
            0.0,
        );
        gallery.frame(
            Input {
                pointer,
                ..Default::default()
            },
            1200.0,
            850.0,
            0.0,
        );
    }
    let commands = gallery.frame(Input::default(), 1200.0, 850.0, 0.0);
    let target = renderer.create_texture(1200, 850)?;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(
        &mut WgpuFrame {
            encoder: &mut encoder,
            target: TextureTarget::from_texture2d(&target)?,
            clear: Some(gallery.background()),
        },
        &commands,
    )?;
    let pitch = 4864;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: pitch * 850,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.texture.as_ref().unwrap().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch as u32),
                rows_per_image: Some(850),
            },
        },
        wgpu::Extent3d {
            width: 1200,
            height: 850,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let data: Vec<u8> = mapped
        .chunks(pitch as usize)
        .flat_map(|row| row[..4800].iter().copied())
        .collect();
    let mut png = png::Encoder::new(std::fs::File::create(path)?, 1200, 850);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()?.write_image_data(&data)?;
    println!("Wrote {path}");
    Ok(())
}
