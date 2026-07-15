use dioxus::prelude::*;

#[component]
pub fn Modal(
    open: bool,
    onclose: EventHandler<()>,
    #[props(into, default)] title: Option<String>,
    #[props(default)] footer: Option<Element>,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }

    rsx! {
        div {
            class: "uikit-modal-backdrop",
            onclick: move |_| onclose.call(()),
            div {
                class: "uikit-modal-content",
                // Prevent click on modal content from closing the modal
                onclick: move |e| e.stop_propagation(),
                if let Some(ref title_text) = title {
                    div {
                        class: "uikit-card-header",
                        style: "display: flex; justify-content: space-between; align-items: center;",
                        span { "{title_text}" }
                        button {
                            style: "background: transparent; border: none; font-size: 20px; cursor: pointer; color: var(--uikit-fg);",
                            onclick: move |_| onclose.call(()),
                            "×"
                        }
                    }
                }
                div { class: "uikit-card-body", {children} }
                if let Some(ref footer_elem) = footer {
                    div { class: "uikit-card-footer", {footer_elem} }
                }
            }
        }
    }
}
