//! Sensorial's look, for any UI Kit application: black, as the brand's
//! intro is, lit by light split as through a prism, with everything on it
//! made of glass that bends that light at its rim and splits it into
//! colours.
//!
//! - [`theme`] is the [`Theme`](ui_kit_immediate::Theme) every component
//!   draws with: set it as `ui.theme`, and [`lower`] each frame's commands,
//!   and buttons become pills of glass, fields and cards panes of it,
//!   popups dark frosted glass, and the primary action the one solid white
//!   thing on the screen.
//! - [`paint_backdrop`] paints the living world behind it all, as an
//!   [`Atmosphere`] says: rays, drifting pools of light, dust that streaks
//!   with speed, a haze and a flash.
//! - [`GlassLook`] is glass of any kind, with the looks the brand uses
//!   ([`GlassLook::control`], [`card`](GlassLook::card),
//!   [`pane`](GlassLook::pane), [`bar`](GlassLook::bar), ...).
//! - [`Type`] and [`scale`] set type; [`FONT`] is the face.
//! - [`Springs`] and the [`motion`] feels are how things move;
//!   [`Pill`], [`Link`], [`GlassCard`] and [`eyebrow`] are the look's own
//!   controls, moving by them.
//! - [`SensorialRenderer`] (feature `wgpu`, on by default) draws the world
//!   and the glass.
//!
//! ```ignore
//! let atmosphere = ui_kit_sensorial::shared(Atmosphere::new(0.55));
//! // Renderer: the look, then the vector meshes it draws lines with.
//! let renderers = (SensorialRenderer::new(&gpu, atmosphere.clone()),
//!                  VectorRenderer::new(&gpu, format));
//! // Each frame:
//! atmosphere.borrow_mut().advance(dt, [width, height], scale, input.pointer);
//! ui.theme = ui_kit_sensorial::theme();
//! ui.begin(input);
//! ui_kit_sensorial::paint_backdrop(&mut ui, screen);
//! // ... any components ...
//! let commands = ui_kit_sensorial::lower(ui.end().to_vec(), &ui.theme.palette);
//! ```

mod atmosphere;
mod color;
mod controls;
mod glass;
mod lower;
pub mod motion;
mod paint;
#[cfg(feature = "wgpu")]
mod render;
mod typography;

pub use atmosphere::{paint_backdrop, shared, Atmosphere, SharedAtmosphere, BACKDROP};
pub use color::{faded, palette, spectrum, theme, white, BLACK, INK, PAPER, WHITE};
pub use controls::{eyebrow, grow, hover, pointer_in, GlassCard, Glyph, Link, Pill, Shown};
pub use glass::{GlassLook, AT_POINTER, GLASS, GLASS_PANEL};
pub use lower::lower;
pub use motion::{Feel, Spring, Springs};
pub use paint::{arrow_down, arrow_out, arrow_right, fill, shadow, speaker, spectral_line};
#[cfg(feature = "wgpu")]
pub use render::{Light, LightLayer, SensorialRenderer};
pub use typography::{paragraph, scale, text, text_width, wrap, Type, FAMILIES, FONT};
