//! Resizer / splitter handle widget for immediate mode UI.

use crate::immediate::{Response, Ui};
use ui_kit_core::Rect;

/// A draggable split handle widget to resize adjacent panes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splitter {
    pub min_size: f32,
    pub max_size: f32,
    pub hit_width: f32,
    pub visual_width: f32,
}

impl Splitter {
    pub fn new(min_size: f32, max_size: f32) -> Self {
        Self {
            min_size,
            max_size,
            hit_width: 8.0,
            visual_width: 1.0,
        }
    }

    /// Render and interact with the splitter.
    ///
    /// `handle_rect` is the interactive area.
    /// `origin` is the reference coordinate from which `current_size` is measured (e.g. left edge of the pane).
    /// `current_size` is updated when dragged.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        handle_rect: Rect,
        origin: f32,
        current_size: &mut f32,
    ) -> Response {
        let response = ui.interact(id, handle_rect);
        if response.active {
            if let Some(p) = ui.input().pointer {
                let new_size = (p.x - origin).clamp(self.min_size, self.max_size);
                if *current_size != new_size {
                    *current_size = new_size;
                }
            }
        }

        // Draw visual divider line centered in handle_rect
        let line_x = handle_rect.x + (handle_rect.width - self.visual_width) * 0.5;
        let line_rect = Rect {
            x: line_x,
            y: handle_rect.y,
            width: self.visual_width,
            height: handle_rect.height,
        };
        let mut style = ui.style.clone();
        if response.active {
            style.fill = style.accent;
        } else if response.hovered {
            for c in &mut style.fill[..3] {
                *c = (*c + 0.15).min(1.0);
            }
            style.fill[3] = (style.fill[3] + 0.2).min(1.0);
        }
        let prev_style = std::mem::replace(&mut ui.style, style);
        ui.panel(line_rect);
        ui.style = prev_style;

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::Point;

    #[test]
    fn splitter_drags_and_clamps() {
        let splitter = Splitter::new(100.0, 400.0);
        let mut ui = Ui::default();
        let mut size = 200.0;
        let handle_rect = Rect {
            x: 200.0,
            y: 0.0,
            width: 10.0,
            height: 500.0,
        };

        // Press down inside handle
        let press_point = Point::new(205.0, 50.0);
        ui.begin(Input {
            pointer: Some(press_point),
            down: true,
            ..Default::default()
        });
        let resp = splitter.show(&mut ui, "split", handle_rect, 0.0, &mut size);
        ui.end();
        assert!(resp.active);

        // Drag to 350.0
        let drag_point = Point::new(350.0, 50.0);
        ui.begin(Input {
            pointer: Some(drag_point),
            down: true,
            ..Default::default()
        });
        let resp = splitter.show(&mut ui, "split", handle_rect, 0.0, &mut size);
        ui.end();
        assert!(resp.active);
        assert_eq!(size, 350.0);

        // Drag beyond max (500.0)
        let drag_max = Point::new(500.0, 50.0);
        ui.begin(Input {
            pointer: Some(drag_max),
            down: true,
            ..Default::default()
        });
        let _ = splitter.show(&mut ui, "split", handle_rect, 0.0, &mut size);
        ui.end();
        assert_eq!(size, 400.0);

        // Drag below min (50.0)
        let drag_min = Point::new(50.0, 50.0);
        ui.begin(Input {
            pointer: Some(drag_min),
            down: true,
            ..Default::default()
        });
        let _ = splitter.show(&mut ui, "split", handle_rect, 0.0, &mut size);
        ui.end();
        assert_eq!(size, 100.0);
    }
}
