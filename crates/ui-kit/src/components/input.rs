use dioxus::prelude::*;

#[component]
pub fn Input(
    #[props(into)] value: String,
    oninput: EventHandler<FormEvent>,
    #[props(into, default)] label: Option<String>,
    #[props(into, default)] placeholder: Option<String>,
    #[props(into, default)] error: Option<String>,
    #[props(into, default)] help_text: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, default = "text".to_string())] r#type: String,
) -> Element {
    let has_error = error.is_some();
    let container_class = if has_error {
        "uikit-input-container uikit-input-error"
    } else {
        "uikit-input-container"
    };

    rsx! {
        div {
            class: "{container_class}",
            if let Some(ref label_text) = label {
                label { class: "uikit-input-label", "{label_text}" }
            }
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
            if let Some(ref err_msg) = error {
                span { class: "uikit-input-err-text", "{err_msg}" }
            } else if let Some(ref help_msg) = help_text {
                span { class: "uikit-input-help", "{help_msg}" }
            }
        }
    }
}
