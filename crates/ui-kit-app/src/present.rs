//! Putting a frame's draw commands on screen: the surface, the renderer and
//! the texture the UI is drawn into. Shared by every runner.

use crate::blit::Blit;
use crate::gpu::Gpu;
use anyhow::{anyhow, bail, Result};
use ui_kit_core::Color;
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::DrawCommand;
use ui_kit_wgpu::{CommandRenderer, Texture2d, TextureTarget, WgpuFrame, WgpuRenderer};

pub(crate) struct Presenter<C> {
    gpu: Gpu,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    pub renderer: WgpuRenderer<C>,
    /// What the UI is drawn into before it is put on the surface, so that
    /// glass can read back what is behind it.
    target: Option<Texture2d>,
    /// How it is put on a surface that cannot be copied into (a browser's
    /// canvas), made the first time one is met.
    blit: Option<Blit>,
    transparent: bool,
}

// Surfaces come and go only natively (Android's suspend and resume), so a
// browser build leaves some of this unused.
impl<C: CommandRenderer> Presenter<C> {
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        transparent: bool,
        make_custom: impl FnOnce(&ui_kit_wgpu::GpuContext, wgpu::TextureFormat) -> C,
    ) -> Result<Self> {
        let config = configure(gpu, &surface, width, height, transparent, None)?;
        let context = gpu.context();
        let custom = make_custom(&context, config.format);
        let renderer = WgpuRenderer::with_custom(&context, config.format, custom)?;
        Ok(Self {
            gpu: gpu.clone(),
            surface: Some(surface),
            config: Some(config),
            renderer,
            target: None,
            blit: None,
            transparent,
        })
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    /// A new surface for the same window, after the platform took the last
    /// one away (Android, on suspend). It keeps the format the renderer was
    /// made for.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn set_surface(&mut self, surface: wgpu::Surface<'static>, width: u32, height: u32) -> Result<()> {
        let previous = self.config.as_ref().map(|c| c.format);
        let config = configure(&self.gpu, &surface, width, height, self.transparent, previous)?;
        self.config = Some(config);
        self.surface = Some(surface);
        Ok(())
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// Forget the surface: the window behind it is going away.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn drop_surface(&mut self) {
        self.surface = None;
    }

    /// The texture this frame is drawn to, resized to `width` by `height`
    /// physical pixels. `None` when there is nothing to draw on right now
    /// (suspended, zero-sized, occluded, or the surface had to be
    /// reconfigured): skip the frame.
    pub fn acquire(&mut self, width: u32, height: u32) -> Result<Option<wgpu::SurfaceTexture>> {
        let (Some(surface), Some(config)) = (self.surface.as_ref(), self.config.as_mut()) else {
            return Ok(None);
        };
        if width == 0 || height == 0 {
            return Ok(None);
        }
        if config.width != width || config.height != height {
            config.width = width;
            config.height = height;
            surface.configure(&self.gpu.device, config);
        }
        Ok(match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(v) | wgpu::CurrentSurfaceTexture::Suboptimal(v) => Some(v),
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&self.gpu.device, config);
                None
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => None,
            wgpu::CurrentSurfaceTexture::Lost => bail!("presentation surface lost"),
            wgpu::CurrentSurfaceTexture::Validation => bail!("surface validation error"),
        })
    }

    /// Draw `commands`, already in physical pixels, onto `output`; opaque
    /// windows are cleared to `background` first. [`Presenter::show`] then
    /// puts it on screen.
    pub fn present(
        &mut self,
        output: &wgpu::SurfaceTexture,
        commands: &[DrawCommand],
        background: Color,
    ) -> Result<()> {
        let config = self.config.as_ref().ok_or_else(|| anyhow!("no surface"))?;
        let (width, height) = (output.texture.width(), output.texture.height());
        if self.target.as_ref().is_none_or(|t| t.size != (width, height)) {
            self.target = Some(self.renderer.create_texture(width, height)?);
        }
        let target = self.target.as_ref().unwrap();
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let frame_target = TextureTarget::from_texture2d(target)?;
        let frame = if self.transparent {
            WgpuFrame::transparent(&mut encoder, frame_target)
        } else {
            WgpuFrame {
                encoder: &mut encoder,
                target: frame_target,
                clear: Some(wgpu::Color {
                    r: background[0] as f64,
                    g: background[1] as f64,
                    b: background[2] as f64,
                    a: background[3] as f64,
                }),
            }
        };
        self.renderer.render(&mut { frame }, commands)?;
        if config.usage.contains(wgpu::TextureUsages::COPY_DST) {
            encoder.copy_texture_to_texture(
                target.texture.as_ref().unwrap().as_image_copy(),
                output.texture.as_image_copy(),
                output.texture.size(),
            );
        } else {
            let blit = self
                .blit
                .get_or_insert_with(|| Blit::new(&self.gpu.device, config.format));
            let surface_view = output.texture.create_view(&Default::default());
            blit.draw(
                &self.gpu.device,
                &mut encoder,
                target.view.as_ref().unwrap(),
                &surface_view,
            );
        }
        self.gpu.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Show a texture acquired from this presenter's surface.
    pub fn show(&self, output: wgpu::SurfaceTexture) {
        self.gpu.queue.present(output);
    }
}

/// Chooses the surface's format and usage and configures it.
fn configure(
    gpu: &Gpu,
    surface: &wgpu::Surface<'static>,
    width: u32,
    height: u32,
    transparent: bool,
    previous: Option<wgpu::TextureFormat>,
) -> Result<wgpu::SurfaceConfiguration> {
    let mut config = surface
        .get_default_config(&gpu.adapter, width.max(1), height.max(1))
        .ok_or_else(|| anyhow!("surface has no supported format"))?;
    let caps = surface.get_capabilities(&gpu.adapter);
    // A renderer made for one format keeps it: a surface made again on
    // resume takes the format the first one had.
    if let Some(format) = previous {
        config.format = format;
    } else if let Some(format) = caps.formats.iter().copied().find(|f| !f.is_srgb()) {
        config.format = format;
    }
    config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    if caps.usages.contains(wgpu::TextureUsages::COPY_DST) {
        config.usage |= wgpu::TextureUsages::COPY_DST;
    }
    if transparent {
        config.alpha_mode = ui_kit_wgpu::transparent_alpha_mode(&caps);
    }
    surface.configure(&gpu.device, &config);
    Ok(config)
}
