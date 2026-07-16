use crate::components::info::metrics::bar::Bar;
use dioxus::prelude::*;

#[component]
pub fn Slider(
    value: f64,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default = false)] disabled: bool,
    on_change: EventHandler<f64>,
    #[props(into, default)] class: String,
) -> Element {
    let range = max - min;
    let progress_pct = if range > 0.0 {
        let clamped = value.clamp(min, max);
        (clamped - min) / range * 100.0
    } else {
        0.0
    };

    rsx! {
        div {
            class: "uikit-slider-container {class}",
            class: if disabled { "disabled" },
            Bar {
                value,
                min,
                max,
                class: "uikit-slider-bar",
                div {
                    class: "uikit-slider-thumb",
                    style: "left: {progress_pct}%",
                }
            }
            input {
                r#type: "range",
                class: "uikit-slider-input",
                min,
                max,
                step,
                value,
                disabled,
                oninput: move |evt| {
                    if let Ok(val) = evt.value().parse::<f64>() {
                        on_change.call(val);
                    }
                }
            }
        }
    }
}
