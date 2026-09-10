//! Combobox (dropdown select) widget for immediate mode UI.

use crate::immediate::{Response, Ui};
use ui_kit_core::Rect;

/// A compact select dropdown control.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Combobox;

impl Combobox {
    pub fn new() -> Self {
        Self
    }

    /// Render and interact with the combobox within `rect`.
    pub fn show<T: Copy + PartialEq + std::fmt::Display>(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        value: &mut T,
        options: &[T],
    ) -> Response {
        let label = format!("{}  ▾", value);
        let trigger = ui.button(id, rect, &label);
        if trigger.clicked {
            if ui.is_combobox_open(id) {
                ui.close_combobox();
            } else {
                ui.open_combobox_id(id);
            }
        }
        let mut response = trigger;

        if ui.is_combobox_open(id) {
            let popup_rect = Rect {
                x: rect.x,
                y: rect.y + rect.height,
                width: rect.width,
                height: rect.height * options.len() as f32,
            };

            // Dismiss popup if pointer was pressed outside both trigger and popup
            if ui.input().down && !ui.previous_down() {
                if let Some(p) = ui.input().pointer {
                    if !rect.contains(p) && !popup_rect.contains(p) {
                        ui.close_combobox();
                        return response;
                    }
                }
            }

            ui.set_active_popup(Some(popup_rect));
            ui.set_in_overlay(true);

            // Soft shadow behind the entire dropdown menu
            ui.shadow_box(popup_rect, 6.0, 10.0, [0.0, 0.0, 0.0, 0.5]);

            // Solid opaque panel background
            let mut panel_style = ui.style.clone();
            panel_style.radius = 6.0;
            panel_style.fill[3] = 1.0;
            let prev_style = std::mem::replace(&mut ui.style, panel_style);
            ui.panel(popup_rect);
            ui.style = prev_style;

            // Render each option as a button
            for (index, option) in options.iter().enumerate() {
                let option_rect = Rect {
                    x: rect.x,
                    y: rect.y + rect.height * (index as f32 + 1.0),
                    width: rect.width,
                    height: rect.height,
                };
                let option_id = format!("{id}.option.{index}");
                let is_selected = *value == *option;
                let mut opt_style = ui.style.clone();
                opt_style.radius = 4.0;
                opt_style.fill[3] = 1.0;
                if is_selected {
                    opt_style.stroke = opt_style.accent;
                    opt_style.stroke_width = 1.0;
                }
                let prev_opt_style = std::mem::replace(&mut ui.style, opt_style);
                let option_response = ui.button(&option_id, option_rect, &option.to_string());
                ui.style = prev_opt_style;

                response.hovered |= option_response.hovered;
                response.active |= option_response.active;
                if option_response.clicked {
                    response.changed = *value != *option;
                    *value = *option;
                    ui.close_combobox();
                    ui.set_active_popup(None);
                }
            }

            ui.set_in_overlay(false);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immediate::Input;
    use ui_kit_core::Point as Vec2;

    #[test]
    fn combobox_opens_closes_and_selects_in_overlay() {
        let mut ui = Ui::default();
        let r = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 30.0,
        };
        let mut selected = 1;
        let options = [1, 2, 3];

        // 1. Initial render: combobox closed
        ui.begin(Input::default());
        ui.combobox("cb", r, &mut selected, &options);
        let cmds = ui.end();
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].text.contains("1"));

        // 2. Click trigger: down
        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: true,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        ui.end();

        // 3. Release trigger: up -> opened
        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: false,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        let cmds = ui.end();
        assert!(cmds.iter().any(|c| c.text == "2"));
        assert!(cmds.iter().any(|c| c.text == "3"));

        // 4. Click option 2: down
        let opt2_pos = Vec2::new(50.0, 10.0 + 30.0 * 2.5);
        ui.begin(Input {
            pointer: Some(opt2_pos),
            down: true,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        ui.end();

        // 5. Release option 2: up -> selected
        ui.begin(Input {
            pointer: Some(opt2_pos),
            down: false,
            ..Default::default()
        });
        let resp = ui.combobox("cb", r, &mut selected, &options);
        assert_eq!(selected, 2);
        assert!(resp.changed);
        ui.end();

        // 6. Next frame: closed, trigger displays updated value "2  ▾"
        ui.begin(Input::default());
        ui.combobox("cb", r, &mut selected, &options);
        let cmds = ui.end();
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].text.contains("2"));
        assert!(!cmds.iter().any(|c| c.text == "3"));
    }

    #[test]
    fn combobox_options_render_after_subsequent_widgets() {
        let mut ui = Ui::default();
        let r = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 30.0,
        };
        let mut selected = 1;
        let options = [1, 2];

        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: true,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        ui.end();
        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: false,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        let btn_r = Rect {
            x: 10.0,
            y: 40.0,
            width: 100.0,
            height: 30.0,
        };
        ui.button("subsequent", btn_r, "Subsequent");
        let cmds = ui.end();

        let sub_idx = cmds.iter().position(|c| c.text == "Subsequent").unwrap();
        let opt_idx = cmds.iter().position(|c| c.text == "2").unwrap();
        assert!(
            opt_idx > sub_idx,
            "overlay options must be rendered after subsequent widgets"
        );
    }

    #[test]
    fn combobox_dismisses_on_click_outside() {
        let mut ui = Ui::default();
        let r = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 30.0,
        };
        let mut selected = 1;
        let options = [1, 2];

        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: true,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        ui.end();
        ui.begin(Input {
            pointer: Some(Vec2::new(50.0, 25.0)),
            down: false,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        assert!(ui.is_combobox_open("cb"));
        ui.end();

        ui.begin(Input {
            pointer: Some(Vec2::new(300.0, 300.0)),
            down: true,
            ..Default::default()
        });
        ui.combobox("cb", r, &mut selected, &options);
        assert!(!ui.is_combobox_open("cb"));
    }
}
