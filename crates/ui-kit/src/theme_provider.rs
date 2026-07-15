use crate::theme::AppTheme;
use dioxus::prelude::*;

const THEME_CSS: &str = include_str!("theme.css");

#[component]
pub fn ThemeProvider(
    #[props(default)] theme: Option<Signal<AppTheme>>,
    children: Element,
) -> Element {
    // If no signal is provided, manage the theme internally (default to AppTheme::default())
    let active_theme = theme.unwrap_or_else(|| use_signal(AppTheme::default));

    // Provide active theme signal to children components
    use_context_provider(|| active_theme);

    let theme_val = *active_theme.read();
    let class = theme_val.class_name();

    rsx! {
        style { {THEME_CSS} }
        div {
            class: "uikit-root {class}",
            style: "min-height: 100vh; background-color: var(--uikit-bg); color: var(--uikit-fg); font-family: var(--uikit-font-sans); transition: var(--uikit-transition-normal); margin: 0; padding: 0;",
            {children}
        }
    }
}
