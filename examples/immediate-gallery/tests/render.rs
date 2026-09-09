use anyhow::Result;
use immediate_gallery::{geometry::GeometryRenderer, Gallery};
use std::sync::Arc;
use ui_kit_core::{Material, Rect};
use ui_kit_immediate::{composition::UiRenderer, Input, Ui};
use ui_kit_wgpu::{GpuContext, TextureTarget, WgpuFrame, WgpuRenderer};

fn gpu() -> Result<GpuContext> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        Ok(GpuContext::new(Arc::new(device), Arc::new(queue)))
    })
}

#[test]
fn text_is_visually_centered_and_vectors_have_partial_pixel_coverage() -> Result<()> {
    let gpu = gpu()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        WgpuRenderer::with_custom(&gpu, format, GeometryRenderer::new(&gpu, format))?;
    let texture = renderer.create_texture(256, 256)?;
    let mut ui = Ui::default();
    ui.begin(Input::default());
    ui.style.fill = [0.0; 4];
    ui.style.stroke_width = 0.0;
    ui.style.foreground = [1.0; 4];
    for (i, text) in ["Primary", "overview.local", "2026-07-16 18:00"]
        .iter()
        .enumerate()
    {
        ui.button(
            &format!("text-{i}"),
            Rect {
                x: 0.,
                y: i as f32 * 50.,
                width: 256.,
                height: 40.,
            },
            text,
        );
    }
    ui.style.fill = [1.0; 4];
    ui.style.material = Material::Custom {
        name: "gallery.triangles".into(),
        parameters: vec![
            10., 180., 210., 220., 210., 222., 10., 180., 210., 222., 10., 182.,
        ],
    };
    ui.panel(Rect {
        x: 9.,
        y: 179.,
        width: 203.,
        height: 45.,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(
        &mut WgpuFrame {
            encoder: &mut encoder,
            target: TextureTarget::from_texture2d(&texture)?,
            clear: Some(wgpu::Color::BLACK),
        },
        ui.end(),
    )?;
    let pixels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.texture.as_ref().unwrap().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &pixels,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024),
                rows_per_image: Some(256),
            },
        },
        wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    pixels.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let data = pixels.slice(..).get_mapped_range()?;
    for i in 0..3 {
        let rows: Vec<_> = (i * 50..i * 50 + 40)
            .filter(|y| (0..256).any(|x| data[y * 1024 + x * 4] > 32))
            .collect();
        assert!(!rows.is_empty());
        let center = (rows[0] + rows.last().unwrap() + 1) as f32 * 0.5;
        assert!(
            (center - (i * 50 + 20) as f32).abs() <= 1.0,
            "text {i} center: {center}"
        );
    }
    let partial = (178..225)
        .flat_map(|y| (8..213).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let c = data[y * 1024 + x * 4];
            c > 0 && c < 255
        })
        .count();
    assert!(
        partial > 150,
        "diagonal must have fractional coverage: {partial}"
    );
    Ok(())
}

#[test]
#[ignore = "opt-in wall-clock rendering diagnostic"]
fn profile_flow_frames() -> Result<()> {
    let gpu = gpu()?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        WgpuRenderer::with_custom(&gpu, format, GeometryRenderer::new(&gpu, format))?;
    let texture = renderer.create_texture(1200, 850)?;
    let mut gallery = Gallery::default();
    gallery.frame(Input::default(), 1200., 850., 0.);
    gallery.scroll = gallery.section_positions["9. Graph Components"];
    let mut times = Vec::new();
    for i in 0..25 {
        let start = std::time::Instant::now();
        let commands = gallery.frame(Input::default(), 1200., 850., i as f32 / 60.);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&texture)?,
                clear: Some(gallery.background()),
            },
            &commands,
        )?;
        gpu.queue.submit([encoder.finish()]);
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        if i >= 5 {
            times.push(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "Flow frame CPU+GPU including synchronous wait: median {:.2} ms, p95 {:.2} ms",
        times[times.len() / 2],
        times[18]
    );
    Ok(())
}
