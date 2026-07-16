use dioxus::prelude::*;
use super::{FormField, LabelLayout};

#[component]
pub fn TextInput(
    #[props(into)] value: String,
    oninput: EventHandler<FormEvent>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] label_layout: LabelLayout,
    #[props(into, default)] alignment: Option<f32>,
    #[props(into, default)] placeholder: Option<String>,
    #[props(into, default)] error: Option<String>,
    #[props(into, default)] help_text: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, default = "text".to_string())] r#type: String,
) -> Element {
    rsx! {
        FormField {
            label: label,
            layout: label_layout,
            alignment: alignment,
            error: error,
            help_text: help_text,
            div {
                class: "uikit-input-wrapper",
                input {
                    class: "uikit-input",
                    r#type: "{r#type}",
                    value: "{value}",
                    placeholder: placeholder,
                    disabled: disabled,
                    oninput: move |e| oninput.call(e),
                }
            }
        }
    }
}

