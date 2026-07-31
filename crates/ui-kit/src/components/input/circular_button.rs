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
    /// Optional timeout in milliseconds for the loading state.
    #[props(default)] timeout_ms: Option<u64>,
    onclick: Option<EventHandler<MouseEvent>>,
    ontimeout: Option<EventHandler<()>>,
    onmouseenter: Option<EventHandler<MouseEvent>>,
    onmouseleave: Option<EventHandler<MouseEvent>>,
    #[props(into, default)] class: Option<String>,
    #[props(into, default)] style: Option<String>,
    #[props(into, default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let variant_class = variant.class_name();
    let size_class = match size {
        ButtonSize::Small => "uikit-circular-btn-sm",
        ButtonSize::Medium => "uikit-circular-btn-md",
        ButtonSize::Large => "uikit-circular-btn-lg",
    };
    let color_class = if color.is_some() { "uikit-btn-custom-color" } else { "" };
    let extra_class = class.unwrap_or_default();
    let combined_class = format!("uikit-btn uikit-circular-btn {variant_class} {size_class} {color_class} {extra_class}");
    
    let custom_style = color
        .as_ref()
        .map(|color| format!("--uikit-btn-color: {color};"))
        .unwrap_or_default();
    let extra_style = style.unwrap_or_default();
    let combined_style = format!("{custom_style} {extra_style}");

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

    let handle_mouseenter = move |e| {
        if let Some(ref handler) = onmouseenter {
            handler.call(e);
        }
    };

    let handle_mouseleave = move |e| {
        if let Some(ref handler) = onmouseleave {
            handler.call(e);
        }
    };

    let spinner_size = match size {
        ButtonSize::Small => SpinnerSize::Small,
        ButtonSize::Medium => SpinnerSize::Small,
        ButtonSize::Large => SpinnerSize::Medium,
    };

    let aria_lbl = aria_label.unwrap_or_default();

    rsx! {
        button {
            class: "{combined_class}",
            style: "{combined_style}",
            disabled: disabled || effective_loading,
            onclick: handle_click,
            onmouseenter: handle_mouseenter,
            onmouseleave: handle_mouseleave,
            aria_label: "{aria_lbl}",
            if effective_loading {
                Spinner {
                    size: spinner_size,
                    variant: SpinnerVariant::Inherit,
                }
            } else {
                {children}
            }
        }
    }
}
