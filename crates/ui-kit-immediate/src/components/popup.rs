//! Placing and drawing things that float over the page.
use super::card::popup_surface;
use crate::{Key, Layer, Ui};
use ui_kit_core::Rect;

/// How far a popup keeps from the viewport's edges.
pub const MARGIN: f32 = 12.0;

/// The area popups have to stay inside: the input's viewport, or unbounded.
pub fn viewport(ui: &Ui) -> Rect {
    ui.input()
        .viewport
        .unwrap_or(Rect::new(0.0, 0.0, f32::MAX / 4.0, f32::MAX / 4.0))
}

/// `rect` moved to stay inside `viewport`, keeping its size.
pub fn keep_inside(rect: Rect, viewport: Rect) -> Rect {
    let area = viewport.inset(MARGIN);
    Rect {
        x: rect.x.min(area.right() - rect.width).max(area.x),
        y: rect.y.min(area.bottom() - rect.height).max(area.y),
        ..rect
    }
}

/// Where a popup `width` × `height` opens under `control`, `gap` below it,
/// moved to stay inside the viewport.
pub fn below(ui: &Ui, control: Rect, gap: f32, width: f32, height: f32) -> Rect {
    keep_inside(
        Rect::new(control.x, control.bottom() + gap, width, height),
        viewport(ui),
    )
}

/// Draw `body` as a popup in `rect`: over everything on `layer`, unclipped,
/// on a surface, taking the pointer from what is under it.
pub fn show<R>(ui: &mut Ui, rect: Rect, layer: Layer, body: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scoped(|ui| {
        ui.clip = None;
        ui.with_layer(layer, |ui| {
            ui.reserve(layer, rect);
            popup_surface(ui, rect, 12.0);
            body(ui)
        })
    })
}

/// Whether the pointer was pressed outside all of `areas`, or Escape was
/// pressed: what dismisses a popup.
pub fn dismissed(ui: &mut Ui, areas: &[Rect]) -> bool {
    let outside = ui.pressed()
        && ui
            .input()
            .pointer
            .is_some_and(|pt| !areas.iter().any(|area| area.contains(pt)));
    outside || ui.consume_key(Key::Escape)
}
