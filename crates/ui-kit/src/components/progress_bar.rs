use dioxus::prelude::*;

#[component]
pub fn ProgressBar(
    value: f64,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(into, default)] min_label: Option<String>,
    #[props(into, default)] max_label: Option<String>,
) -> Element {
    let range = max - min;
    let progress_pct = if range > 0.0 {
        let clamped = value.clamp(min, max);
        (clamped - min) / range * 100.0
    } else {
        0.0
    };

    let has_labels = min_label.is_some() || max_label.is_some();

    rsx! {
        div { class: "uikit-progress-bar-container",
            div {
                class: "uikit-progress-bar-track",
                div {
                    class: "uikit-progress-bar-fill",
                    style: "width: {progress_pct}%",
                }
            }
            if has_labels {
                div { class: "uikit-progress-bar-labels",
                    if let Some(ref min_lbl) = min_label {
                        span { class: "uikit-progress-bar-label-min", "{min_lbl}" }
                    } else {
                        span {}
                    }
                    if let Some(ref max_lbl) = max_label {
                        span { class: "uikit-progress-bar-label-max", "{max_lbl}" }
                    } else {
                        span {}
                    }
                }
            }
        }
    }
}
