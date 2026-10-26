use crate::app::{App, WindowCommand};
use crate::config::AppConfig;
use crate::context::{FrameContext, SafeArea, TitleBar};
use crate::damage::Region;
use crate::gpu;
use crate::pipeline_cache::PipelineCache;
use crate::present::Presenter;
use crate::scale::scale_commands;
use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use ui_kit_core::Rect;
use ui_kit_wgpu::{CommandRenderer, GpuContext, NoCustom};
use ui_kit_window::{InputCollector, WindowAction, WindowControl, WindowStyle, WinitWindow};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, MouseScrollDelta, WindowEvent},
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
    presenter: Option<Presenter<C>>,
    /// The pipelines compiled so far, kept for the next launch (Android).
    pipeline_cache: Option<PipelineCache>,
    chrome: WinitWindow,
    collector: InputCollector,
    start: Instant,
    next: Instant,
    error: Option<String>,
    /// Whether the window is on screen. A hidden window is not drawn, but
    /// the application is still updated at the frame cadence.
    visible: bool,
    /// What the frames did, printed each second when `UI_KIT_FRAME_STATS` is set.
    stats: Option<FrameStats>,
    /// When the last frame was made that changed nothing. Presenting waits
    /// for the display, but a frame that is not presented does not: until a
    /// refresh has passed, another is put off rather than made, so a mouse
    /// reporting a thousand times a second makes no more frames than the
    /// display shows.
    unchanged_at: Option<Instant>,
    /// A frame put off until then.
    redraw_at: Option<Instant>,
}

/// How tall the window's buttons are on a bar the application draws, at
/// most: centred on a taller bar.
const CONTROL_HEIGHT: f32 = 40.0;

/// How long the display shows a frame.

fn refresh_interval(window: &Window) -> Duration {
    let millihertz = window
        .current_monitor()
        .and_then(|monitor| monitor.refresh_rate_millihertz())
        .unwrap_or(60_000)
        .max(1_000);
    Duration::from_micros(1_000_000_000 / millihertz as u64)
}

/// How many frames were made in a second, and how many of them drawn.
#[derive(Default)]
struct FrameStats {
    since: Option<Instant>,
    built: u32,
    whole: u32,
    parts: u32,
    /// The share of the window the parts drawn covered, summed.
    part_area: f32,
    /// Time spent making and drawing frames, in seconds.
    busy: f64,
}

impl FrameStats {
    fn frame(&mut self, region: Region, size: (u32, u32), busy: Duration) {
        self.built += 1;
        self.busy += busy.as_secs_f64();
        match region {
            Region::Nothing => {}
            Region::Everywhere => self.whole += 1,
            Region::Part(rect) => {
                self.parts += 1;
                self.part_area += rect.width * rect.height / (size.0 as f32 * size.1 as f32);
            }
        }
        let since = *self.since.get_or_insert_with(Instant::now);
        if since.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "{} frames made: {} drawn whole, {} in part ({:.1}% of the window each), {} unchanged; {:.2} ms to make each",
                self.built,
                self.whole,
                self.parts,
                self.part_area / self.parts.max(1) as f32 * 100.0,
                self.built - self.whole - self.parts,
                self.busy * 1000.0 / self.built.max(1) as f64,
            );
            *self = Self::default();
        }
    }
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
        if let Some(height) = config.custom_title_bar {
            chrome.chrome = chrome.chrome.clone().custom_bar(height);
            chrome.chrome.control_height = Some(height.min(CONTROL_HEIGHT));
        }
        if config.fullscreen_button {
            chrome.chrome.controls.insert(0, WindowControl::Fullscreen);
        }

        Self {
            app,
            config,
            custom_renderer: Some(Box::new(make_custom)),
            window: None,
            presenter: None,
            pipeline_cache: None,
            chrome,
            collector: InputCollector::new(),
            start: Instant::now(),
            next: Instant::now(),
            error: None,
            visible,
            stats: std::env::var_os("UI_KIT_FRAME_STATS").map(|_| FrameStats::default()),
            unchanged_at: None,
            redraw_at: None,
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

    /// The window, once the event loop has made it: for a host running
    /// several, to tell their events apart, and to place it.
    pub fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }

    /// Draw the window, or stop drawing it, without showing or hiding it --
    /// for a host that shows it itself: without taking the keyboard from
    /// the window that has it, say.
    pub fn set_drawn(&mut self, drawn: bool) {
        self.visible = drawn;
        if let (Some(window), true) = (&self.window, drawn) {
            window.request_redraw();
        }
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
                WindowCommand::Redraw => {
                    if let (Some(window), true) = (&self.window, self.visible) {
                        window.request_redraw();
                    }
                }
                WindowCommand::ToggleFullscreen => {
                    if let Some(window) = &self.window {
                        self.chrome.action(window, WindowAction::ToggleFullscreen);
                        window.request_redraw();
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
        // iOS shows (and keeps up) the on-screen keyboard whenever IME is
        // allowed; apps with no text entry there must leave it off. Desktop
        // and Android keep IME for text input.
        window.set_ime_allowed(!cfg!(target_os = "ios"));
        #[cfg(not(target_os = "android"))]
        if self.config.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }

        let instance = pollster::block_on(gpu::instance());
        let surface = instance.create_surface(window.clone())?;
        let gpu = pollster::block_on(gpu::request(instance, &surface, &self.app))?;
        self.app.gpu_ready(&gpu);
        self.pipeline_cache = PipelineCache::open(&gpu);

        let make_custom = self
            .custom_renderer
            .take()
            .ok_or_else(|| anyhow::anyhow!("renderer already initialized"))?;
        let size = window.inner_size();
        let presenter = Presenter::new(
            &gpu,
            surface,
            size.width,
            size.height,
            self.config.transparent,
            self.pipeline_cache.as_ref().map(|c| c.cache().clone()),
            make_custom,
        )?;
        self.app.fonts_ready(&presenter.renderer);
        self.app.text_measure_ready(presenter.renderer.text_measure());
        self.presenter = Some(presenter);
        // The window is created hidden either way, so a tray application
        // never flashes a frame it is about to hide.
        window.set_visible(self.visible);
        self.window = Some(window);
        Ok(())
    }

    /// A new surface for the existing window, after the platform took the
    /// last one away (Android, on suspend).
    fn recreate_surface(&mut self) -> Result<()> {
        let (Some(window), Some(presenter)) = (&self.window, self.presenter.as_mut()) else {
            return Ok(());
        };
        let surface = presenter.gpu().instance.create_surface(window.clone())?;
        let size = window.inner_size();
        presenter.set_surface(surface, size.width, size.height)?;
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
        let Some(presenter) = self.presenter.as_mut() else {
            return Ok(());
        };
        let started = Instant::now();
        if let Some(unchanged) = self.unchanged_at {
            let due = unchanged + refresh_interval(window);
            if started < due {
                self.redraw_at = Some(due);
                return Ok(());
            }
        }
        self.unchanged_at = None;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let scale = window.scale_factor() as f32;
        // The device's safe-area insets (logical points): off the notch, the
        // home indicator and the rounded corners. iOS only; zero elsewhere.
        #[cfg(target_os = "ios")]
        let safe_area = {
            use winit::platform::ios::WindowExtIOS;
            let (top, right, bottom, left) = window.safe_area_insets();
            SafeArea { top: top as f32, right: right as f32, bottom: bottom as f32, left: left as f32 }
        };
        #[cfg(not(target_os = "ios"))]
        let safe_area = SafeArea::default();
        let mut style = WindowStyle::default();
        let bg = self.config.background;
        style.frame.fill = bg;
        style.bar.fill = bg;
        // A faint edge in the window's own light, not the default style's blue.
        style.frame.stroke = if self.config.dark {
            [1.0, 1.0, 1.0, 0.12]
        } else {
            [0.0, 0.0, 0.0, 0.14]
        };
        style.title.foreground = if self.config.dark {
            [0.96, 0.96, 0.96, 1.0]
        } else {
            [0.04, 0.04, 0.04, 1.0]
        };
        style.control.foreground = style.title.foreground;
        style.close.foreground = style.title.foreground;

        // Pictures first, so the frame can draw what was just uploaded.
        self.app.prepare(&mut presenter.images());

        let elapsed = self.start.elapsed().as_secs_f32();
        let dark = self.config.dark;
        let bounds = self.chrome.bounds(window);
        let custom_bar = self.config.chrome && self.chrome.chrome.custom_bar;
        // Where the application's bar is, and the buttons on it.
        self.chrome.sync(window);
        let title_bar = custom_bar.then(|| {
            let chrome = &self.chrome.chrome;
            let rect = chrome.title_bar(bounds);
            let width = chrome.controls_width();
            TitleBar {
                rect,
                buttons: Rect::new(rect.x + rect.width - width, rect.y, width, rect.height),
                maximized: chrome.maximized,
                fullscreen: chrome.fullscreen,
            }
        });
        // The UI steps through the input in the order it came -- each
        // stretch between button changes a step of its own, so a click is
        // seen where it was made, however slow the frame -- and the last
        // step is the one drawn.
        let mut commands = Vec::new();
        for input in self.collector.take(bounds) {
            commands = if self.config.chrome {
                self.chrome
                    .frame(window, input, &style, |mut input, width, height| {
                        input.viewport = Some(Rect::new(0.0, 0.0, width, height));
                        let ctx = FrameContext::new(input, width, height, elapsed, scale, dark)
                            .with_title_bar(title_bar)
                            .with_safe_area(safe_area);
                        self.app.frame(&ctx)
                    })
            } else {
                let ctx = FrameContext::new(input, bounds.width, bounds.height, elapsed, scale, dark)
                    .with_safe_area(safe_area);
                self.app.frame(&ctx)
            };
            // The cursor, input method and clipboard follow what the step asked for.
            // On the frame's resize edges (and while one is dragged) its
            // resize cursor stays: set the content's there and the two would
            // take turns each frame.
            if let Some(output) = self.app.output() {
                let shape = match self.config.chrome.then(|| self.chrome.cursor()).flatten() {
                    Some(edge) => edge,
                    None => ui_kit_window::cursor(output.cursor),
                };
                self.collector.apply_with_cursor(window, &output, shape);
            }
        }
        if custom_bar {
            // The application's bar as it laid it out: its height, and where
            // it has controls the bar must not take as a move. Its
            // coordinates are the window's, the content filling it.
            let layout = self.app.title_bar();
            if let Some(height) = layout.height.filter(|h| *h > 0.0) {
                self.chrome.chrome.title_bar_height = height;
                self.chrome.chrome.control_height = Some(height.min(CONTROL_HEIGHT));
            }
            self.chrome.set_bar_controls(bounds, layout.controls);
        }
        // Where the system does not round the window, the corners are cut
        // out of what is presented, on a surface that shows through.
        presenter.set_corner_radius(
            if self.config.chrome && self.config.transparent && !self.chrome.rounded_by_system() {
                self.chrome.chrome.effective_radius() * scale
            } else {
                0.0
            },
        );

        scale_commands(&mut commands, scale);
        // A frame that changes nothing is not drawn: the window keeps
        // showing the last one.
        let region = presenter.region(&commands, size.width, size.height);
        if let Some(stats) = &mut self.stats {
            stats.frame(region, (size.width, size.height), started.elapsed());
        }
        if region == Region::Nothing {
            self.unchanged_at = Some(started);
            return Ok(());
        }
        // Suspended, minimized or occluded: there is nothing to draw on.
        let Some(surface_texture) = presenter.acquire(size.width, size.height)? else {
            return Ok(());
        };
        presenter.present(&surface_texture, commands, bg, region)?;
        window.pre_present_notify();
        presenter.show(surface_texture);
        // Something changed, and may be moving: look again at the display's
        // next refresh rather than the next update. Presenting waits for the
        // refresh, so this runs at the display's rate while things move and
        // stops the first frame nothing does.
        window.request_redraw();
        Ok(())
    }
}

impl<A: App, C: CommandRenderer + 'static> ApplicationHandler for AppRunner<A, C> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = if self.window.is_none() {
            self.initialize(event_loop)
        } else if self.presenter.as_ref().is_some_and(|p| !p.has_surface()) {
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
        if let Some(presenter) = self.presenter.as_mut() {
            presenter.drop_surface();
        }
        // The app may be killed in the background without another word.
        if let Some(cache) = self.pipeline_cache.as_mut() {
            cache.save();
        }
        self.app.on_suspend();
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(cache) = self.pipeline_cache.as_mut() {
            cache.save();
        }
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
            WindowEvent::Focused(true) => {
                self.hide_system_bars();
                self.app.on_focus_gained();
            }
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

    /// The mouse moved: the motion a held pointer reports.
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let (DeviceEvent::MouseMotion { delta }, Some(window)) = (event, &self.window) {
            self.collector.motion(window, delta);
        }
    }

    /// A task was woken: move it along now rather than at the next frame.
    fn user_event(&mut self, _: &ActiveEventLoop, (): ()) {
        crate::task::run_ready();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let frame_interval = Duration::from_micros(1_000_000 / self.config.fps as u64);
        if now >= self.next {
            // Updated whether or not there is a window to draw: a tray
            // application hidden from the screen still polls and still
            // answers its tray menu.
            crate::task::run_ready();
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
        let mut wake = self.next;
        if let Some(at) = self.redraw_at {
            if now >= at {
                self.redraw_at = None;
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            } else {
                wake = wake.min(at);
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
    }
}
