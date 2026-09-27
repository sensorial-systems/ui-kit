//! A frame drawn only where it changed, over the frame before, looks the
//! same as the frame drawn whole.

use anyhow::Result;
use std::sync::Arc;
use ui_kit_app::damage_region;
use ui_kit_core::{Material, Point, Rect};
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::{vector, DrawCommand, Input, Ui};
use ui_kit_wgpu::{GpuContext, TextureTarget, VectorRenderer, WgpuFrame, WgpuRenderer};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 400;
const BACKGROUND: [f32; 4] = [0.05, 0.06, 0.08, 1.0];

fn gpu() -> Result<GpuContext> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        Ok(GpuContext::new(Arc::new(device), Arc::new(queue)))
    })
}

/// A scene with a little of everything; `moved` shifts one button and
/// changes a line of text.
fn scene(moved: bool) -> Vec<DrawCommand> {
    let mut ui = Ui::default();
    ui.begin(Input::default());
    ui.style.fill = [0.2, 0.3, 0.5, 1.0];
    ui.shadow(Rect::new(20.0, 20.0, 120.0, 60.0), 12.0);
    ui.panel(Rect::new(20.0, 20.0, 120.0, 60.0));
    ui.label(Rect::new(24.0, 30.0, 110.0, 30.0), if moved { "Changed" } else { "Label" });
    let x = if moved { 190.0 } else { 170.0 };
    ui.button("moving", Rect::new(x, 30.0, 100.0, 36.0), "Button");
    vector::circle(&mut ui, Point::new(80.0, 150.0), 24.0, [0.9, 0.6, 0.2, 1.0]);
    // Glass over the button's path: drawn again whole when it moves.
    ui.style.fill = [1.0, 1.0, 1.0, 0.2];
    ui.style.material = Material::Glass { blur: 6.0 };
    ui.panel(Rect::new(160.0, 50.0, 140.0, 100.0));
    ui.style.material = Material::Solid;
    ui.label(Rect::new(170.0, 160.0, 140.0, 30.0), "Unchanged");
    ui.end().to_vec()
}

fn read(gpu: &GpuContext, texture: &wgpu::Texture) -> Result<Vec<u8>> {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (WIDTH * HEIGHT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let data = buffer.slice(..).get_mapped_range()?.to_vec();
    Ok(data)
}

#[test]
fn a_frame_drawn_where_it_changed_looks_like_one_drawn_whole() -> Result<()> {
    let gpu = gpu()?;
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let mut renderer = WgpuRenderer::with_custom(&gpu, format, VectorRenderer::new(&gpu, format))?;
    let (before, after) = (scene(false), scene(true));
    let clear = wgpu::Color {
        r: BACKGROUND[0] as f64,
        g: BACKGROUND[1] as f64,
        b: BACKGROUND[2] as f64,
        a: BACKGROUND[3] as f64,
    };
    let mut draw = |target: &ui_kit_wgpu::Texture2d, commands: &[DrawCommand], clear: Option<wgpu::Color>| -> Result<()> {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(target)?,
                clear,
            },
            commands,
        )?;
        gpu.queue.submit([encoder.finish()]);
        Ok(())
    };

    let whole = WgpuRenderer::new(&gpu, format)?.create_texture(WIDTH, HEIGHT)?;
    draw(&whole, &after, Some(clear))?;

    let part = WgpuRenderer::new(&gpu, format)?.create_texture(WIDTH, HEIGHT)?;
    draw(&part, &before, Some(clear))?;
    let (region, commands) = damage_region(&before, &after, (WIDTH, HEIGHT), BACKGROUND)
        .expect("the frames differ");
    assert!(
        region.width * region.height < (WIDTH * HEIGHT) as f32 * 0.6,
        "only a part is drawn: {region:?}"
    );
    draw(&part, &commands, None)?;

    let whole = read(&gpu, whole.texture.as_ref().unwrap())?;
    let part = read(&gpu, part.texture.as_ref().unwrap())?;
    let worst = whole
        .iter()
        .zip(&part)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(worst <= 1, "the frames differ by up to {worst} in a channel");
    Ok(())
}
