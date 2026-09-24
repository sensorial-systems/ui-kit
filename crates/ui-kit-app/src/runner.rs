use crate::app::{App, WindowCommand};
use crate::config::AppConfig;
use crate::context::FrameContext;
use crate::scale::scale_commands;
use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use ui_kit_core::Rect;
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_wgpu::{CommandRenderer, GpuContext, NoCustom, TextureTarget, WgpuFrame, WgpuRenderer};
use ui_kit_window::{InputCollector, WindowStyle, WinitWindow};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowId},
};

/// The main application runner managing the winit event loop, GPU presentation, and input routing.
///
/// On Android the system takes the native window away whenever the activity
/// goes to the background: the runner drops its surface on
/// [`suspended`](ApplicationHandler::suspended) and makes a new one on the
/// next [`resumed`](ApplicationHandler::resumed), keeping the device, the
/// renderer and the application.
pub struct AppRunner<A, C = NoCustom> {
    pub app: A,
    pub config: AppConfig,
    custom_renderer: Option<Box<dyn FnOnce(&GpuContext, wgpu::TextureFormat) -> C>>,
    window: Option<Arc<Window>>,
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    gpu: Option<GpuContext>,
    renderer: Option<WgpuRenderer<C>>,
    target: Option<ui_kit_wgpu::Texture2d>,
    chrome: WinitWindow,
    collector: InputCollector,
    start: Instant,
    next: Instant,
    error: Option<String>,
    /// Whether the window is on screen. A hidden window is not drawn, but
    /// the application is still updated at the frame cadence.
    visible: bool,
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
        let visible = config.visible;
        let mut chrome = WinitWindow::new(title);
        chrome.chrome.transparent = transparent;
        chrome.chrome.resizable = resizable;

        Self {
            app,
            config,
            custom_renderer: Some(Box::new(make_custom)),
            window: None,
            instance: None,
            adapter: None,
            surface: None,
            surface_config: None,
            gpu: None,
            renderer: None,
            target: None,
            chrome,
            collector: InputCollector::new(),
            start: Instant::now(),
            next: Instant::now(),
            error: None,
            visible,
        }
    }

    fn hide_system_bars(&self) {
        #[cfg(target_os = "android")]
        if self.config.fullscreen {
            crate::android::hide_system_bars();
        }
    }

    /// What stopped the event loop, if it stopped on an error.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Do what the application asked of its window since the last update.
    fn apply_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        for command in self.app.window_commands() {
            match command {
                WindowCommand::Show => {
                    self.visible = true;
                    if let Some(window) = &self.window {
                        window.set_visible(true);
                        window.set_minimized(false);
                        window.focus_window();
                        window.request_redraw();
                    }
                }
                WindowCommand::Hide => {
                    self.visible = false;
                    if let Some(window) = &self.window {
                        window.set_visible(false);
                    }
                }
                WindowCommand::Focus => {
                    if let Some(window) = &self.window {
                        window.focus_window();
                    }
                }
                WindowCommand::Exit => event_loop.exit(),
            }
        }
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let mut attributes = Window::default_attributes()
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .with_visible(false);

        if let (Some(w), Some(h)) = (self.config.min_width, self.config.min_height) {
            attributes = attributes.with_min_inner_size(LogicalSize::new(w, h));
        }
        let attributes = if self.config.chrome {
            self.chrome.attributes(attributes)
        } else {
            attributes
                .with_title(&self.config.title)
                .with_resizable(self.config.resizable)
                .with_transparent(self.config.transparent)
        };

        let window = Arc::new(event_loop.create_window(attributes)?);
        window.set_ime_allowed(true);
        #[cfg(not(target_os = "android"))]
        if self.config.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));

        let surface_config = self.configure(&surface, &adapter, &gpu, &window)?;
        let format = surface_config.format;
        let make_custom = self
            .custom_renderer
            .take()
            .ok_or_else(|| anyhow::anyhow!("renderer already initialized"))?;
        let custom = make_custom(&gpu, format);
        let renderer = WgpuRenderer::with_custom(&gpu, format, custom)?;

        self.app.text_measure_ready(renderer.text_measure());
        self.renderer = Some(renderer);
        self.surface_config = Some(surface_config);
        self.surface = Some(surface);
        self.adapter = Some(adapter);
        self.instance = Some(instance);
        self.gpu = Some(gpu);
        // The window is created hidden either way, so a tray application
        // never flashes a frame it is about to hide.
        window.set_visible(self.visible);
        self.window = Some(window);
        Ok(())
    }

    /// Chooses the surface's format and usage and configures it.
    fn configure(
        &self,
        surface: &wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        gpu: &GpuContext,
        window: &Window,
    ) -> Result<wgpu::SurfaceConfiguration> {
        let size = window.inner_size();
        let mut surface_config = surface
            .get_default_config(adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| anyhow::anyhow!("surface has no supported format"))?;
        let caps = surface.get_capabilities(adapter);
        // A renderer made for one format keeps it: a surface made again on
        // resume takes the format the first one had.
        if let Some(previous) = &self.surface_config {
            surface_config.format = previous.format;
        } else if let Some(format) = caps.formats.iter().copied().find(|f| !f.is_srgb()) {
            surface_config.format = format;
        }
        surface_config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST;
        if self.config.transparent {
            surface_config.alpha_mode = ui_kit_wgpu::transparent_alpha_mode(&caps);
        }
        surface.configure(&gpu.device, &surface_config);
        Ok(surface_config)
    }

    /// A new surface for the existing window, after the platform took the
    /// last one away (Android, on suspend).
    fn recreate_surface(&mut self) -> Result<()> {
        let (Some(window), Some(instance), Some(adapter), Some(gpu)) =
            (&self.window, &self.instance, &self.adapter, &self.gpu)
        else {
            return Ok(());
        };
        let surface = instance.create_surface(window.clone())?;
        let surface_config = self.configure(&surface, adapter, gpu, window)?;
        self.surface_config = Some(surface_config);
        self.surface = Some(surface);
        window.request_redraw();
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        if !self.visible {
            return Ok(());
        }
        let (Some(surface), Some(surface_config)) = (self.surface.as_ref(), self.surface_config.as_mut())
        else {
            // Suspended: there is nothing to draw on.
            return Ok(());
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let gpu = self.gpu.as_ref().unwrap();

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

        // Pictures first, so the frame can draw what was just uploaded.
        if let Some(renderer) = self.renderer.as_mut() {
            self.app.prepare(renderer);
        }

        let elapsed = self.start.elapsed().as_secs_f32();
        let dark = self.config.dark;
        let bounds = self.chrome.bounds(window);
        let input = self.collector.take(bounds);
        let mut commands = if self.config.chrome {
            self.chrome
                .frame(window, input, &style, |mut input, width, height| {
                    input.viewport = Some(Rect::new(0.0, 0.0, width, height));
                    let ctx = FrameContext::new(input, width, height, elapsed, scale, dark);
                    self.app.frame(&ctx)
                })
        } else {
            let ctx = FrameContext::new(input, bounds.width, bounds.height, elapsed, scale, dark);
            self.app.frame(&ctx)
        };
        // The cursor, input method and clipboard follow what the frame asked for.
        if let Some(output) = self.app.output() {
            self.collector.apply(window, &output);
        }

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
        let result = if self.window.is_none() {
            self.initialize(event_loop)
        } else if self.surface.is_none() {
            self.recreate_surface()
        } else {
            Ok(())
        };
        if let Err(e) = result {
            self.error = Some(e.to_string());
            event_loop.exit();
            return;
        }
        self.hide_system_bars();
        self.app.on_resume();
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        // The native window behind the surface is going away (Android).
        self.surface = None;
        self.app.on_suspend();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.app.should_exit() {
            event_loop.exit();
            return;
        }

        if let (Some(window), true) = (&self.window, self.config.chrome) {
            match self.chrome.event(window, &event) {
                Ok(response) => {
                    if response.close_requested && self.app.on_close_requested() {
                        event_loop.exit();
                    }
                    if response.consumed {
                        self.apply_window_commands(event_loop);
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
        if let Some(window) = &self.window {
            if self.collector.event(window, &event) {
                window.request_redraw();
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.app.on_close_requested() {
                    event_loop.exit();
                }
                self.apply_window_commands(event_loop);
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    self.error = Some(e.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(false) => self.app.on_focus_lost(),
            // Android shows the system bars again after a dialog or the
            // notification shade took focus.
            WindowEvent::Focused(true) => self.hide_system_bars(),
            WindowEvent::MouseWheel { delta, .. } => {
                let vertical = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 48.0,
                    MouseScrollDelta::PixelDelta(pos) => -pos.y as f32 / scale,
                };
                self.app.on_scroll(vertical);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let frame_interval = Duration::from_micros(1_000_000 / self.config.fps as u64);
        if now >= self.next {
            // Updated whether or not there is a window to draw: a tray
            // application hidden from the screen still polls and still
            // answers its tray menu.
            self.app.update();
            self.apply_window_commands(event_loop);
            if self.app.should_exit() {
                event_loop.exit();
                return;
            }
            if self.visible {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            self.next = now + frame_interval;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next));
    }
}
