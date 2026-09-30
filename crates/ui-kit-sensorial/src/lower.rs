//! Every component in glass.
//!
//! Components draw their surfaces as ordinary commands: a button is a
//! `button` command with its label on it, a field a `text_input`, a card a
//! `panel` in [`Material::Glass`]. [`lower`] turns a frame of them into
//! what the look draws: each surface becomes a pane of glass
//! ([`GLASS_PANEL`]) with the rest of the command -- its label, its value,
//! its focus ring -- drawn over it, and buttons become pills.
//!
//! A surface is glass when its material is the look's [`GLASS`], or the
//! kit's own `Material::Glass`, or when it is filled in the palette's
//! [`surface`](ui_kit_immediate::Palette::surface) or
//! [`raised`](ui_kit_immediate::Palette::raised) colour: the colours the
//! components fill a surface with. A surface disabled (drawn fainter),
//! filled in the accent, square (a table's stripe) or smaller than a
//! fingertip stays as it was.
//!
//! [`GLASS`]: crate::GLASS

use ui_kit_core::{Color, Material};
use ui_kit_immediate::{DrawCommand, Layer, Palette};

use crate::glass::{GlassLook, AT_POINTER, GLASS_PANEL};

/// The kinds that draw a surface under what they show.
const SURFACES: [&str; 6] = [
    "panel",
    "button",
    "text_input",
    "checkbox",
    "slider",
    "progress",
];

/// Smaller than this each way, a surface is too small to bend light.
const SMALLEST: f32 = 12.0;

fn same(a: Color, b: Color) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
}

/// How much the pointer is on a command: 1 pressed, most of the way
/// hovered.
fn hover(command: &DrawCommand) -> f32 {
    if command.active {
        1.0
    } else if command.hovered {
        0.7
    } else {
        0.0
    }
}

/// `look`, lit further as the pointer comes onto it.
fn lit(mut look: GlassLook, hover: f32) -> GlassLook {
    if hover > 0.0 {
        look.tint_amount += 0.05 * hover;
        look.rim += 0.18 * hover;
        look.spectral += 0.3 * hover;
        look.refraction += 5.0 * hover;
        look.highlight_strength += 0.08 * hover;
        look.brightness += 0.05 * hover;
        look.highlight = AT_POINTER;
    }
    look
}

/// The glass `command`'s surface is made of, if it is.
fn glass_of(command: &DrawCommand, palette: &Palette) -> Option<GlassLook> {
    let style = &command.style;
    let rect = command.rect;
    let over = command.layer >= Layer::MODAL;
    let fitting = |hover: f32| {
        if over {
            GlassLook::popup()
        } else if rect.height <= 64.0 {
            GlassLook::control(hover)
        } else {
            GlassLook::card(hover)
        }
    };
    let look = match &style.material {
        Material::Custom { .. } => GlassLook::of(&style.material)?,
        Material::Glass { blur } => GlassLook {
            blur: blur.max(8.0),
            ..fitting(0.0)
        },
        Material::Solid => {
            let surface = same(style.fill, palette.surface) || same(style.fill, palette.raised);
            let rounded = style.radius >= 2.0 || style.corner_radii.is_some();
            if !surface || !rounded || !SURFACES.contains(&command.kind.as_ref()) {
                return None;
            }
            fitting(0.0)
        }
    };
    if rect.width.min(rect.height) < SMALLEST {
        return None;
    }
    Some(lit(look, hover(command)))
}

/// Round a button into a pill, unless it asks for corners of its own.
fn pill(command: &mut DrawCommand) {
    if command.kind == "button" && command.style.corner_radii.is_none() {
        command.style.radius = command.style.radius.max(command.rect.height * 0.5);
    }
}

/// A frame of commands, as the look draws it: surfaces in glass, with what
/// is on them drawn over the glass, and buttons as pills. Commands the look
/// already draws -- the backdrop, bare glass, meshes -- pass through.
pub fn lower(commands: Vec<DrawCommand>, palette: &Palette) -> Vec<DrawCommand> {
    let mut lowered = Vec::with_capacity(commands.len() + commands.len() / 4);
    for mut command in commands {
        if command.kind == GLASS_PANEL {
            lowered.push(command);
            continue;
        }
        pill(&mut command);
        let Some(look) = glass_of(&command, palette) else {
            if GlassLook::of(&command.style.material).is_some() {
                // Glass too small to draw as glass: a faint surface.
                command.style.material = Material::Solid;
                command.style.fill = palette.surface;
            }
            lowered.push(command);
            continue;
        };
        let mut pane = command.clone();
        pane.kind = GLASS_PANEL.into();
        pane.text.clear();
        pane.rich_text = None;
        pane.value = 0.0;
        pane.style.material = look.material();
        pane.style.corner_radii = None;
        lowered.push(pane);
        // What is on the surface, over the glass: the glass is its fill
        // and its rim its border. A bare panel has nothing more to draw.
        if command.kind == "panel" && command.text.is_empty() && !command.focused {
            continue;
        }
        command.style.material = Material::Solid;
        command.style.fill = [0.0; 4];
        command.style.gradient_end = None;
        command.style.stroke = [0.0; 4];
        lowered.push(command);
    }
    lowered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{palette, theme};
    use ui_kit_core::Rect;
    use ui_kit_immediate::components::{Button, ButtonVariant, Card, TextInput};
    use ui_kit_immediate::{Input, Ui};

    fn frame(draw: impl FnOnce(&mut Ui)) -> Vec<DrawCommand> {
        let mut ui = Ui::default();
        ui.theme = theme();
        ui.begin(Input::default());
        draw(&mut ui);
        ui.end().to_vec()
    }

    fn kinds(commands: &[DrawCommand]) -> Vec<&str> {
        commands.iter().map(|c| c.kind.as_ref()).collect()
    }

    #[test]
    fn a_button_is_a_pill_of_glass_with_its_label_over_it() {
        let commands = frame(|ui| {
            Button::new("Save").variant(ButtonVariant::Secondary).show(
                ui,
                "save",
                Rect::new(0.0, 0.0, 120.0, 40.0),
            );
        });
        let lowered = lower(commands, &palette());
        assert_eq!(kinds(&lowered), [GLASS_PANEL, "button"]);
        assert!(GlassLook::of(&lowered[0].style.material).is_some());
        assert_eq!(lowered[0].style.radius, 20.0);
        assert_eq!(lowered[1].text, "Save");
        assert_eq!(lowered[1].style.fill[3], 0.0);
    }

    #[test]
    fn the_primary_button_stays_solid_white() {
        let commands = frame(|ui| {
            Button::new("Go").show(ui, "go", Rect::new(0.0, 0.0, 120.0, 40.0));
        });
        let lowered = lower(commands, &palette());
        assert_eq!(kinds(&lowered), ["button"]);
        assert_eq!(lowered[0].style.fill, palette().accent);
        assert_eq!(lowered[0].style.radius, 20.0);
    }

    #[test]
    fn a_card_and_a_field_are_glass() {
        let commands = frame(|ui| {
            Card::new().show(ui, Rect::new(0.0, 0.0, 300.0, 200.0));
            let mut text = String::from("hello");
            TextInput::new("Name").show(ui, "name", Rect::new(10.0, 10.0, 200.0, 36.0), &mut text);
        });
        let lowered = lower(commands, &palette());
        let panes = lowered.iter().filter(|c| c.kind == GLASS_PANEL).count();
        assert_eq!(panes, 2);
        assert!(lowered.iter().any(|c| c.kind == "text_input"));
    }

    #[test]
    fn a_disabled_surface_stays_as_it_was() {
        let commands = frame(|ui| {
            Button::new("No")
                .variant(ButtonVariant::Secondary)
                .disabled(true)
                .show(ui, "no", Rect::new(0.0, 0.0, 120.0, 40.0));
        });
        let lowered = lower(commands, &palette());
        assert_eq!(kinds(&lowered), ["button"]);
    }
}
