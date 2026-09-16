//! Getting around an application: a sidebar of sections.
use super::card::panel;
use super::feedback::{Badge, StatusVariant};
use super::theme::with_alpha;
use super::Label;
use crate::icon::Icon;
use crate::{CursorIcon, Stack, Ui};
use ui_kit_core::{Color, Rect};

/// One entry of a [`Sidebar`].
#[derive(Clone, Debug, PartialEq)]
pub struct SidebarItem {
    pub label: String,
    pub icon: Option<Icon>,
    /// A short figure shown at the trailing end, as a count of things waiting.
    pub badge: Option<String>,
    pub badge_color: Option<Color>,
}

impl SidebarItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            badge: None,
            badge_color: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn badge_color(mut self, color: Color) -> Self {
        self.badge_color = Some(color);
        self
    }
}

/// A column of sections, one of them chosen, with a highlight that slides
/// between them.
///
/// Narrower than [`Sidebar::COLLAPSE_WIDTH`], it shows only the icons.
#[derive(Clone, Debug, PartialEq)]
pub struct Sidebar<'a> {
    items: &'a [SidebarItem],
    row_height: f32,
    gap: f32,
}

impl<'a> Sidebar<'a> {
    /// Below this width the labels are dropped and the icons centred.
    pub const COLLAPSE_WIDTH: f32 = 120.0;

    pub fn new(items: &'a [SidebarItem]) -> Self {
        Self {
            items,
            row_height: 40.0,
            gap: 4.0,
        }
    }

    pub fn rows(mut self, row_height: f32, gap: f32) -> Self {
        self.row_height = row_height;
        self.gap = gap;
        self
    }

    /// How tall the items are together.
    pub fn height(&self) -> f32 {
        let n = self.items.len() as f32;
        n * self.row_height + (n - 1.0).max(0.0) * self.gap
    }

    /// Draw the items down from the top of `rect`. Returns whether `selected`
    /// changed.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, selected: &mut usize) -> bool {
        let p = ui.theme.palette;
        let collapsed = rect.width < Self::COLLAPSE_WIDTH;
        let mut rows = Stack::new(rect, self.gap);
        let mut changed = false;
        let mut chosen_row: Option<Rect> = None;
        let mut rects = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let row = rows.row(self.row_height);
            rects.push(row);
            let item_id = format!("{id}.{index}");
            let response = ui.interact(&item_id, row);
            if response.hovered {
                ui.set_cursor(CursorIcon::Pointer);
            }
            if response.clicked && *selected != index {
                *selected = index;
                changed = true;
            }
            let chosen = *selected == index;
            if chosen {
                chosen_row = Some(row);
            } else if response.hovered {
                panel(ui, row, with_alpha(p.text, 0.05), [0.0; 4], 10.0);
            }
            if collapsed {
                ui.tooltip(&item_id, &item.label);
            }
        }
        // The highlight follows the chosen row rather than jumping to it.
        if let Some(row) = chosen_row {
            let y = ui.animate(&format!("{id}.highlight"), row.y, 0.18);
            let highlight = Rect { y, ..row };
            panel(
                ui,
                highlight,
                with_alpha(p.accent, 0.16),
                with_alpha(p.accent, 0.35),
                10.0,
            );
            panel(
                ui,
                Rect::new(
                    highlight.x,
                    highlight.y + 10.0,
                    3.0,
                    highlight.height - 20.0,
                ),
                p.accent,
                [0.0; 4],
                1.5,
            );
        }
        for (index, (item, row)) in self.items.iter().zip(rects).enumerate() {
            let chosen = *selected == index;
            let color = if chosen { p.text } else { p.muted };
            let inner = row.inset_by(crate::Insets::symmetric(14.0, 0.0));
            if collapsed {
                if let Some(icon) = item.icon {
                    icon.paint(ui, row.centered(20.0, 20.0), color);
                }
                continue;
            }
            let (icon_area, rest) = inner.split_left(20.0, 12.0);
            if let Some(icon) = item.icon {
                icon.paint(ui, icon_area.center_vertically(20.0), color);
            }
            let (text, badge) = match &item.badge {
                Some(badge) => {
                    let b =
                        Badge::new(StatusVariant::Info).color(item.badge_color.unwrap_or(p.accent));
                    let width = b.width(ui, badge).max(26.0);
                    let (text, slot) = rest.split_right(width, 8.0);
                    (text, Some((b, slot, badge)))
                }
                None => (rest, None),
            };
            Label::new()
                .size(14.0)
                .weight(if chosen { 600 } else { 500 })
                .color(color)
                .show(ui, text, &item.label);
            if let Some((badge, slot, value)) = badge {
                badge.show(ui, slot.center_vertically(22.0), value);
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Harness;

    #[test]
    fn clicking_an_item_selects_it() {
        let items = vec![
            SidebarItem::new("Overview").icon(Icon::Gauge),
            SidebarItem::new("Tasks").icon(Icon::Activity).badge("3"),
        ];
        let mut selected = 0;
        let mut harness = Harness::new(Rect::new(0.0, 0.0, 240.0, 400.0));
        let mut view = |ui: &mut Ui| {
            Sidebar::new(&items).show(ui, "nav", Rect::new(0.0, 0.0, 220.0, 200.0), &mut selected);
        };
        // The harness finds a control where the last frame drew it.
        harness.run(&mut view);
        let clicked = harness.click("nav.1", &mut view);
        assert!(clicked);
        assert_eq!(selected, 1);
    }
}
