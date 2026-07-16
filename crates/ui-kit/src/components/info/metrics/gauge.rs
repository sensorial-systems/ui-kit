use crate::components::info::metrics::bar::Bar;
use dioxus::prelude::*;

#[component]
pub fn Gauge(
    value: f64,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(into, default)] min_label: Option<String>,
    #[props(into, default)] max_label: Option<String>,
    #[props(into, default)] class: String,
) -> Element {
    let has_labels = min_label.is_some() || max_label.is_some();
    rsx! {
        div { class: "uikit-gauge-container {class}",
            Bar {
                value,
                min,
                max,
                class: "uikit-gauge-bar",
            }
            if has_labels {
                div { class: "uikit-gauge-labels",
                    if let Some(ref min_lbl) = min_label {
                        span { class: "uikit-gauge-label-min", "{min_lbl}" }
                    } else {
                        span {}
                    }
                    if let Some(ref max_lbl) = max_label {
                        span { class: "uikit-gauge-label-max", "{max_lbl}" }
                    } else {
                        span {}
                    }
                }
            }
        }
    }
}
