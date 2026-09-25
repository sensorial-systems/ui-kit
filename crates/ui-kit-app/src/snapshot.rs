//! An application drawn to pixels without a window: its frames run on a
//! device of their own and the last one is read back, to compare with a
//! reference or to test what it draws.

use crate::{resolve_transforms, App, FrameContext, Gpu};
use anyhow::{anyhow, Context, Result};
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::{Input, Rect};
use ui_kit_wgpu::{TextureTarget, VectorRenderer, WgpuFrame, WgpuRenderer};

/// How big to draw, and for how many frames.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    /// Frames to run before the last is kept: time for what the first ones
    /// started -- loads, animations -- to finish.
    pub frames: u32,
}

impl Snapshot {
    /// The size and frame count `--size WxH --frames N` ask for, if the
    /// command line has `--snapshot`, with the path after it.
    pub fn from_args() -> Option<(String, Self)> {
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1).cloned())
        };
        let out = value("--snapshot")?;
        let (width, height) = value("--size")
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            })
            .unwrap_or((1280, 800));
        let frames = value("--frames").and_then(|f| f.parse().ok()).unwrap_or(30);
        Some((
            out,
            Self {
                width,
                height,
                frames,
            },
        ))
    }
}

/// Run `app` for the snapshot's frames, 60 to a second, and return the last
/// one's pixels: RGBA, 8 bits a channel, row after row.
pub fn render<A: App>(app: &mut A, snapshot: Snapshot) -> Result<Vec<u8>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&app.device_descriptor(&adapter)))?;
    let gpu = Gpu {
        instance: instance.into(),
        adapter: adapter.into(),
        device: device.into(),
        queue: queue.into(),
    };
    app.gpu_ready(&gpu);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let context = gpu.context();
    let mut renderer = WgpuRenderer::with_custom(&context, format, VectorRenderer::new(&context, format))?;
    app.fonts_ready(&renderer);
    app.text_measure_ready(renderer.text_measure());

    let (width, height) = (snapshot.width as f32, snapshot.height as f32);
    let mut commands = Vec::new();
    for frame in 0..snapshot.frames.max(1) {
        crate::task::run_ready();
        let time = frame as f64 / 60.0;
        let input = Input {
            time: Some(time),
            viewport: Some(Rect::new(0.0, 0.0, width, height)),
            ..Default::default()
        };
        let ctx = FrameContext::new(input, width, height, time as f32, 1.0, true);
        app.prepare(&mut renderer);
        commands = app.frame(&ctx);
        resolve_transforms(&mut commands);
        // Let what the frame started (loads from storage) finish before the next.
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // What the last frame asked to show is shown.
    app.prepare(&mut renderer);

    let target = renderer.create_texture(snapshot.width, snapshot.height)?;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(
        &mut WgpuFrame {
            encoder: &mut encoder,
            target: TextureTarget::from_texture2d(&target)?,
            clear: Some(wgpu::Color::BLACK),
        },
        &commands,
    )?;
    let pitch = (snapshot.width * 4).div_ceil(256) * 256;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("snapshot"),
        size: (pitch * snapshot.height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.texture.as_ref().ok_or_else(|| anyhow!("no texture"))?.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(snapshot.height),
            },
        },
        wgpu::Extent3d {
            width: snapshot.width,
            height: snapshot.height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    Ok(mapped
        .chunks(pitch as usize)
        .flat_map(|row| row[..(snapshot.width * 4) as usize].iter().copied())
        .collect())
}

/// Write RGBA pixels, row after row, as a PNG.
pub fn save_png(path: &str, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    let file = std::fs::File::create(path).with_context(|| path.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}
