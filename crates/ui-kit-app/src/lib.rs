//! Native UI Kit application runner with automatic windowing, GPU presentation,
//! transparent backing, and input routing.

pub mod app;
pub mod config;
pub mod context;
pub mod images;
pub mod runner;
pub mod scale;

pub use app::{App, ClosureApp, WindowCommand};
pub use config::AppConfig;
pub use context::FrameContext;
pub use images::ImageStore;
pub use runner::AppRunner;
pub use scale::scale_commands;

use anyhow::Result;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::CommandRenderer;
use winit::event_loop::EventLoop;

/// Run a closure as a full desktop application with default configuration.
pub fn run<F>(title: impl Into<String>, frame: F) -> Result<()>
where
    F: FnMut(&FrameContext) -> Vec<DrawCommand> + 'static,
{
    let config = AppConfig::new(title);
    run_app(ClosureApp::new(frame), config)
}

/// Run an [`App`] instance with a specific [`AppConfig`].
pub fn run_app<A: App + 'static>(app: A, config: AppConfig) -> Result<()> {
    run_app_on(EventLoop::new()?, app, config)
}

/// [`run_app`] on an event loop the caller built, such as
/// [`android::event_loop`].
pub fn run_app_on<A: App + 'static>(event_loop: EventLoop<()>, app: A, config: AppConfig) -> Result<()> {
    run_with_custom_on(event_loop, app, config, |_, _| ui_kit_wgpu::NoCustom)
}

/// Run an [`App`] with custom GPU shader command rendering.
pub fn run_with_custom<A, C, F>(app: A, config: AppConfig, make_custom: F) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    run_with_custom_on(EventLoop::new()?, app, config, make_custom)
}

/// [`run_with_custom`] on an event loop the caller built.
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
    let mut runner = AppRunner::with_custom(app, config, make_custom);
    event_loop.run_app(&mut runner)?;
    match runner.error() {
        Some(error) => Err(anyhow::anyhow!("{error}")),
        None => Ok(()),
    }
}

#[cfg(target_os = "android")]
pub mod android;
