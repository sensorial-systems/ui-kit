use dioxus::prelude::*;
use crate::components::info::{Spinner, SpinnerSize, SpinnerVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Text,
}

impl ButtonVariant {
    pub fn class_name(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "uikit-btn-primary",
            ButtonVariant::Secondary => "uikit-btn-secondary",
            ButtonVariant::Outline => "uikit-btn-outline",
            ButtonVariant::Text => "uikit-btn-text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl ButtonSize {
    pub fn class_name(&self) -> &'static str {
        match self {
            ButtonSize::Small => "uikit-btn-sm",
            ButtonSize::Medium => "uikit-btn-md",
            ButtonSize::Large => "uikit-btn-lg",
        }
    }
}

#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    /// Optional CSS color used as the button's background and border color.
    #[props(into, default)] color: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] loading: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();
    let size_class = size.class_name();
    let color_class = if color.is_some() { "uikit-btn-custom-color" } else { "" };
    let custom_style = color
        .as_ref()
        .map(|color| format!("--uikit-btn-color: {color};"))
        .unwrap_or_default();

    let handle_click = move |e| {
        if !disabled && !loading {
            if let Some(ref handler) = onclick {
                handler.call(e);
            }
        }
    };

    rsx! {
        button {
            class: "uikit-btn {variant_class} {size_class} {color_class}",
            style: "{custom_style}",
            disabled: disabled || loading,
            onclick: handle_click,
            if loading {
                Spinner {
                    size: SpinnerSize::Small,
                    variant: SpinnerVariant::Inherit,
                    class: "uikit-btn-spinner",
                }
            }
            {children}
        }
    }
}
