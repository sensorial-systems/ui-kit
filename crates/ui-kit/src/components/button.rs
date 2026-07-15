use dioxus::prelude::*;

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
    #[props(default)] disabled: bool,
    #[props(default)] loading: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();
    let size_class = size.class_name();

    let handle_click = move |e| {
        if !disabled && !loading {
            if let Some(ref handler) = onclick {
                handler.call(e);
            }
        }
    };

    rsx! {
        button {
            class: "uikit-btn {variant_class} {size_class}",
            disabled: disabled || loading,
            onclick: handle_click,
            if loading {
                span { class: "uikit-btn-spinner" }
            }
            {children}
        }
    }
}
