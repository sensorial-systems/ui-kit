use dioxus::prelude::*;

#[component]
pub fn Card(
    #[props(default)] shadowed: bool,
    #[props(default)] header: Option<Element>,
    #[props(default)] footer: Option<Element>,
    children: Element,
) -> Element {
    let shadow_class = if shadowed { "uikit-card-shadowed" } else { "" };

    rsx! {
        div {
            class: "uikit-card {shadow_class}",
            if let Some(ref header_elem) = header {
                div { class: "uikit-card-header", {header_elem} }
            }
            div { class: "uikit-card-body", {children} }
            if let Some(ref footer_elem) = footer {
                div { class: "uikit-card-footer", {footer_elem} }
            }
        }
    }
}
