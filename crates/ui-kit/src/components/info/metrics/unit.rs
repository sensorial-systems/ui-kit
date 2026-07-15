use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitSize {
    Small,
    Medium,
    #[default]
    Large,
}

impl UnitSize {
    pub fn class_name(&self) -> &'static str {
        match self {
            UnitSize::Small => "uikit-value-with-unit-sm",
            UnitSize::Medium => "uikit-value-with-unit-md",
            UnitSize::Large => "uikit-value-with-unit-lg",
        }
    }
}

#[component]
pub fn Unit(
    #[props(into)] value: String,
    #[props(into, default)] unit: Option<String>,
    #[props(default)] size: UnitSize,
) -> Element {
    let size_class = size.class_name();

    rsx! {
        span { class: "uikit-value-with-unit {size_class}",
            span { class: "uikit-value", "{value}" }
            if let Some(u) = unit {
                span { class: "uikit-unit", "{u}" }
            }
        }
    }
}
