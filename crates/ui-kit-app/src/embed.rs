//! Drawing into a window someone else made and runs: a plug-in's editor in a
//! DAW, a view inside another toolkit's window.
//!
//! There is no event loop here. The host makes the surface on its window,
//! collects the input itself, and calls [`Embedded::frame`] when it draws;
//! the surface, damage tracking and presentation are the runners' own.

use crate::damage::Region;
use crate::gpu::Gpu;
use crate::present::Presenter;
use crate::scale::scale_commands;
use anyhow::Result;
use ui_kit_core::Color;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::{CommandRenderer, WgpuRenderer};

/// A window's surface and the renderer that draws on it.
pub struct Embedded<C> {
    presenter: Presenter<C>,
}

impl<C: CommandRenderer> Embedded<C> {
    /// Draw on `surface`, `width` by `height` physical pixels, with the
    /// device `gpu` holds. `make_custom` makes the renderer's own shaders
    /// for the surface's format, as [`run_with_custom`](crate::run_with_custom)
    /// takes them.
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        make_custom: impl FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C,
    ) -> Result<Self> {
        let presenter = Presenter::new(gpu, surface, width, height, false, None, make_custom)?;
        Ok(Self { presenter })
    }

    /// The renderer, for loading fonts and measuring text.
    pub fn renderer(&self) -> &WgpuRenderer<C> {
        &self.presenter.renderer
    }

    /// Put `commands`, in logical points, on the surface: scaled by `scale`
    /// to its `width` by `height` physical pixels, over `background`.
    /// Whether a frame was presented: none is when nothing changed or there
    /// is nothing to draw on right now.
    pub fn frame(
        &mut self,
        mut commands: Vec<DrawCommand>,
        scale: f32,
        width: u32,
        height: u32,
        background: Color,
    ) -> Result<bool> {
        scale_commands(&mut commands, scale);
        let region = self.presenter.region(&commands, width, height);
        if region == Region::Nothing {
            return Ok(false);
        }
        let Some(output) = self.presenter.acquire(width, height)? else {
            return Ok(false);
        };
        self.presenter.present(&output, commands, background, region)?;
        self.presenter.show(output);
        Ok(true)
    }
}
