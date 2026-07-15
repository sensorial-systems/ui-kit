use dioxus::prelude::*;

#[component]
pub fn Unit(
    #[props(into)] value: String,
    #[props(into, default)] unit: Option<String>,
) -> Element {
    rsx! {
        span { class: "uikit-value-with-unit",
            span { class: "uikit-value", "{value}" }
            if let Some(u) = unit {
                span { class: "uikit-unit", "{u}" }
            }
        }
    }
}
