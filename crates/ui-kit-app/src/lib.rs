//! UI Kit application runner with automatic windowing, GPU presentation,
//! transparent backing, and input routing.
//!
//! The same [`App`] runs natively, in a winit window, and in a browser, on a
//! canvas: [`run_app`] and [`run_with_custom`] pick the host for the target.

pub mod app;
mod blit;
mod damage;
pub mod config;
pub mod context;
pub mod fonts;
pub mod gpu;
pub mod images;
mod present;
#[cfg(not(target_arch = "wasm32"))]
pub mod runner;
pub mod scale;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;
pub use ui_kit_immediate::task;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use app::{App, ClosureApp, WindowCommand};
pub use config::AppConfig;
pub use context::FrameContext;
pub use fonts::Fonts;
pub use gpu::Gpu;
pub use images::ImageStore;
#[cfg(not(target_arch = "wasm32"))]
pub use runner::AppRunner;
pub use scale::{resolve_transforms, scale_commands};
pub use task::{spawn, spawn_detached, Task};

use anyhow::Result;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::CommandRenderer;
#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::EventLoop;

/// Run a closure as a full application with default configuration.
pub fn run<F>(title: impl Into<String>, frame: F) -> Result<()>
where
    F: FnMut(&FrameContext) -> Vec<DrawCommand> + 'static,
{
    let config = AppConfig::new(title);
    run_app(ClosureApp::new(frame), config)
}

/// Run an [`App`] instance with a specific [`AppConfig`].
///
/// In a browser this returns once the application is started: the page's
/// event loop runs it from then on.
pub fn run_app<A: App + 'static>(app: A, config: AppConfig) -> Result<()> {
    run_with_custom(app, config, |_, _| ui_kit_wgpu::NoCustom)
}

/// Run an [`App`] with custom GPU shader command rendering.
pub fn run_with_custom<A, C, F>(app: A, config: AppConfig, make_custom: F) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    #[cfg(target_arch = "wasm32")]
    return web::run(app, config, make_custom);
    #[cfg(not(target_arch = "wasm32"))]
    return run_with_custom_on(EventLoop::new()?, app, config, make_custom);
}

/// [`run_app`] on an event loop the caller built, such as
/// [`android::event_loop`].
#[cfg(not(target_arch = "wasm32"))]
pub fn run_app_on<A: App + 'static>(event_loop: EventLoop<()>, app: A, config: AppConfig) -> Result<()> {
    run_with_custom_on(event_loop, app, config, |_, _| ui_kit_wgpu::NoCustom)
}

/// [`run_with_custom`] on an event loop the caller built.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_with_custom_on<A, C, F>(
    event_loop: EventLoop<()>,
    app: A,
    config: AppConfig,
    make_custom: F,
) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    // Tokio's timers and I/O need a runtime to be current where the tasks
    // are polled: the UI thread. Its workers drive them and wake the tasks.
    #[cfg(feature = "tokio")]
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    #[cfg(feature = "tokio")]
    let _entered = runtime.enter();

    // A task woken from any thread brings the event loop round to poll it.
    let proxy = std::sync::Mutex::new(event_loop.create_proxy());
    task::set_wake_hook(move || {
        let _ = proxy.lock().unwrap().send_event(());
    });

    let mut runner = AppRunner::with_custom(app, config, make_custom);
    event_loop.run_app(&mut runner)?;
    match runner.error() {
        Some(error) => Err(anyhow::anyhow!("{error}")),
        None => Ok(()),
    }
}

#[cfg(target_os = "android")]
pub mod android;

/// Where `after` differs from `before` on a `size` target, and the commands
/// that draw it there over `before`, the part cleared to `background` first.
/// `None` when nothing or everything has to be drawn. For tests of drawing
/// in part.
#[doc(hidden)]
pub fn damage_region(
    before: &[DrawCommand],
    after: &[DrawCommand],
    size: (u32, u32),
    background: ui_kit_core::Color,
) -> Option<(ui_kit_core::Rect, Vec<DrawCommand>)> {
    let mut damage = damage::Damage::default();
    damage.shown(before.to_vec());
    match damage.region(after, size, |_| false) {
        damage::Region::Part(part) => {
            let screen = ui_kit_core::Rect::new(0.0, 0.0, size.0 as f32, size.1 as f32);
            Some((part, damage::clip_to(after, part, screen, background)))
        }
        _ => None,
    }
}
