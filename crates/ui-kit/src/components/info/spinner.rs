use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl SpinnerSize {
    pub fn class_name(&self) -> &'static str {
        match self {
            SpinnerSize::Small => "uikit-spinner-sm",
            SpinnerSize::Medium => "uikit-spinner-md",
            SpinnerSize::Large => "uikit-spinner-lg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
    Info,
    #[default]
    Inherit,
}

impl SpinnerVariant {
    pub fn class_name(&self) -> &'static str {
        match self {
            SpinnerVariant::Primary => "uikit-spinner-primary",
            SpinnerVariant::Secondary => "uikit-spinner-secondary",
            SpinnerVariant::Success => "uikit-spinner-success",
            SpinnerVariant::Warning => "uikit-spinner-warning",
            SpinnerVariant::Error => "uikit-spinner-error",
            SpinnerVariant::Info => "uikit-spinner-info",
            SpinnerVariant::Inherit => "uikit-spinner-inherit",
        }
    }
}

#[component]
pub fn Spinner(
    #[props(default)] size: SpinnerSize,
    #[props(default)] variant: SpinnerVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] style: Option<String>,
) -> Element {
    let size_class = size.class_name();
    let variant_class = variant.class_name();
    let extra_class = class.unwrap_or_default();
    let class_name = format!("uikit-spinner {size_class} {variant_class} {extra_class}");
    let style_attr = style.unwrap_or_default();

    rsx! {
        span {
            class: "{class_name}",
            style: "{style_attr}",
            role: "status",
            span {
                class: "uikit-sr-only",
                "Loading..."
            }
        }
    }
}
