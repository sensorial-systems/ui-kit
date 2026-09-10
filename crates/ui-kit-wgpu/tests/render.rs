use anyhow::Result;
use std::sync::Arc;
use ui_kit_immediate::{
    composition::UiRenderer,
    immediate::{DrawCommand, Input, Material, Rect, Ui},
};
use ui_kit_wgpu::GpuContext;
use ui_kit_wgpu::{CommandRenderer, TextureTarget, WgpuFrame, WgpuRenderer};

#[test]
fn gaussian_glass_smooths_edges_and_preserves_rounded_corners() -> Result<()> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
        let mut renderer = WgpuRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm)?;
        let texture = renderer.create_texture(64, 64)?;
        let mut ui = Ui::default();
        ui.begin(Input::default());
        ui.style.fill = [1.0, 0.0, 0.0, 1.0];
        ui.style.stroke_width = 0.0;
        ui.style.radius = 0.0;
        ui.panel(Rect {
            x: 0.0,
            y: 0.0,
            width: 32.0,
            height: 64.0,
        });
        ui.style.material = Material::Glass { blur: 4.0 };
        ui.style.fill = [0.0; 4];
        ui.style.radius = 12.0;
        ui.panel(Rect {
            x: 8.0,
            y: 8.0,
            width: 48.0,
            height: 48.0,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&texture)?,
                clear: Some(wgpu::Color::BLUE),
            },
            ui.end(),
        )?;
        let pixels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.texture.as_ref().unwrap().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &pixels,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
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
        let pixel = |x: usize, y: usize| &data[y * 256 + x * 4..y * 256 + x * 4 + 4];
        assert_eq!(
            pixel(8, 8),
            [255, 0, 0, 255],
            "rounded cutout must preserve original backdrop"
        );
        assert_eq!(
            pixel(62, 32),
            [0, 0, 255, 255],
            "pixels outside glass must remain untouched"
        );
        assert!(
            (100..=150).contains(&pixel(31, 32)[2]),
            "edge must blend both colors"
        );
        for x in 24..39 {
            assert!(
                pixel(x + 1, 32)[2] > pixel(x, 32)[2],
                "Gaussian edge must be smooth and monotonic"
            );
            assert_eq!(pixel(x, 32)[3], 255);
        }
        Ok(())
    })
}

struct Recorder {
    seen: Vec<String>,
    handle: bool,
}
impl CommandRenderer for Recorder {
    fn render(
        &mut self,
        _: &GpuContext,
        _: &mut WgpuFrame<'_>,
        command: &DrawCommand,
    ) -> Result<bool> {
        self.seen.push(command.kind.clone());
        Ok(self.handle && command.kind == "custom")
    }
}

#[test]
fn shared_device_texture_pixels_and_composition() -> Result<()> {
    pollster::block_on(async {
        let instance = Arc::new(wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_without_display_handle(),
        ));
        let adapter = Arc::new(instance.request_adapter(&Default::default()).await?);
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let context = GpuContext::new(Arc::new(device), Arc::new(queue));
        let scope = context
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let mut renderer = WgpuRenderer::with_custom(
            &context,
            wgpu::TextureFormat::Rgba8Unorm,
            (
                Recorder {
                    seen: vec![],
                    handle: false,
                },
                Recorder {
                    seen: vec![],
                    handle: true,
                },
            ),
        )?;
        assert!(Arc::ptr_eq(&context.device, &renderer.context().device));
        assert!(Arc::ptr_eq(&context.queue, &renderer.context().queue));
        assert!(renderer.create_texture(0, 64).is_err());
        let texture = renderer.create_texture(64, 64)?;
        let second = renderer.create_texture(128, 64)?;
        let mut ui = Ui::default();
        ui.begin(Input::default());
        ui.style.fill = [1.0, 0.0, 0.0, 1.0];
        ui.style.stroke = [1.0, 0.0, 0.0, 1.0];
        ui.style.radius = 0.0;
        ui.clip = Some(Rect {
            x: 8.0,
            y: 8.0,
            width: 24.0,
            height: 24.0,
        });
        ui.panel(Rect {
            x: 0.0,
            y: 0.0,
            width: 64.0,
            height: 64.0,
        });
        ui.clip = None;
        ui.style.material = Material::Glass { blur: 0.0 };
        ui.style.fill = [0.0, 0.0, 0.0, 0.0];
        ui.style.stroke = [0.0, 0.0, 0.0, 0.0];
        ui.panel(Rect {
            x: 12.0,
            y: 12.0,
            width: 12.0,
            height: 12.0,
        });
        // A custom command is handled by the second renderer in the chain.
        let mut custom = ui.commands[0].clone();
        custom.kind = "custom".into();
        custom.style.material = Material::Custom {
            name: "test".into(),
            parameters: vec![],
        };
        ui.commands.push(custom);
        let commands = ui.end();
        let mut encoder = context.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&texture)?,
                clear: Some(wgpu::Color::BLUE),
            },
            commands,
        )?;
        let pixels = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 2 * 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.texture.as_ref().unwrap().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &pixels,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        // Reuse one backend for another target before submitting either frame.
        let mut transparent_commands = commands.to_vec();
        transparent_commands[0].style.fill[3] = 0.5;
        transparent_commands[0].style.stroke[3] = 0.5;
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&second)?,
                clear: Some(wgpu::Color::TRANSPARENT),
            },
            &transparent_commands,
        )?;
        encoder.copy_texture_to_buffer(
            second.texture.as_ref().unwrap().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &pixels,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 64 * 256,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        context.queue.submit([encoder.finish()]);
        let (send, recv) = std::sync::mpsc::channel();
        pixels.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = send.send(r);
        });
        context.device.poll(wgpu::PollType::wait_indefinitely())?;
        recv.recv()??;
        let data = pixels.slice(..).get_mapped_range()?;
        let pixel = |x: usize, y: usize| &data[y * 256 + x * 4..y * 256 + x * 4 + 4];
        assert_eq!(
            pixel(2, 2),
            [0, 0, 255, 255],
            "clear survives outside clipping"
        );
        assert_eq!(
            pixel(10, 10),
            [255, 0, 0, 255],
            "solid shape paints inside clipping"
        );
        assert_eq!(
            pixel(18, 18),
            [255, 0, 0, 255],
            "glass samples earlier drawing"
        );
        assert_eq!(renderer.custom.0.seen, renderer.custom.1.seen);
        assert_eq!(
            pixel(18, 64 + 18),
            pixel(10, 64 + 10),
            "glass preserves premultiplied RGB and alpha on a transparent target"
        );
        assert!((127..=128).contains(&pixel(18, 64 + 18)[3]));
        assert_eq!(
            renderer.custom.1.seen,
            vec!["panel", "panel", "custom", "panel", "panel", "custom"]
        );
        assert!(scope.pop().await.is_none(), "wgpu validation error");
        Ok(())
    })
}

/// One description, two targets, one colour.
///
/// A style colour is sRGB-encoded whatever it is painted onto. On a plain
/// target it is written as it stands; on a target that encodes on write it has
/// to be converted first, or it is encoded twice and everything washes out.
/// This pins that the two agree, which is the thing that silently drifts.
#[test]
fn a_style_colour_reads_back_the_same_on_a_plain_and_an_srgb_target() -> Result<()> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
        // A mid grey, where a stray encode is impossible to miss.
        let colour = [0.216, 0.216, 0.216, 1.0];

        let mut read = |format| -> Result<[u8; 4]> {
            let mut renderer = WgpuRenderer::new(&gpu, format)?;
            let texture = renderer.create_texture(8, 8)?;
            let mut ui = Ui::default();
            ui.begin(Input::default());
            ui.style.fill = colour;
            ui.style.stroke_width = 0.0;
            ui.style.radius = 0.0;
            ui.panel(Rect {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            renderer.render(
                &mut WgpuFrame {
                    encoder: &mut encoder,
                    target: TextureTarget::from_texture2d(&texture)?,
                    clear: None,
                },
                ui.end(),
            )?;
            let pixels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 8 * 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                texture.texture.as_ref().unwrap().as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &pixels,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(8),
                    },
                },
                wgpu::Extent3d {
                    width: 8,
                    height: 8,
                    depth_or_array_layers: 1,
                },
            );
            gpu.queue.submit([encoder.finish()]);
            let (sender, receiver) = std::sync::mpsc::channel();
            pixels.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = sender.send(r);
            });
            gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
            receiver.recv()??;
            let mapped = pixels.slice(..).get_mapped_range()?;
            Ok([mapped[0], mapped[1], mapped[2], mapped[3]])
        };

        // Both textures store the same bits: the sRGB one because the pipeline
        // handed it linear and it encoded, the plain one because it was handed
        // the encoded value and stored it.
        let plain = read(wgpu::TextureFormat::Rgba8Unorm)?;
        let srgb = read(wgpu::TextureFormat::Rgba8UnormSrgb)?;
        let expected = (colour[0] * 255.0).round() as u8;
        for channel in 0..3 {
            assert!(
                plain[channel].abs_diff(expected) <= 1,
                "plain target changed the colour: {plain:?} for {expected}"
            );
            assert!(
                srgb[channel].abs_diff(expected) <= 2,
                "sRGB target encoded twice: {srgb:?} for {expected}"
            );
        }
        Ok(())
    })
}

#[test]
fn distance_field_shadow_falls_off_smoothly_with_smoothstep() -> Result<()> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&Default::default()).await?;
        let (device, queue) = adapter.request_device(&Default::default()).await?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
        let mut renderer = WgpuRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm)?;
        let texture = renderer.create_texture(64, 64)?;
        let mut ui = Ui::default();
        ui.begin(Input::default());
        ui.shadow_box(
            Rect {
                x: 16.0,
                y: 16.0,
                width: 32.0,
                height: 32.0,
            },
            4.0,
            16.0,
            [1.0, 1.0, 1.0, 1.0],
        );
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&texture)?,
                clear: Some(wgpu::Color::TRANSPARENT),
            },
            ui.end(),
        )?;
        let pixels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.texture.as_ref().unwrap().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &pixels,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        pixels.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = sender.send(r);
        });
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let mapped = pixels.slice(..).get_mapped_range()?;
        let sample_alpha = |x: usize, y: usize| -> u8 { mapped[y * 256 + x * 4 + 3] };

        // Inside the rectangle: full opacity
        let inside = sample_alpha(32, 32);
        assert!(inside >= 250, "inside alpha should be ~255, got {inside}");

        // Halfway through the blur (x = 48 + 8 = 56, y = 32):
        let mid_blur = sample_alpha(56, 32);
        assert!(
            mid_blur > 50 && mid_blur < 200,
            "mid blur alpha should be smooth ~128, got {mid_blur}"
        );

        // Near / beyond the blur radius:
        let edge_blur = sample_alpha(63, 32);
        assert!(
            edge_blur <= 10,
            "beyond blur should be near zero, got {edge_blur}"
        );

        // Monotonic smooth falloff from edge to outer limit
        let mut prev = sample_alpha(48, 32);
        for x in 49..64 {
            let curr = sample_alpha(x, 32);
            assert!(
                curr as i32 <= prev as i32 + 2,
                "shadow falloff must be monotonic: at x={x}, curr={curr} > prev={prev}"
            );
            prev = curr;
        }

        Ok(())
    })
}
