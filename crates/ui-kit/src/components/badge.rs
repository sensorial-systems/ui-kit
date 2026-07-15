use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Success,
    Warning,
    Error,
    Info,
}

impl BadgeVariant {
    pub fn class_name(&self) -> &'static str {
        match self {
            BadgeVariant::Default => "uikit-badge-default",
            BadgeVariant::Success => "uikit-badge-success",
            BadgeVariant::Warning => "uikit-badge-warning",
            BadgeVariant::Error => "uikit-badge-error",
            BadgeVariant::Info => "uikit-badge-info",
        }
    }
}

#[component]
pub fn Badge(#[props(default)] variant: BadgeVariant, children: Element) -> Element {
    let variant_class = variant.class_name();

    rsx! {
        span {
            class: "uikit-badge {variant_class}",
            {children}
        }
    }
}
