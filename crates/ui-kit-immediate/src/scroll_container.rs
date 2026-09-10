//! Scrollable container widget with toggleable scrollbar for immediate mode UI.

use crate::immediate::{Response, Ui};
use ui_kit_core::Rect;

/// A container widget that clips content to a viewport and optionally displays an interactive scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollContainer {
    pub scrollbar_enabled: bool,
    pub scrollbar_width: f32,
}

impl Default for ScrollContainer {
    fn default() -> Self {
        Self {
            scrollbar_enabled: true,
            scrollbar_width: 6.0,
        }
    }
}

impl ScrollContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scrollbar(mut self, enabled: bool) -> Self {
        self.scrollbar_enabled = enabled;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.scrollbar_width = width;
        self
    }

    /// Compute the viewport rectangle, clamp scroll offset, handle scrollbar interaction, and paint the scrollbar.
    ///
    /// Returns the inner `viewport` `Rect` for clipping content, and the clamped `scroll` offset.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        bounds: Rect,
        scroll: &mut f32,
        content_height: f32,
    ) -> (Rect, Response) {
        let max_scroll = (content_height - bounds.height).max(0.0);
        *scroll = (*scroll).clamp(0.0, max_scroll);

        let show_bar = self.scrollbar_enabled && max_scroll > 0.0 && bounds.height > 0.0;
        let bar_w = if show_bar { self.scrollbar_width } else { 0.0 };
        let viewport = Rect {
            x: bounds.x,
            y: bounds.y,
            width: (bounds.width - bar_w - if show_bar { 3.0 } else { 0.0 }).max(0.0),
            height: bounds.height,
        };

        let mut bar_response = Response::default();
        if show_bar {
            let track_x = bounds.x + bounds.width - self.scrollbar_width;
            let track_rect = Rect {
                x: track_x,
                y: bounds.y,
                width: self.scrollbar_width,
                height: bounds.height,
            };

            let thumb_height =
                ((bounds.height / content_height) * bounds.height).clamp(24.0, bounds.height);
            let travel = bounds.height - thumb_height;
            let fraction = if max_scroll > 0.0 {
                *scroll / max_scroll
            } else {
                0.0
            };
            let thumb_y = bounds.y + fraction * travel;
            let thumb_rect = Rect {
                x: track_x,
                y: thumb_y,
                width: self.scrollbar_width,
                height: thumb_height,
            };

            let track_id = format!("{id}.scrollbar");
            bar_response = ui.interact(&track_id, track_rect);
            if bar_response.active {
                if let Some(p) = ui.input().pointer {
                    let rel_y = (p.y - bounds.y - thumb_height * 0.5).clamp(0.0, travel);
                    if travel > 0.0 {
                        *scroll = (rel_y / travel) * max_scroll;
                    }
                }
            }

            // Draw track
            let mut track_style = ui.style.clone();
            track_style.fill = [0.0; 4];
            track_style.stroke = [0.0; 4];
            track_style.stroke_width = 0.0;
            let prev_style = std::mem::replace(&mut ui.style, track_style);
            ui.panel(track_rect);

            // Draw thumb pill
            let mut thumb_style = prev_style.clone();
            thumb_style.radius = self.scrollbar_width * 0.5;
            thumb_style.stroke = [0.0; 4];
            thumb_style.stroke_width = 0.0;
            if bar_response.active {
                thumb_style.fill = prev_style.accent;
            } else if bar_response.hovered {
                for c in &mut thumb_style.fill[..3] {
                    *c = (*c + 0.25).min(1.0);
                }
                thumb_style.fill[3] = 0.7;
            } else {
                thumb_style.fill[3] = 0.4;
            }
            ui.style = thumb_style;
            ui.panel(thumb_rect);
            ui.style = prev_style;
        }

        (viewport, bar_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::Point;

    #[test]
    fn scroll_container_clamps_and_drags() {
        let container = ScrollContainer::new().with_width(8.0);
        let mut ui = Ui::default();
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let mut scroll = 0.0;
        let content_height = 500.0; // max_scroll = 400.0

        let (viewport, resp) =
            container.show(&mut ui, "scroll", bounds, &mut scroll, content_height);
        assert_eq!(viewport.width, 200.0 - 8.0 - 3.0);
        assert!(!resp.active);

        // Toggle disabled scrollbar
        let disabled_container = ScrollContainer::new().with_scrollbar(false);
        let (vp2, _) =
            disabled_container.show(&mut ui, "scroll2", bounds, &mut scroll, content_height);
        assert_eq!(vp2.width, 200.0);

        // Drag scrollbar
        let drag_point = Point::new(196.0, 50.0);
        ui.begin(Input {
            pointer: Some(drag_point),
            down: true,
            ..Default::default()
        });
        let (_, drag_resp) =
            container.show(&mut ui, "scroll_drag", bounds, &mut scroll, content_height);
        ui.end();

        assert!(drag_resp.active);
        assert!(scroll > 0.0);
    }
}
