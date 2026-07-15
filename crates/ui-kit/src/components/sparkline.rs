use dioxus::prelude::*;

#[component]
pub fn Sparkline(
    data: Vec<f64>,
    #[props(default = 200.0)] width: f64,
    #[props(default = 50.0)] height: f64,
    #[props(default = false)] fill: bool,
) -> Element {
    let n = data.len();
    if n == 0 {
        return rsx! {
            div { class: "uikit-sparkline-no-data", "No data" }
        };
    }

    let min_val = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let points_str = if n == 1 {
        format!("0,{} {},{}", height / 2.0, width, height / 2.0)
    } else {
        let mut pts = String::new();
        for (i, &val) in data.iter().enumerate() {
            let x = i as f64 * (width / (n - 1) as f64);
            let y = if (max_val - min_val).abs() < f64::EPSILON {
                height / 2.0
            } else {
                let ratio = (val - min_val) / (max_val - min_val);
                (height - 4.0) - (ratio * (height - 8.0))
            };
            pts.push_str(&format!("{:.1},{:.1} ", x, y));
        }
        pts
    };

    let fill_path = if fill && n > 1 {
        let mut path = format!("0,{} ", height);
        for (i, &val) in data.iter().enumerate() {
            let x = i as f64 * (width / (n - 1) as f64);
            let y = if (max_val - min_val).abs() < f64::EPSILON {
                height / 2.0
            } else {
                let ratio = (val - min_val) / (max_val - min_val);
                (height - 4.0) - (ratio * (height - 8.0))
            };
            path.push_str(&format!("{:.1},{:.1} ", x, y));
        }
        path.push_str(&format!("{},{} ", width, height));
        Some(path)
    } else {
        None
    };

    let gradient_id = "uikit-sparkline-gradient";

    rsx! {
        svg {
            class: "uikit-sparkline-svg",
            view_box: "0 0 {width} {height}",
            preserve_aspect_ratio: "none",
            
            line { x1: "0", y1: "{height * 0.25}", x2: "{width}", y2: "{height * 0.25}", class: "uikit-sparkline-grid" }
            line { x1: "0", y1: "{height * 0.5}", x2: "{width}", y2: "{height * 0.5}", class: "uikit-sparkline-grid" }
            line { x1: "0", y1: "{height * 0.75}", x2: "{width}", y2: "{height * 0.75}", class: "uikit-sparkline-grid" }
            
            if fill {
                defs {
                    linearGradient { id: "{gradient_id}", x1: "0%", y1: "0%", x2: "0%", y2: "100%",
                        stop { offset: "0%", class: "uikit-sparkline-gradient-start" }
                        stop { offset: "100%", class: "uikit-sparkline-gradient-stop" }
                    }
                }
            }

            if let Some(ref path) = fill_path {
                polygon {
                    class: "uikit-sparkline-fill",
                    points: "{path}",
                    fill: "url(#{gradient_id})"
                }
            }

            polyline {
                class: "uikit-sparkline-path",
                points: "{points_str}"
            }
        }
    }
}
