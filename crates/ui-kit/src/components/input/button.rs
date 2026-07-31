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
    /// Optional timeout in milliseconds for the loading state.
    #[props(default)] timeout_ms: Option<u64>,
    onclick: Option<EventHandler<MouseEvent>>,
    ontimeout: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();
    let size_class = size.class_name();
    let color_class = if color.is_some() { "uikit-btn-custom-color" } else { "" };
    let custom_style = color
        .as_ref()
        .map(|color| format!("--uikit-btn-color: {color};"))
        .unwrap_or_default();

    let mut is_timed_out = use_signal(|| false);

    use_effect(use_reactive((&loading, &timeout_ms), move |(loading, timeout_ms)| {
        if loading {
            is_timed_out.set(false);
            if let Some(ms) = timeout_ms {
                spawn(async move {
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(ms as u32).await;
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    
                    is_timed_out.set(true);
                    if let Some(ref handler) = ontimeout {
                        handler.call(());
                    }
                });
            }
        } else {
            is_timed_out.set(false);
        }
    }));

    let effective_loading = loading && !*is_timed_out.read();

    let handle_click = move |e| {
        if !disabled && !effective_loading {
            if let Some(ref handler) = onclick {
                handler.call(e);
            }
        }
    };

    rsx! {
        button {
            class: "uikit-btn {variant_class} {size_class} {color_class}",
            style: "{custom_style}",
            disabled: disabled || effective_loading,
            onclick: handle_click,
            if effective_loading {
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
