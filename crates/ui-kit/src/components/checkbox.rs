use dioxus::prelude::*;

#[component]
pub fn Checkbox(
    checked: bool,
    onchange: EventHandler<bool>,
    #[props(into, default)] label: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    let disabled_class = if disabled { "disabled" } else { "" };

    rsx! {
        label {
            class: "uikit-checkbox-container {disabled_class}",
            input {
                r#type: "checkbox",
                class: "uikit-checkbox-input",
                checked: checked,
                disabled: disabled,
                onchange: move |_| {
                    if !disabled {
                        onchange.call(!checked);
                    }
                }
            }
            span {
                class: "uikit-checkbox-box",
                if checked {
                    svg {
                        class: "uikit-checkbox-checkmark",
                        view_box: "0 0 24 24",
                        path {
                            d: "M20.285 2l-11.285 11.567-5.286-5.011-3.714 3.716 9 8.728 15-15.285z"
                        }
                    }
                }
            }
            if let Some(ref label_text) = label {
                span { class: "uikit-checkbox-label", "{label_text}" }
            }
        }
    }
}
