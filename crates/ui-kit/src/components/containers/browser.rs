use dioxus::prelude::*;

/// Presentation-only browser chrome. Navigation and loading remain caller-owned.
#[component]
pub fn Browser(
    #[props(into)] address: String,
    #[props(into, default = "Browser preview".to_string())] aria_label: String,
    onaddresschange: EventHandler<String>,
    onnavigate: EventHandler<String>,
    #[props(default)] oninteract: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let pointer_interaction = oninteract;
    let keyboard_interaction = oninteract;
    let submitted_address = address.clone();
    rsx! {
        div {
            class: "uikit-browser",
            aria_label,
            onpointerdown: move |_| {
                if let Some(handler) = pointer_interaction {
                    handler.call(());
                }
            },
            onkeydown: move |_| {
                if let Some(handler) = keyboard_interaction {
                    handler.call(());
                }
            },
            div { class: "uikit-browser-bar",
                b { class: "uikit-browser-controls", aria_hidden: "true",
                    i {}
                    i {}
                    i {}
                }
                input {
                    class: "uikit-browser-address",
                    aria_label: "Address",
                    autocomplete: "off",
                    spellcheck: false,
                    value: address,
                    oninput: move |event| onaddresschange.call(event.value()),
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            onnavigate.call(submitted_address.clone());
                        }
                    },
                }
            }
            {children}
        }
    }
}

/// One controlled tab and the arbitrary UI the caller wants it to display.
#[derive(Clone, PartialEq)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub panel: Element,
}

impl Tab {
    pub fn new(id: impl Into<String>, label: impl Into<String>, panel: Element) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            panel,
        }
    }
}

/// A controlled tab strip and panel stack with no application behavior or selection state.
#[component]
pub fn TabbedContainer(
    tabs: Vec<Tab>,
    selected: usize,
    onselect: EventHandler<usize>,
    #[props(default)] onclose: Option<EventHandler<usize>>,
    #[props(default)] onnewtab: Option<EventHandler<()>>,
    #[props(into, default = "Tabs".to_string())] aria_label: String,
) -> Element {
    let close_tab = onclose;
    let new_tab = onnewtab;
    rsx! {
        div { class: "uikit-tabbed-container",
            div { class: "uikit-tabbed-container-tabs", role: "tablist", aria_label,
                for (index, tab) in tabs.iter().enumerate() {
                    div {
                        key: "{tab.id}",
                        class: if index == selected { "uikit-tab-item is-active" } else { "uikit-tab-item" },
                        button {
                            r#type: "button",
                            role: "tab",
                            class: "uikit-tab",
                            aria_selected: index == selected,
                            onclick: move |_| onselect.call(index),
                            "{tab.label}"
                        }
                        if let Some(handler) = close_tab {
                            button {
                                r#type: "button",
                                class: "uikit-tab-close",
                                aria_label: "Close {tab.label}",
                                onclick: move |event| {
                                    event.stop_propagation();
                                    handler.call(index);
                                },
                                "×"
                            }
                        }
                    }
                }
                if let Some(handler) = new_tab {
                    button {
                        r#type: "button",
                        class: "uikit-tab-new",
                        aria_label: "New tab",
                        onclick: move |_| handler.call(()),
                        "+"
                    }
                }
            }
            div { class: "uikit-tabbed-container-panels",
                for (index, tab) in tabs.iter().enumerate() {
                    div {
                        key: "{tab.id}",
                        class: if index == selected { "uikit-tab-panel is-active" } else { "uikit-tab-panel" },
                        aria_hidden: index != selected,
                        {tab.panel.clone()}
                    }
                }
            }
        }
    }
}
