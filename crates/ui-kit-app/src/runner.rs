use crate::app::App;
use crate::config::AppConfig;
use crate::context::FrameContext;
use crate::scale::scale_commands;
use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use ui_kit_core::Point;
use ui_kit_immediate::{composition::UiRenderer, immediate::TextNavigation, Input};
use ui_kit_wgpu::{CommandRenderer, GpuContext, NoCustom, TextureTarget, WgpuFrame, WgpuRenderer};
use ui_kit_window::{WindowStyle, WinitWindow};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

/// The main application runner managing the winit event loop, GPU presentation, and input routing.
pub struct AppRunner<A, C = NoCustom> {
    pub app: A,
    pub config: AppConfig,
    custom_renderer: Option<Box<dyn FnOnce(&GpuContext, wgpu::TextureFormat) -> C>>,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    gpu: Option<GpuContext>,
    renderer: Option<WgpuRenderer<C>>,
    target: Option<ui_kit_wgpu::Texture2d>,
    chrome: WinitWindow,
    input: Input,
    modifiers: ModifiersState,
    ime: bool,
    start: Instant,
    next: Instant,
    error: Option<String>,
}

impl<A: App> AppRunner<A, NoCustom> {
    pub fn new(app: A, config: AppConfig) -> Self {
        Self::with_custom(app, config, |_, _| NoCustom)
    }
}

impl<A: App, C: CommandRenderer + 'static> AppRunner<A, C> {
    pub fn with_custom<F>(app: A, config: AppConfig, make_custom: F) -> Self
    where
        F: FnOnce(&GpuContext, wgpu::TextureFormat) -> C + 'static,
    {
        let title = config.title.clone();
        let transparent = config.transparent;
        let resizable = config.resizable;
        let mut chrome = WinitWindow::new(title);
        chrome.chrome.transparent = transparent;
        chrome.chrome.resizable = resizable;

        Self {
            app,
            config,
            custom_renderer: Some(Box::new(make_custom)),
            window: None,
            surface: None,
            surface_config: None,
            gpu: None,
            renderer: None,
            target: None,
            chrome,
            input: Input::default(),
            modifiers: ModifiersState::default(),
            ime: false,
            start: Instant::now(),
            next: Instant::now(),
            error: None,
        }
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let mut attributes = Window::default_attributes()
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));

        if let (Some(w), Some(h)) = (self.config.min_width, self.config.min_height) {
            attributes = attributes.with_min_inner_size(LogicalSize::new(w, h));
        }

        let window = Arc::new(event_loop.create_window(self.chrome.attributes(attributes))?);
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
        let mut surface_config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| anyhow::anyhow!("surface has no supported format"))?;

        if let Some(format) = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| !f.is_srgb())
        {
            surface_config.format = format;
        }
        surface_config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST;

        let caps = surface.get_capabilities(&adapter);
        if self.config.transparent {
            surface_config.alpha_mode = ui_kit_wgpu::transparent_alpha_mode(&caps);
        }

        surface.configure(&gpu.device, &surface_config);

        let format = surface_config.format;
        let make_custom = self
            .custom_renderer
            .take()
            .ok_or_else(|| anyhow::anyhow!("renderer already initialized"))?;
        let custom = make_custom(&gpu, format);
        let renderer = WgpuRenderer::with_custom(&gpu, format, custom)?;

        self.renderer = Some(renderer);
        self.surface_config = Some(surface_config);
        self.surface = Some(surface);
        self.gpu = Some(gpu);
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
        let surface_config = self.surface_config.as_mut().unwrap();

        if surface_config.width != size.width || surface_config.height != size.height {
            surface_config.width = size.width;
            surface_config.height = size.height;
            surface.configure(&gpu.device, surface_config);
        }

        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(v)
            | wgpu::CurrentSurfaceTexture::Suboptimal(v) => v,
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&gpu.device, surface_config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("presentation surface lost")
            }
            wgpu::CurrentSurfaceTexture::Validation => anyhow::bail!("surface validation error"),
        };

        let scale = window.scale_factor() as f32;
        let mut style = WindowStyle::default();
        let bg = self.config.background;
        style.frame.fill = bg;
        style.bar.fill = bg;
        style.title.foreground = if self.config.dark {
            [0.96, 0.96, 0.96, 1.0]
        } else {
            [0.04, 0.04, 0.04, 1.0]
        };
        style.control.foreground = style.title.foreground;
        style.close.foreground = style.title.foreground;

        let elapsed = self.start.elapsed().as_secs_f32();
        let dark = self.config.dark;
        let mut commands = self.chrome.frame(
            window,
            self.input.clone(),
            &style,
            |input, width, height| {
                let ctx = FrameContext::new(input, width, height, elapsed, scale, dark);
                self.app.frame(&ctx)
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

        let frame = if self.config.transparent {
            WgpuFrame::transparent(&mut encoder, TextureTarget::from_texture2d(&target)?)
        } else {
            WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&target)?,
                clear: Some(wgpu::Color {
                    r: bg[0] as f64,
                    g: bg[1] as f64,
                    b: bg[2] as f64,
                    a: bg[3] as f64,
                }),
            }
        };

        renderer.render(&mut { frame }, &commands)?;

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

impl<A: App, C: CommandRenderer + 'static> ApplicationHandler for AppRunner<A, C> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.initialize(event_loop) {
                self.error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.app.should_exit() {
            event_loop.exit();
            return;
        }

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
                self.app.update();
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
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.input.down = state == ElementState::Pressed;
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(false) => {
                self.input.down = false;
                self.input.pointer = None;
                self.app.on_focus_lost();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let vertical = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 48.0,
                    MouseScrollDelta::PixelDelta(pos) => -pos.y as f32 / scale,
                };
                self.app.on_scroll(vertical);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.input.extend_selection = self.modifiers.shift_key();
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => self.ime = true,
                Ime::Disabled => self.ime = false,
                Ime::Commit(text) => {
                    self.input.text.push_str(&text);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                _ => {}
            },
            WindowEvent::KeyboardInput { event: key, .. } => {
                if key.state == ElementState::Pressed {
                    match key.logical_key {
                        Key::Named(NamedKey::Backspace) => self.input.backspace = true,
                        Key::Named(NamedKey::Delete) => self.input.delete = true,
                        Key::Named(NamedKey::Tab) => self.input.tab = true,
                        Key::Named(NamedKey::Enter) => self.input.activate = true,
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.input.step = -1;
                            self.input.navigation = Some(TextNavigation::Left);
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            self.input.step = 1;
                            self.input.navigation = Some(TextNavigation::Right);
                        }
                        Key::Named(NamedKey::ArrowUp) => self.input.step = 1,
                        Key::Named(NamedKey::ArrowDown) => self.input.step = -1,
                        Key::Named(NamedKey::Home) => {
                            self.input.navigation = Some(TextNavigation::Home);
                        }
                        Key::Named(NamedKey::End) => {
                            self.input.navigation = Some(TextNavigation::End);
                        }
                        Key::Character(text) => {
                            let ctrl = self.modifiers.control_key() || self.modifiers.super_key();
                            if ctrl {
                                match text.as_str() {
                                    "a" | "A" => self.input.select_all = true,
                                    "z" | "Z" => {
                                        if self.modifiers.shift_key() {
                                            self.input.redo = true;
                                        } else {
                                            self.input.undo = true;
                                        }
                                    }
                                    "y" | "Y" => self.input.redo = true,
                                    "s" | "S" => self.input.save = true,
                                    _ => {}
                                }
                            } else if !self.ime {
                                self.input.text.push_str(text.as_str());
                            }
                        }
                        _ => {}
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let frame_interval = Duration::from_micros(1_000_000 / self.config.fps as u64);
        if now >= self.next {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            self.next = now + frame_interval;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next));
    }
}
