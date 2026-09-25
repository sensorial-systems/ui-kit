use ui_kit_immediate::FontFamily;
use ui_kit_wgpu::{CommandRenderer, WgpuRenderer};

/// The fonts the renderer draws and measures text with.
///
/// Handed to [`App::fonts_ready`](crate::App::fonts_ready) once the renderer
/// exists, before anything is measured: the place to add an application's
/// own fonts and to say which installed faces the generic families mean.
pub trait Fonts {
    /// Add a font, from the bytes of a TrueType or OpenType file.
    fn load_font(&self, bytes: Vec<u8>);

    /// Point `family` at the first of `candidates` that is installed, as a
    /// CSS font stack chooses, and return the one chosen.
    fn prefer_family(&self, family: FontFamily, candidates: &[&str]) -> Option<String>;

    /// Make the family `alias` find the first of `candidates` that is
    /// installed, when no face of that name is; returns the family it finds.
    fn alias_family(&self, alias: &str, candidates: &[&str]) -> Option<String>;
}

impl<C: CommandRenderer> Fonts for WgpuRenderer<C> {
    fn load_font(&self, bytes: Vec<u8>) {
        WgpuRenderer::load_font(self, bytes);
    }

    fn prefer_family(&self, family: FontFamily, candidates: &[&str]) -> Option<String> {
        WgpuRenderer::prefer_family(self, family, candidates)
    }

    fn alias_family(&self, alias: &str, candidates: &[&str]) -> Option<String> {
        WgpuRenderer::alias_family(self, alias, candidates)
    }
}
