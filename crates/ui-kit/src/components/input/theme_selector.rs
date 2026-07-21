use super::{LabelLayout, Select};
use crate::theme::AppTheme;
use dioxus::prelude::*;

#[component]
pub fn ThemeSelector(
    #[props(default)] theme: Option<Signal<AppTheme>>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] label_layout: LabelLayout,
    #[props(default)] disabled: bool,
) -> Element {
    let context_theme = try_use_context::<Signal<AppTheme>>();
    let active_theme = theme.or(context_theme);

    let (current_value, onchange_handler) = if let Some(mut theme_sig) = active_theme {
        let current_val = theme_sig.read().class_name().to_string();
        let handler = move |val: String| {
            if val == "uikit-theme-black" {
                theme_sig.set(AppTheme::Black);
            } else if val == "uikit-theme-white" {
                theme_sig.set(AppTheme::White);
            }
        };
        (current_val, Callback::new(handler))
    } else {
        ("uikit-theme-white".to_string(), Callback::new(|_| {}))
    };

    let options = vec![
        ("uikit-theme-black".to_string(), "Black".to_string()),
        ("uikit-theme-white".to_string(), "White".to_string()),
    ];

    rsx! {
        Select {
            value: current_value,
            onchange: move |val: String| onchange_handler.call(val),
            options: options,
            label: label.or_else(|| Some("Select Theme".to_string())),
            label_layout: label_layout,
            disabled: disabled,
        }
    }
}
