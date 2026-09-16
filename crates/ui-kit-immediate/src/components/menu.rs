//! Menus: a bar or list of items whose children open as cascading popups,
//! and a [`Select`](super::Select)-style dropdown of choices.
use super::button::{Button, ButtonVariant};
use super::card::{panel, popup_surface};
use crate::icon::Icon;
use crate::{Insets, Key, Layer, Stack, Strip, Ui};
use ui_kit_core::Rect;

/// An entry in a [`Menu`]: a leaf that is chosen, or a parent whose children
/// open beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub children: Vec<MenuItem>,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }
}

/// Which way a [`Menu`]'s top-level items run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuLayout {
    /// Side by side; children open below.
    #[default]
    Horizontal,
    /// One above the other; children open to the right.
    Vertical,
}

/// Which items of a [`Menu`] are open: the index of the open top-level item,
/// then of the open child within it, and so on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuState {
    pub open: Vec<usize>,
}

impl MenuState {
    pub fn is_open(&self) -> bool {
        !self.open.is_empty()
    }

    pub fn close(&mut self) {
        self.open.clear();
    }
}

/// How tall a row is in a popup, the gap between rows, and the popup's padding.
const ROW: f32 = 34.0;
const ROW_GAP: f32 = 4.0;
const PADDING: f32 = 6.0;
/// How far a popup sits from what opened it.
const OFFSET: f32 = 4.0;
const MARGIN: f32 = super::popup::MARGIN;

/// How tall a popup of `rows` rows is.
pub fn popup_height(rows: usize) -> f32 {
    rows as f32 * (ROW + ROW_GAP) - ROW_GAP + PADDING * 2.0
}

use super::popup::{keep_inside, viewport};

/// A menu bar or list whose items open their children in cascading popups.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu<'a> {
    items: &'a [MenuItem],
    layout: MenuLayout,
    widths: Option<&'a [f32]>,
    popup_width: f32,
}

impl<'a> Menu<'a> {
    pub fn horizontal(items: &'a [MenuItem]) -> Self {
        Self {
            items,
            layout: MenuLayout::Horizontal,
            widths: None,
            popup_width: 240.0,
        }
    }

    pub fn vertical(items: &'a [MenuItem]) -> Self {
        Self {
            layout: MenuLayout::Vertical,
            ..Self::horizontal(items)
        }
    }

    /// Fixed widths for a horizontal menu's items; otherwise each fits its label.
    pub fn widths(mut self, widths: &'a [f32]) -> Self {
        self.widths = Some(widths);
        self
    }

    pub fn popup_width(mut self, width: f32) -> Self {
        self.popup_width = width;
        self
    }

    /// How big the bar is: its width for a horizontal menu, its height for a
    /// vertical one.
    pub fn extent(&self, ui: &Ui) -> f32 {
        match self.layout {
            MenuLayout::Horizontal => {
                (0..self.items.len())
                    .map(|i| self.item_width(ui, i))
                    .sum::<f32>()
                    + (self.items.len().saturating_sub(1)) as f32 * 6.0
            }
            MenuLayout::Vertical => popup_height(self.items.len()),
        }
    }

    fn item_width(&self, ui: &Ui, index: usize) -> f32 {
        if let Some(width) = self.widths.and_then(|w| w.get(index)) {
            return *width;
        }
        let item = &self.items[index];
        let label = Button::new(&item.label).variant(ButtonVariant::Text);
        let style = label.style(ui, Rect::default());
        ui.fit_text(&item.label, &style) + if item.children.is_empty() { 0.0 } else { 22.0 }
    }

    /// Draw the menu in `rect` and its open popups over everything else.
    /// Returns the id of the leaf chosen this frame.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, state: &mut MenuState) -> Option<String> {
        let p = ui.theme.palette;
        panel(ui, rect, p.surface, p.border, 8.0);
        let mut chosen = None;
        let mut hit_areas = vec![rect];
        // The top-level items, and where each one's children open.
        let mut anchors = Vec::with_capacity(self.items.len());
        let vertical = self.layout == MenuLayout::Vertical;
        let mut row = Strip::row(rect, 6.0);
        let mut column = Stack::new(rect.inset_by(Insets::symmetric(0.0, PADDING)), ROW_GAP);
        for (i, item) in self.items.iter().enumerate() {
            let slot = if vertical {
                column.row(38.0)
            } else {
                row.leading(self.item_width(ui, i))
            };
            let open = state.open.first() == Some(&i);
            let mut button = Button::new(&item.label)
                .variant(if open {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Text
                })
                .align(if vertical {
                    ui_kit_core::TextAlign::Left
                } else {
                    ui_kit_core::TextAlign::Center
                });
            if !item.children.is_empty() {
                button = button.trailing(if vertical {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                });
            }
            let response = button.show(ui, &format!("{id}-{i}"), slot);
            if response.clicked {
                if item.children.is_empty() {
                    chosen = Some(item.id.clone());
                    state.close();
                } else if open {
                    state.close();
                } else {
                    state.open = vec![i];
                }
            } else if response.hovered && state.is_open() && !open && !item.children.is_empty() {
                // Moving along an open menu bar follows the pointer.
                state.open = vec![i];
            }
            anchors.push(if vertical {
                Rect::new(rect.right() + OFFSET, slot.y - PADDING, 0.0, 0.0)
            } else {
                Rect::new(slot.x, slot.bottom() + OFFSET, 0.0, 0.0)
            });
        }

        // Each open level is a popup; its open child cascades beside it.
        let bounds = viewport(ui);
        let mut level: &[MenuItem] = self.items;
        let mut anchor = None;
        let mut depth = 0;
        while depth < state.open.len() {
            let index = state.open[depth];
            let Some(parent) = level.get(index) else {
                state.open.truncate(depth);
                break;
            };
            if parent.children.is_empty() {
                state.open.truncate(depth);
                break;
            }
            let Some(at) = anchor.or_else(|| (depth == 0).then(|| anchors[index])) else {
                state.open.truncate(depth);
                break;
            };
            let size = (self.popup_width, popup_height(parent.children.len()));
            let mut popup = keep_inside(Rect::new(at.x, at.y, size.0, size.1), bounds);
            if depth > 0 && popup.x < at.x {
                // No room to the right: open to the left of the parent popup.
                popup.x = (at.x - OFFSET * 2.0 - self.popup_width * 2.0).max(bounds.x + MARGIN);
            }
            hit_areas.push(popup);
            let path = state.open[..=depth]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let mut next_anchor = None;
            ui.scoped(|ui| {
                ui.clip = None;
                ui.with_layer(Layer(Layer::OVERLAY.0 + depth as i32), |ui| {
                    ui.reserve(ui.layer(), popup);
                    popup_surface(ui, popup, 10.0);
                    let mut rows = Stack::new(popup.inset(PADDING), ROW_GAP);
                    for (j, child) in parent.children.iter().enumerate() {
                        let slot = rows.row(ROW);
                        let open = state.open.get(depth + 1) == Some(&j);
                        let mut button = Button::new(&child.label)
                            .variant(if open {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Text
                            })
                            .align(ui_kit_core::TextAlign::Left);
                        if !child.children.is_empty() {
                            button = button.trailing(Icon::ChevronRight);
                        }
                        let response = button.show(ui, &format!("{id}-{path}-{j}"), slot);
                        let opens = !child.children.is_empty();
                        let deeper_open = state.open.len() > depth + 1;
                        if response.clicked || (opens && response.hovered && deeper_open) {
                            if opens {
                                state.open.truncate(depth + 1);
                                state.open.push(j);
                            } else if response.clicked {
                                chosen = Some(child.id.clone());
                            }
                        }
                        // Where this row's children open, if it is (now) the open one.
                        if state.open.get(depth + 1) == Some(&j) {
                            next_anchor = Some(Rect::new(
                                popup.right() + OFFSET,
                                slot.y - PADDING,
                                0.0,
                                0.0,
                            ));
                        }
                    }
                });
            });
            if chosen.is_some() {
                state.close();
                break;
            }
            level = &parent.children;
            anchor = next_anchor;
            depth += 1;
        }

        // Pressing anywhere else, or Escape, closes the menu.
        let outside = ui.pressed()
            && ui
                .input()
                .pointer
                .is_some_and(|pt| !hit_areas.iter().any(|area| area.contains(pt)));
        if state.is_open() && (outside || ui.consume_key(Key::Escape)) {
            state.close();
        }
        chosen
    }
}

/// A button showing the chosen option that opens a list of all of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dropdown<'a> {
    options: &'a [&'a str],
    disabled: bool,
}

impl<'a> Dropdown<'a> {
    pub fn new(options: &'a [&'a str]) -> Self {
        Self {
            options,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draw the dropdown; `changed` means `selected` was changed.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, selected: &mut usize) -> crate::Response {
        let label = self.options.get(*selected).copied().unwrap_or("");
        let mut response = super::Select::new(label)
            .disabled(self.disabled)
            .show(ui, id, rect);
        let popup_id = format!("{id}-options");
        if response.clicked {
            ui.toggle_popup(&popup_id);
        }
        if !ui.is_popup_open(&popup_id) {
            return response;
        }
        let popup = keep_inside(
            Rect::new(
                rect.x,
                rect.bottom() + OFFSET,
                rect.width,
                popup_height(self.options.len()),
            ),
            viewport(ui),
        );
        ui.scoped(|ui| {
            ui.clip = None;
            ui.with_layer(Layer::OVERLAY, |ui| {
                ui.reserve(Layer::OVERLAY, popup);
                popup_surface(ui, popup, 10.0);
                let mut rows = Stack::new(popup.inset(PADDING), ROW_GAP);
                for (i, option) in self.options.iter().enumerate() {
                    let variant = if i == *selected {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Text
                    };
                    let clicked = Button::new(option)
                        .variant(variant)
                        .show(ui, &format!("{popup_id}-{i}"), rows.row(ROW))
                        .clicked;
                    if clicked {
                        response.changed = i != *selected;
                        *selected = i;
                        ui.close_popup(&popup_id);
                    }
                }
                // Claims the popup's id, which is what keeps it open next frame.
                ui.interact(&popup_id, popup);
            });
        });
        let outside = ui.pressed()
            && ui
                .input()
                .pointer
                .is_some_and(|pt| !popup.contains(pt) && !rect.contains(pt));
        if outside || ui.consume_key(Key::Escape) {
            ui.close_popup(&popup_id);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::Point;

    fn frame(
        ui: &mut Ui,
        items: &[MenuItem],
        state: &mut MenuState,
        at: Point,
        down: bool,
    ) -> Option<String> {
        ui.begin(Input {
            pointer: Some(at),
            down,
            viewport: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
            ..Default::default()
        });
        let chosen =
            Menu::horizontal(items).show(ui, "menu", Rect::new(0.0, 0.0, 300.0, 38.0), state);
        let _ = ui.end();
        chosen
    }

    fn click(ui: &mut Ui, items: &[MenuItem], state: &mut MenuState, at: Point) -> Option<String> {
        frame(ui, items, state, at, false);
        frame(ui, items, state, at, true);
        frame(ui, items, state, at, false)
    }

    #[test]
    fn children_cascade_open_and_a_leaf_closes_everything() {
        let items = vec![MenuItem::new("a", "A").with_children(vec![
            MenuItem::new("b", "B").with_children(vec![MenuItem::new("c", "C")])
        ])];
        let mut ui = Ui::default();
        let mut state = MenuState::default();
        click(&mut ui, &items, &mut state, Point::new(20.0, 19.0));
        assert_eq!(state.open, vec![0]);
        // B is the first row of the popup under A.
        click(&mut ui, &items, &mut state, Point::new(40.0, 65.0));
        assert_eq!(state.open, vec![0, 0]);
        // C opens beside B's popup, which stays open.
        let c = ui
            .commands
            .iter()
            .find(|c| c.text == "C")
            .map(|c| c.rect)
            .unwrap();
        assert!(ui.commands.iter().any(|c| c.text == "B"));
        let chosen = click(
            &mut ui,
            &items,
            &mut state,
            Point::new(c.x + 10.0, c.y + 10.0),
        );
        assert_eq!(chosen.as_deref(), Some("c"));
        assert!(!state.is_open());
    }

    #[test]
    fn pressing_outside_closes_the_menu() {
        let items = vec![MenuItem::new("a", "A").with_children(vec![MenuItem::new("b", "B")])];
        let mut ui = Ui::default();
        let mut state = MenuState::default();
        click(&mut ui, &items, &mut state, Point::new(20.0, 19.0));
        assert!(state.is_open());
        click(&mut ui, &items, &mut state, Point::new(700.0, 500.0));
        assert!(!state.is_open());
    }
}
