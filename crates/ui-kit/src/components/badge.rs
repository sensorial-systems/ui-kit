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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeSize {
    #[default]
    Normal,
    Large,
}

impl BadgeSize {
    pub fn class_name(&self) -> &'static str {
        match self {
            BadgeSize::Normal => "uikit-badge-normal",
            BadgeSize::Large => "uikit-badge-lg",
        }
    }
}

#[component]
pub fn Badge(
    #[props(default)] variant: BadgeVariant,
    #[props(default)] size: BadgeSize,
    #[props(default = false)] borderless: bool,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();
    let size_class = size.class_name();
    let borderless_class = if borderless { "uikit-badge-borderless" } else { "" };

    rsx! {
        span {
            class: "uikit-badge {variant_class} {size_class} {borderless_class}",
            {children}
        }
    }
}
