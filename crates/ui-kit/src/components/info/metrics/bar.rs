use dioxus::prelude::*;

#[component]
pub fn Bar(
    value: f64,
    #[props(default = 0.0)] min: f64,
    #[props(default = 100.0)] max: f64,
    #[props(into, default)] class: String,
    #[props(into, default)] track_class: String,
    #[props(into, default)] fill_class: String,
    children: Element,
) -> Element {
    let range = max - min;
    let progress_pct = if range > 0.0 {
        let clamped = value.clamp(min, max);
        (clamped - min) / range * 100.0
    } else {
        0.0
    };

    rsx! {
        div { class: "uikit-bar-container {class}",
            div { class: "uikit-bar-track {track_class}",
                div {
                    class: "uikit-bar-fill {fill_class}",
                    style: "width: {progress_pct}%",
                }
                {children}
            }
        }
    }
}
