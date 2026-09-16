//! UI rendering on the application's wgpu device, into caller-owned textures.
//! Layered on top of `prism-ui-render`.

pub use prism_ui_render::{
    alpha_mode::transparent_alpha_mode,
    command_renderer::{CommandRenderer, NoCustom},
    frame::WgpuFrame,
    gpu_context::GpuContext,
    renderer::WgpuUiRenderer,
    rich_text::RichTextLayout,
    target::TextureTarget,
    text_measure::TextMeasurer,
    Texture2d, WgpuRenderer,
};
pub use vector::VectorRenderer;

mod vector;

use ui_kit_core::RichText;

/// Places the caret in `ui_kit_immediate::components::EditableText` by
/// shaping the text the way the renderer draws it.
#[derive(Default)]
pub struct ShapedRichText(pub RichTextLayout);

impl ui_kit_immediate::components::RichTextMetrics for ShapedRichText {
    fn hit(&mut self, doc: &RichText, width: f32, height: f32, size: f32, x: f32, y: f32) -> usize {
        self.0.hit(doc, width, height, size, x, y)
    }

    fn vertical(
        &mut self,
        doc: &RichText,
        width: f32,
        height: f32,
        size: f32,
        position: usize,
        down: bool,
    ) -> usize {
        self.0.vertical(doc, width, height, size, position, down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_alpha_mode_prefers_premultiplied() {
        let caps = wgpu::SurfaceCapabilities {
            formats: vec![wgpu::TextureFormat::Bgra8Unorm],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![
                wgpu::CompositeAlphaMode::Opaque,
                wgpu::CompositeAlphaMode::PreMultiplied,
            ],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format_capabilities: Default::default(),
        };
        assert_eq!(
            transparent_alpha_mode(&caps),
            wgpu::CompositeAlphaMode::PreMultiplied
        );
    }

    #[test]
    fn transparent_alpha_mode_falls_back_when_no_alpha_mode() {
        let caps = wgpu::SurfaceCapabilities {
            formats: vec![wgpu::TextureFormat::Bgra8Unorm],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format_capabilities: Default::default(),
        };
        assert_eq!(
            transparent_alpha_mode(&caps),
            wgpu::CompositeAlphaMode::Opaque
        );
    }
}
