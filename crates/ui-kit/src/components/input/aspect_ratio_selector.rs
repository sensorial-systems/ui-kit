use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AspectRatio {
    #[default]
    Landscape16x9,
    Portrait9x16,
}

impl AspectRatio {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Landscape16x9 => "16:9",
            Self::Portrait9x16 => "9:16",
        }
    }

    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Landscape16x9 => (1920, 1080),
            Self::Portrait9x16 => (1080, 1920),
        }
    }

    pub const fn is_portrait(self) -> bool {
        matches!(self, Self::Portrait9x16)
    }
}

#[component]
pub fn AspectRatioSelector(
    #[props(default)] value: AspectRatio,
    #[props(default)] disabled: bool,
    #[props(default)] onchange: Option<EventHandler<AspectRatio>>,
    #[props(into, default = "Video aspect ratio".to_string())] aria_label: String,
) -> Element {
    let select = move |ratio: AspectRatio| {
        if !disabled && ratio != value {
            if let Some(handler) = onchange {
                handler.call(ratio);
            }
        }
    };

    rsx! {
        div {
            class: "uikit-aspect-ratio-selector",
            role: "group",
            aria_label,
            for ratio in [AspectRatio::Landscape16x9, AspectRatio::Portrait9x16] {
                button {
                    key: "{ratio.label()}",
                    r#type: "button",
                    class: if ratio == value {
                        "uikit-aspect-ratio-option uikit-aspect-ratio-option-active"
                    } else {
                        "uikit-aspect-ratio-option"
                    },
                    disabled,
                    aria_pressed: ratio == value,
                    onclick: move |_| select(ratio),
                    "{ratio.label()}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_expose_recording_dimensions() {
        assert_eq!(AspectRatio::Landscape16x9.dimensions(), (1920, 1080));
        assert_eq!(AspectRatio::Portrait9x16.dimensions(), (1080, 1920));
    }
}
