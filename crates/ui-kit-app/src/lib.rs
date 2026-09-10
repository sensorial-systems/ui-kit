//! Native UI Kit application runner with automatic windowing, GPU presentation,
//! transparent backing, and input routing.

pub mod app;
pub mod config;
pub mod context;
pub mod runner;
pub mod scale;

pub use app::{App, ClosureApp};
pub use config::AppConfig;
pub use context::FrameContext;
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
    let event_loop = EventLoop::new()?;
    let mut runner = AppRunner::new(app, config);
    event_loop.run_app(&mut runner)?;
    Ok(())
}

/// Run an [`App`] with custom GPU shader command rendering.
pub fn run_with_custom<A, C, F>(app: A, config: AppConfig, make_custom: F) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    let event_loop = EventLoop::new()?;
    let mut runner = AppRunner::with_custom(app, config, make_custom);
    event_loop.run_app(&mut runner)?;
    Ok(())
}
