use anyhow::Result;
use shared_gallery::{gallery_theme, settings_view, Settings};
use std::sync::Arc;
use ui_kit_core::Rect;
use ui_kit_immediate::{composition::UiRenderer, ImmediateHost, Input};
use ui_kit_wgpu::{GpuContext, TextureTarget, WgpuFrame, WgpuRenderer};

fn main() -> Result<()> {
    pollster::block_on(run())
}
async fn run() -> Result<()> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance.request_adapter(&Default::default()).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
    let mut renderer = WgpuRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm)?;
    let texture = renderer.create_texture(768, 512)?;
    let state = Settings::default();
    let view = settings_view(&state);
    let theme = gallery_theme();
    let frame = ImmediateHost::default()
        .frame(
            &view,
            &theme,
            Input::default(),
            Rect {
                x: 24.0,
                y: 24.0,
                width: 720.0,
                height: 464.0,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(
        &mut WgpuFrame {
            encoder: &mut encoder,
            target: TextureTarget::from_texture2d(&texture)?,
            clear: Some(wgpu::Color {
                r: 0.08,
                g: 0.12,
                b: 0.22,
                a: 1.0,
            }),
        },
        &frame.commands,
    )?;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Gallery readback"),
        size: 768 * 512 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.texture.as_ref().unwrap().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(768 * 4),
                rows_per_image: Some(512),
            },
        },
        wgpu::Extent3d {
            width: 768,
            height: 512,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let (send, recv) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = send.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    recv.recv()??;
    let pixels = buffer.slice(..).get_mapped_range()?;
    std::fs::create_dir_all("target")?;
    let mut png = png::Encoder::new(
        std::fs::File::create("target/shared-gallery.png")?,
        768,
        512,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()?.write_image_data(&pixels)?;
    use dioxus::prelude::*;
    use ui_kit_dioxus::SharedView;
    let mut dom = VirtualDom::new(
        || rsx! {SharedView{view:settings_view(&Settings::default()),theme:gallery_theme(),on_action:move |_|{}}},
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    std::fs::write("target/shared-gallery.html",format!("<!doctype html><meta charset='utf-8'><title>Shared UI gallery</title><style>body{{background:#101b30;color:white;font:16px system-ui;padding:24px}}main{{display:flex;gap:24px;flex-wrap:wrap}}section{{width:768px}}button,input{{font:inherit}}</style><h1>One view · Dioxus and wgpu</h1><p>Static comparison. Run the web gallery for live application actions.</p><main><section><h2>Dioxus DOM</h2>{html}</section><section><h2>Immediate → wgpu texture</h2><img width='768' src='shared-gallery.png'></section></main>"))?;
    println!("Rendered target/shared-gallery.png and target/shared-gallery.html");
    Ok(())
}
