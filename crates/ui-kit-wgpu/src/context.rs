use std::sync::Arc;
/// Borrow an application's existing device and queue. No instance or surface ownership.
#[derive(Clone)]
pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}
impl GpuContext {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self { device, queue }
    }
}
#[derive(Clone)]
pub struct Texture2d {
    pub texture: Option<Arc<wgpu::Texture>>,
    pub view: Option<Arc<wgpu::TextureView>>,
    pub size: (u32, u32),
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}
pub(crate) struct TextState {
    pub font_system: glyphon::FontSystem,
    pub swash_cache: glyphon::SwashCache,
    pub cache: glyphon::Cache,
    pub atlas: glyphon::TextAtlas,
}
impl TextState {
    /// `linear` says whether the target wants linear colour, and it has to be
    /// the same answer the shapes are given: glyphon's accurate mode converts
    /// the text colour out of sRGB exactly as `target_color` does, and its web
    /// mode passes it through exactly as the shapes do untouched. A float target
    /// is linear without being sRGB, which is why this is not `format.is_srgb()`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        linear: bool,
    ) -> Self {
        let cache = glyphon::Cache::new(device);
        let atlas = glyphon::TextAtlas::with_color_mode(
            device,
            queue,
            &cache,
            format,
            if linear {
                glyphon::ColorMode::Accurate
            } else {
                glyphon::ColorMode::Web
            },
        );
        Self {
            font_system: glyphon::FontSystem::new(),
            swash_cache: glyphon::SwashCache::new(),
            cache,
            atlas,
        }
    }
}
