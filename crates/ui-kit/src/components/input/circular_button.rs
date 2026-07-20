use dioxus::prelude::*;
use crate::components::info::{Spinner, SpinnerSize, SpinnerVariant};
use crate::components::input::{ButtonSize, ButtonVariant};

#[component]
pub fn CircularButton(
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
    let size_class = match size {
        ButtonSize::Small => "uikit-circular-btn-sm",
        ButtonSize::Medium => "uikit-circular-btn-md",
        ButtonSize::Large => "uikit-circular-btn-lg",
    };
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

    let spinner_size = match size {
        ButtonSize::Small => SpinnerSize::Small,
        ButtonSize::Medium => SpinnerSize::Small,
        ButtonSize::Large => SpinnerSize::Medium,
    };

    rsx! {
        button {
            class: "uikit-btn uikit-circular-btn {variant_class} {size_class} {color_class}",
            style: "{custom_style}",
            disabled: disabled || loading,
            onclick: handle_click,
            if loading {
                Spinner {
                    size: spinner_size,
                    variant: SpinnerVariant::Inherit,
                }
            } else {
                span {
                    class: "uikit-circular-btn-content",
                    {children}
                }
            }
        }
    }
}
