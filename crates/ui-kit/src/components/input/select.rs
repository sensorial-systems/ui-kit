use dioxus::prelude::*;
use super::{FormField, LabelLayout};

#[component]
pub fn Select(
    #[props(into)] value: String,
    onchange: EventHandler<String>,
    options: Vec<(String, String)>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] label_layout: LabelLayout,
    #[props(into, default)] alignment: Option<f32>,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        FormField {
            label: label,
            layout: label_layout,
            alignment: alignment,
            select {
                class: "uikit-select",
                value: "{value}",
                disabled: disabled,
                onchange: move |e| onchange.call(e.value()),
                for (val, label) in options {
                    option {
                        value: "{val}",
                        selected: val == value,
                        "{label}"
                    }
                }
            }
        }
    }
}

