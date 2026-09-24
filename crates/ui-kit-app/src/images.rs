use std::sync::Arc;
use ui_kit_immediate::ImageId;
use ui_kit_wgpu::{CommandRenderer, GpuContext, Texture2d, WgpuRenderer};

/// The pictures the renderer can draw for [`Ui::image`](ui_kit_immediate::Ui::image).
///
/// The runner owns the renderer, so an application reaches it through this:
/// in [`App::prepare`](crate::App::prepare), before each drawn frame, it
/// uploads what it will show and forgets what it no longer does. A texture
/// the application renders itself -- a 3D view, a video frame -- goes on
/// [`gpu`](Self::gpu)'s device, in [`image_format`](Self::image_format), and
/// is registered with [`register_texture`](Self::register_texture).
pub trait ImageStore {
    /// Upload 8-bit sRGB-encoded RGBA pixels, `width * height * 4` bytes in
    /// rows, as `id`, replacing whatever `id` showed.
    fn upload_rgba(&mut self, id: ImageId, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<()>;

    /// Show `texture` wherever `id` is drawn. Its view format should suit the
    /// target: an sRGB texture for an sRGB target.
    fn register(&mut self, id: ImageId, texture: Texture2d);

    /// Show a texture the application rendered, in
    /// [`image_format`](Self::image_format) with `TEXTURE_BINDING` usage,
    /// wherever `id` is drawn.
    fn register_texture(&mut self, id: ImageId, texture: Arc<wgpu::Texture>, view: Arc<wgpu::TextureView>);

    /// Stop showing `id`, freeing its texture unless something else holds it.
    fn remove(&mut self, id: ImageId);

    /// The device and queue the renderer draws with, for textures the
    /// application renders itself.
    fn gpu(&self) -> &GpuContext;

    /// The format a texture the application renders should have to look
    /// right: sRGB when the renderer blends in linear light.
    fn image_format(&self) -> wgpu::TextureFormat;
}

impl<C: CommandRenderer> ImageStore for WgpuRenderer<C> {
    fn upload_rgba(&mut self, id: ImageId, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<()> {
        self.upload_image(id, width, height, rgba)
    }

    fn register(&mut self, id: ImageId, texture: Texture2d) {
        self.register_image(id, texture);
    }

    fn register_texture(&mut self, id: ImageId, texture: Arc<wgpu::Texture>, view: Arc<wgpu::TextureView>) {
        WgpuRenderer::register_texture(self, id, texture, view);
    }

    fn remove(&mut self, id: ImageId) {
        self.remove_image(id);
    }

    fn gpu(&self) -> &GpuContext {
        self.gpu_context()
    }

    fn image_format(&self) -> wgpu::TextureFormat {
        WgpuRenderer::image_format(self)
    }
}
