use dioxus::prelude::*;

#[component]
pub fn Switch(
    checked: bool,
    onchange: EventHandler<bool>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    let disabled_class = if disabled { "disabled" } else { "" };

    rsx! {
        label {
            class: "uikit-switch-container {disabled_class}",
            input {
                r#type: "checkbox",
                class: "uikit-switch-input",
                checked: checked,
                disabled: disabled,
                onchange: move |_| {
                    if !disabled {
                        onchange.call(!checked);
                    }
                }
            }
            span {
                class: "uikit-switch-track",
                span { class: "uikit-switch-thumb" }
            }
            if let Some(ref label_text) = label {
                span { class: "uikit-switch-label", "{label_text}" }
            }
        }
    }
}
