use dioxus::prelude::*;

#[component]
pub fn Select(
    #[props(into)] value: String,
    onchange: EventHandler<String>,
    options: Vec<(String, String)>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    rsx! {
        div {
            class: "uikit-input-container",
            if let Some(ref label_text) = label {
                label { class: "uikit-input-label", "{label_text}" }
            }
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
