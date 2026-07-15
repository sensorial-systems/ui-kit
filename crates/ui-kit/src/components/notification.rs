use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationVariant {
    pub fn class_name(&self) -> &'static str {
        match self {
            NotificationVariant::Info => "uikit-notification-info",
            NotificationVariant::Success => "uikit-notification-success",
            NotificationVariant::Warning => "uikit-notification-warning",
            NotificationVariant::Error => "uikit-notification-error",
        }
    }
}

#[component]
pub fn Notification(
    #[props(default)] variant: NotificationVariant,
    #[props(into, default)] title: Option<String>,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();

    rsx! {
        div {
            class: "uikit-notification {variant_class}",
            if let Some(ref title_text) = title {
                div { class: "uikit-notification-title", "{title_text}" }
            }
            div { class: "uikit-notification-body", {children} }
        }
    }
}
