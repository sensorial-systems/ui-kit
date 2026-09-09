use anyhow::Result;
use immediate_gallery::geometry::GeometryRenderer;
use immediate_gallery::{scale_commands, Gallery};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use ui_kit_core::Point;
use ui_kit_immediate::immediate::TextNavigation;
use ui_kit_immediate::{composition::UiRenderer, Input};
use ui_kit_wgpu::{GpuContext, TextureTarget, WgpuFrame, WgpuRenderer};
use ui_kit_window::{WindowStyle, WinitWindow};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    gpu: Option<GpuContext>,
    renderer: Option<WgpuRenderer<GeometryRenderer>>,
    target: Option<ui_kit_wgpu::Texture2d>,
    gallery: Gallery,
    chrome: WinitWindow,
    input: Input,
    modifiers: ModifiersState,
    ime: bool,
    start: Instant,
    next: Instant,
    error: Option<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            config: None,
            gpu: None,
            renderer: None,
            target: None,
            gallery: Gallery::default(),
            chrome: WinitWindow::new("UI Kit Component Gallery — Immediate / wgpu"),
            input: Input::default(),
            modifiers: ModifiersState::default(),
            ime: false,
            start: Instant::now(),
            next: Instant::now(),
            error: None,
        }
    }
}
impl App {
    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(event_loop.create_window(self.chrome.attributes(
            Window::default_attributes().with_inner_size(LogicalSize::new(1200.0, 850.0)),
        ))?);
        window.set_ime_allowed(true);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| anyhow::anyhow!("surface has no supported format"))?;
        if let Some(format) = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| !f.is_srgb())
        {
            config.format = format;
        }
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        // Glass reads the previous target. Render to our sampleable texture, then copy to surface.
        let caps = surface.get_capabilities(&adapter);
        anyhow::ensure!(
            caps.usages.contains(wgpu::TextureUsages::COPY_DST),
            "surface does not support presentation copies"
        );
        config.usage |= wgpu::TextureUsages::COPY_DST;
        surface.configure(&gpu.device, &config);
        self.renderer = Some(WgpuRenderer::with_custom(
            &gpu,
            config.format,
            GeometryRenderer::new(&gpu, config.format),
        )?);
        self.config = Some(config);
        self.surface = Some(surface);
        self.gpu = Some(gpu);
        // Explicitly show after initialization, including launches from hidden shell hosts.
        window.set_visible(true);
        self.window = Some(window);
        Ok(())
    }
    fn draw(&mut self) -> Result<()> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let gpu = self.gpu.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let config = self.config.as_mut().unwrap();
        if config.width != size.width || config.height != size.height {
            config.width = size.width;
            config.height = size.height;
            surface.configure(&gpu.device, config);
        }
        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(v)
            | wgpu::CurrentSurfaceTexture::Suboptimal(v) => v,
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&gpu.device, config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("presentation surface lost; restart the gallery")
            }
            wgpu::CurrentSurfaceTexture::Validation => anyhow::bail!("surface validation error"),
        };
        let scale = window.scale_factor() as f32;
        let mut style = WindowStyle::default();
        let background = self.gallery.background();
        style.frame.fill = [
            background.r as f32,
            background.g as f32,
            background.b as f32,
            background.a as f32,
        ];
        style.bar.fill = style.frame.fill;
        style.title.foreground = if self.gallery.dark {
            [0.96, 0.96, 0.96, 1.0]
        } else {
            [0.04, 0.04, 0.04, 1.0]
        };
        style.control.foreground = style.title.foreground;
        style.close.foreground = style.title.foreground;
        let mut commands = self.chrome.frame(
            window,
            self.input.clone(),
            &style,
            |input, width, height| {
                self.gallery
                    .frame(input, width, height, self.start.elapsed().as_secs_f32())
            },
        );
        self.input.text.clear();
        self.input.backspace = false;
        self.input.tab = false;
        self.input.activate = false;
        self.input.step = 0;
        self.input.select_all = false;
        self.input.navigation = None;
        self.input.delete = false;
        self.input.undo = false;
        self.input.redo = false;
        self.input.save = false;
        scale_commands(&mut commands, scale);
        let renderer = self.renderer.as_mut().unwrap();
        if self
            .target
            .as_ref()
            .is_none_or(|t| t.size.0 != size.width || t.size.1 != size.height)
        {
            self.target = Some(renderer.create_texture(size.width, size.height)?);
        }
        let target = self.target.as_ref().unwrap();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&target)?,
                clear: Some(self.gallery.background()),
            },
            &commands,
        )?;
        encoder.copy_texture_to_texture(
            target.texture.as_ref().unwrap().as_image_copy(),
            output.texture.as_image_copy(),
            output.texture.size(),
        );
        gpu.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        gpu.queue.present(output);
        Ok(())
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.initialize(event_loop) {
                self.error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(window) = &self.window {
            match self.chrome.event(window, &event) {
                Ok(response) => {
                    if response.close_requested {
                        event_loop.exit();
                    }
                    if response.consumed {
                        return;
                    }
                }
                Err(error) => {
                    eprintln!("Window operation unavailable: {error}");
                    return;
                }
            }
        }
        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor()) as f32;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.pointer = Some(Point::new(
                    position.x as f32 / scale,
                    position.y as f32 / scale,
                ));
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if !self.input.down {
                    self.input.pointer = None;
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.input.down = state == ElementState::Pressed;
                if let Err(e) = self.draw() {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.gallery.scroll_by(match delta {
                MouseScrollDelta::LineDelta(_, y) => -y * 48.0,
                MouseScrollDelta::PixelDelta(p) => -p.y as f32 / scale,
            }),
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
                self.gallery.shift = self.modifiers.shift_key();
                self.input.extend_selection = self.modifiers.shift_key();
            }
            WindowEvent::Focused(false) => {
                self.input.down = false;
                self.input.pointer = None;
                self.gallery.cancel_drag();
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => self.ime = !text.is_empty(),
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.input.text.push_str(&text);
                self.ime = false;
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.input.extend_selection = self.modifiers.shift_key();
                match &event.logical_key {
                    Key::Character(s)
                        if (self.modifiers.control_key() || self.modifiers.super_key())
                            && s.eq_ignore_ascii_case("z") =>
                    {
                        if self.modifiers.shift_key() {
                            self.input.redo = true
                        } else {
                            self.input.undo = true
                        }
                    }
                    Key::Character(s)
                        if (self.modifiers.control_key() || self.modifiers.super_key())
                            && s.eq_ignore_ascii_case("y") =>
                    {
                        self.input.redo = true
                    }
                    Key::Named(NamedKey::Enter)
                        if self.modifiers.control_key() || self.modifiers.super_key() =>
                    {
                        self.input.save = true
                    }
                    Key::Named(NamedKey::Delete) => self.input.delete = true,
                    Key::Character(s)
                        if (self.modifiers.control_key() || self.modifiers.super_key())
                            && s.eq_ignore_ascii_case("a") =>
                    {
                        self.input.select_all = true
                    }
                    Key::Named(NamedKey::Escape) => self.gallery.dismiss(),
                    Key::Named(NamedKey::Tab) => self.input.tab = true,
                    Key::Named(NamedKey::Enter) => self.input.activate = true,
                    Key::Named(NamedKey::Backspace) => self.input.backspace = true,
                    Key::Named(NamedKey::ArrowRight) => {
                        self.input.step = 1;
                        self.input.navigation = Some(TextNavigation::Right)
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.input.step = 1;
                        self.input.navigation = Some(TextNavigation::Up)
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.input.step = -1;
                        self.input.navigation = Some(TextNavigation::Left)
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.input.step = -1;
                        self.input.navigation = Some(TextNavigation::Down)
                    }
                    Key::Named(NamedKey::PageDown) => self.gallery.scroll_by(650.0),
                    Key::Named(NamedKey::PageUp) => self.gallery.scroll_by(-650.0),
                    Key::Named(NamedKey::Home) if self.modifiers.control_key() => {
                        self.gallery.scroll_by(-100000.0)
                    }
                    Key::Named(NamedKey::End) if self.modifiers.control_key() => {
                        self.gallery.scroll_by(100000.0)
                    }
                    Key::Named(NamedKey::Home) => {
                        self.input.navigation = Some(TextNavigation::Home)
                    }
                    Key::Named(NamedKey::End) => self.input.navigation = Some(TextNavigation::End),
                    _ => {
                        if !self.ime && !self.modifiers.control_key() && !self.modifiers.super_key()
                        {
                            if let Some(text) = event.text {
                                self.input.text.push_str(&text);
                            }
                        }
                    }
                }
                if let Err(e) = self.draw() {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            self.next = now + Duration::from_micros(16_667);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next));
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
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        anyhow::bail!(error)
    }
    Ok(())
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
