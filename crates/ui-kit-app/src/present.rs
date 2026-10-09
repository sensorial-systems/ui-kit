//! Putting a frame's draw commands on screen: the surface, the renderer and
//! the texture the UI is drawn into. Shared by every runner.

use crate::blit::Blit;
use crate::damage::{clip_to, Damage, Region};
use crate::gpu::Gpu;
use crate::images::TrackedImages;
use anyhow::{anyhow, bail, Result};
use std::collections::HashSet;
use ui_kit_core::{Color, Material, Rect};
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::{DrawCommand, Image, ImageId, IMAGE_MATERIAL};
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
    /// What the target shows, to draw only what changes.
    damage: Damage,
    /// How round the window's corners are cut out of what is presented,
    /// physical pixels: 0 leaves them square.
    corner_radius: f32,
    /// The pictures the application renders into itself.
    live_images: HashSet<ImageId>,
    /// Whether a picture changed since the last frame drawn.
    images_changed: bool,
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
            damage: Damage::default(),
            corner_radius: 0.0,
            live_images: HashSet::new(),
            images_changed: false,
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

    /// Cut the window's corners round, `radius` physical pixels, out of what
    /// is presented: for a window that shows through where nothing is drawn.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn set_corner_radius(&mut self, radius: f32) {
        self.corner_radius = radius.max(0.0);
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

    /// The pictures the renderer draws, for [`App::prepare`](crate::App::prepare).
    pub fn images(&mut self) -> TrackedImages<'_, C> {
        TrackedImages {
            renderer: &mut self.renderer,
            live: &mut self.live_images,
            changed: &mut self.images_changed,
        }
    }

    /// Where `commands`, in physical pixels, have to be drawn on a `width`
    /// by `height` surface for it to show them. [`Region::Nothing`] when it
    /// already does: there is no frame to present.
    pub fn region(&self, commands: &[DrawCommand], width: u32, height: u32) -> Region {
        let target = self.target.as_ref().map(|t| t.size);
        if target != Some((width, height)) || self.images_changed {
            return Region::Everywhere;
        }
        let live = |command: &DrawCommand| match &command.style.material {
            Material::Custom { name, parameters } if name == IMAGE_MATERIAL => {
                Image::decode(parameters).is_some_and(|(id, _)| self.live_images.contains(&id))
            }
            _ => self.renderer.custom.wants(command) && !self.renderer.custom.is_static(command),
        };
        self.damage.region(commands, (width, height), live)
    }

    /// Draw `commands`, already in physical pixels, onto `output` where
    /// [`region`](Self::region) found they changed; opaque windows are
    /// cleared to `background` first. [`Presenter::show`] then puts it on
    /// screen.
    pub fn present(
        &mut self,
        output: &wgpu::SurfaceTexture,
        commands: Vec<DrawCommand>,
        background: Color,
        region: Region,
    ) -> Result<()> {
        let config = self.config.as_ref().ok_or_else(|| anyhow!("no surface"))?;
        let (width, height) = (output.texture.width(), output.texture.height());
        let mut region = region;
        if self.target.as_ref().is_none_or(|t| t.size != (width, height)) {
            self.target = Some(self.renderer.create_texture(width, height)?);
            region = Region::Everywhere;
        }
        self.images_changed = false;
        // Until this frame is drawn, what the target shows is not known.
        self.damage.invalidate();
        let target = self.target.as_ref().unwrap();
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let frame_target = TextureTarget::from_texture2d(target)?;
        let part = match region {
            Region::Part(part) => Some(part),
            _ => None,
        };
        let frame = if part.is_some() {
            // The part is cleared by the first command drawn in it.
            WgpuFrame::new(&mut encoder, frame_target)
        } else if self.transparent {
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
        match part {
            Some(part) => {
                let screen = Rect::new(0.0, 0.0, width as f32, height as f32);
                // The part is cleared to what the whole would be.
                let clear = if self.transparent { [0.0; 4] } else { background };
                let clipped = clip_to(&commands, part, screen, clear);
                self.renderer.render(&mut { frame }, &clipped)?;
            }
            None => self.renderer.render(&mut { frame }, &commands)?,
        }
        let rounded = self.corner_radius > 0.0;
        if config.usage.contains(wgpu::TextureUsages::COPY_DST) && !rounded {
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
                &self.gpu.queue,
                &mut encoder,
                target.view.as_ref().unwrap(),
                &surface_view,
                (width, height, self.corner_radius),
            );

        }
        self.gpu.queue.submit([encoder.finish()]);
        self.damage.shown(commands);
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
    // Presenting waits for the display's refresh: a host that draws again
    // as soon as a frame is shown runs at the display's rate, not faster.
    config.present_mode = wgpu::PresentMode::AutoVsync;
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
