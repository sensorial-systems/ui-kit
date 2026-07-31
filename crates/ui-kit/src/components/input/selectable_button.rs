use dioxus::prelude::*;

#[component]
pub fn SelectableButton(
    #[props(into, default)] label: Option<String>,
    #[props(default)] selected: bool,
    #[props(default)] disabled: bool,
    onselect: Option<EventHandler<bool>>,
    #[props(into, default)] icon: Option<String>,
    children: Option<Element>,
) -> Element {
    let handle_click = move |_| {
        if !disabled {
            if let Some(ref handler) = onselect {
                handler.call(!selected);
            }
        }
    };

    let selected_class = if selected { "uikit-selectable-button-selected" } else { "" };

    rsx! {
        button {
            type: "button",
            class: "uikit-selectable-button {selected_class}",
            disabled: disabled,
            onclick: handle_click,
            if let Some(ref icon_str) = icon {
                span { class: "uikit-selectable-button-icon", "{icon_str}" }
            }
            if let Some(ref lbl) = label {
                "{lbl}"
            }
            if let Some(ref child_elems) = children {
                {child_elems}
            }
        }
    }
}


