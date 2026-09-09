//! Number picker widget for immediate mode UI.

use crate::immediate::{Response, Ui};
use ui_kit_core::Rect;

/// A numeric stepper widget with decrement [-], value display, and increment [+] controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberPicker<T> {
    pub min: T,
    pub max: T,
    pub step: T,
}

impl<T> NumberPicker<T>
where
    T: Copy + PartialOrd + std::ops::Add<Output = T> + std::ops::Sub<Output = T> + std::fmt::Display,
{
    pub fn new(min: T, max: T, step: T) -> Self {
        Self { min, max, step }
    }

    /// Render and interact with the number picker within `rect`.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, value: &mut T) -> Response {
        let mut response = Response::default();
        let button_w = 28.0f32.min(rect.width * 0.25).max(20.0);
        let dec_r = Rect {
            x: rect.x,
            y: rect.y,
            width: button_w,
            height: rect.height,
        };
        let inc_r = Rect {
            x: rect.x + rect.width - button_w,
            y: rect.y,
            width: button_w,
            height: rect.height,
        };
        let val_r = Rect {
            x: rect.x + button_w,
            y: rect.y,
            width: (rect.width - button_w * 2.0).max(0.0),
            height: rect.height,
        };

        let dec_id = format!("{id}.dec");
        let dec_resp = ui.button(&dec_id, dec_r, "-");
        if dec_resp.clicked && *value > self.min {
            let next = *value - self.step;
            *value = if next < self.min { self.min } else { next };
            response.changed = true;
        }

        let inc_id = format!("{id}.inc");
        let inc_resp = ui.button(&inc_id, inc_r, "+");
        if inc_resp.clicked && *value < self.max {
            let next = *value + self.step;
            *value = if next > self.max { self.max } else { next };
            response.changed = true;
        }

        let val_text = format!("{}", *value);
        let mut val_style = ui.style.clone();
        val_style.text_align = ui_kit_core::TextAlign::Center;
        val_style.fill = [0.0; 4];
        val_style.stroke = [0.0; 4];
        val_style.text_padding = 0.0;
        let prev_style = std::mem::replace(&mut ui.style, val_style);
        ui.label(val_r, &val_text);
        ui.style = prev_style;

        response.hovered = dec_resp.hovered || inc_resp.hovered;
        response.active = dec_resp.active || inc_resp.active;
        response
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::Point;

    #[test]
    fn number_picker_increments_and_clamps() {
        let picker = NumberPicker::new(0, 5, 1);
        let mut ui = Ui::default();
        let mut val = 4;
        let rect = Rect { x: 10.0, y: 10.0, width: 100.0, height: 30.0 };

        // Click increment (+)
        let inc_center = Point::new(rect.x + rect.width - 10.0, rect.y + 15.0);
        ui.begin(Input { pointer: Some(inc_center), down: true, ..Default::default() });
        let _ = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        ui.begin(Input { pointer: Some(inc_center), down: false, ..Default::default() });
        let resp = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        assert!(resp.changed);
        assert_eq!(val, 5);

        // Click again: clamped at max 5
        ui.begin(Input { pointer: Some(inc_center), down: true, ..Default::default() });
        let _ = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        ui.begin(Input { pointer: Some(inc_center), down: false, ..Default::default() });
        let resp = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        assert!(!resp.changed);
        assert_eq!(val, 5);
    }

    #[test]
    fn number_picker_decrements_and_clamps() {
        let picker = NumberPicker::new(0, 5, 1);
        let mut ui = Ui::default();
        let mut val = 1;
        let rect = Rect { x: 10.0, y: 10.0, width: 100.0, height: 30.0 };

        // Click decrement (-)
        let dec_center = Point::new(rect.x + 10.0, rect.y + 15.0);
        ui.begin(Input { pointer: Some(dec_center), down: true, ..Default::default() });
        let _ = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        ui.begin(Input { pointer: Some(dec_center), down: false, ..Default::default() });
        let resp = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        assert!(resp.changed);
        assert_eq!(val, 0);

        // Click again: clamped at min 0
        ui.begin(Input { pointer: Some(dec_center), down: true, ..Default::default() });
        let _ = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        ui.begin(Input { pointer: Some(dec_center), down: false, ..Default::default() });
        let resp = picker.show(&mut ui, "test", rect, &mut val);
        ui.end();

        assert!(!resp.changed);
        assert_eq!(val, 0);
    }
}
