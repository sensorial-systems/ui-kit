use dioxus::prelude::*;
use dioxus::document::eval;
use super::{FormField, LabelLayout};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditableTextVariant {
    #[default]
    Full,
    /// Edits the rendered text in place without showing the formatting toolbar.
    Inline,
}

#[component]
pub fn EditableText(
    #[props(into)] value: String,
    onchange: EventHandler<String>,
    #[props(default)] variant: EditableTextVariant,
    #[props(into, default)] placeholder: Option<String>,
    #[props(default)] disabled: bool,
    #[props(into, default)] label: Option<String>,
    #[props(default)] label_layout: LabelLayout,
    #[props(into, default)] alignment: Option<f32>,
    #[props(into, default)] error: Option<String>,
    #[props(into, default)] help_text: Option<String>,
) -> Element {
    let unique_id = *use_signal(|| NEXT_ID.fetch_add(1, Ordering::SeqCst)).read();
    let editing = use_signal(|| false);
    let mut local_value = use_signal(|| value.clone());

    // Sync parent prop changes to local value when not editing
    if !*editing.read() && *local_value.read() != value {
        local_value.set(value.clone());
    }

    use_effect(move || {
        if *editing.read() {
            let _ = eval(&format!(
                r#"
                setTimeout(() => {{
                    let el = document.getElementById("uikit-wysiwyg-editor-{}");
                    if (el) {{
                        el.focus();
                        // Position cursor at the end of the content
                        let range = document.createRange();
                        let sel = window.getSelection();
                        range.selectNodeContents(el);
                        range.collapse(false);
                        sel.removeAllRanges();
                        sel.addRange(range);
                    }}
                }}, 50);
                "#,
                unique_id
            ));
        }
    });

    let exec_cmd = move |cmd: &'static str| {
        let _ = eval(&format!("document.execCommand('{}', false, null);", cmd));
    };

    let exec_cmd_val = move |cmd: &'static str, val: &'static str| {
        let _ = eval(&format!("document.execCommand('{}', false, '{}');", cmd, val));
    };

    let placeholder_str = placeholder.unwrap_or_else(|| "Double click to write rich text...".to_string());

    rsx! {
        FormField {
            label: label,
            layout: label_layout,
            alignment: alignment,
            error: error,
            help_text: help_text,
            div {
                class: "uikit-wysiwyg-wrapper",
                if *editing.read() {
                    if variant == EditableTextVariant::Inline {
                        div {
                            id: "uikit-wysiwyg-editor-{unique_id}",
                            class: "uikit-wysiwyg-editor uikit-wysiwyg-editor-inline",
                            contenteditable: true,
                            "data-placeholder": "{placeholder_str}",
                            dangerous_inner_html: "{local_value}",
                            onblur: move |_| {
                                let mut editing = editing;
                                let mut local_value = local_value;
                                let mut eval_handle = eval(&format!(
                                    r#"
                                    let el = document.getElementById("uikit-wysiwyg-editor-{}");
                                    dioxus.send(el ? el.innerHTML : "");
                                    "#,
                                    unique_id
                                ));
                                spawn(async move {
                                    if let Ok(html) = eval_handle.recv::<String>().await {
                                        let trimmed = html.trim();
                                        let clean_html = if trimmed == "<br>" { "".to_string() } else { html };
                                        local_value.set(clean_html.clone());
                                        onchange.call(clean_html);
                                    }
                                    editing.set(false);
                                });
                            },
                            onkeydown: move |event| {
                                event.stop_propagation();
                                if event.key() == Key::Escape {
                                    let mut editing = editing;
                                    editing.set(false);
                                } else if event.key() == Key::Enter && event.modifiers().ctrl() {
                                    event.prevent_default();
                                    let _ = eval("document.activeElement?.blur();");
                                }
                            }
                        }
                    } else {
                    div {
                        class: "uikit-wysiwyg-container",
                        div {
                            class: "uikit-wysiwyg-toolbar",
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Bold",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("bold");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" }
                                    path { d: "M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Italic",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("italic");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "19", y1: "4", x2: "10", y2: "4" }
                                    line { x1: "14", y1: "20", x2: "5", y2: "20" }
                                    line { x1: "15", y1: "4", x2: "9", y2: "20" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Underline",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("underline");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M6 3v7a6 6 0 0 0 6 6 6 6 0 0 0 6-6V3" }
                                    line { x1: "4", y1: "21", x2: "20", y2: "21" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Strikethrough",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("strikeThrough");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M16 4H9a4 4 0 0 0-4 4v1a4 4 0 0 0 4 4h6a4 4 0 0 1 4 4v1a4 4 0 0 1-4 4H7" }
                                    line { x1: "4", y1: "12", x2: "20", y2: "12" }
                                }
                            }
                            div { class: "uikit-wysiwyg-divider" }
                            button {
                                class: "uikit-wysiwyg-btn text-font-btn",
                                r#type: "button",
                                title: "Heading 1",
                                style: "font-weight: 800; font-size: 11px;",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd_val("formatBlock", "h1");
                                },
                                "H1"
                            }
                            button {
                                class: "uikit-wysiwyg-btn text-font-btn",
                                r#type: "button",
                                title: "Heading 2",
                                style: "font-weight: 750; font-size: 11px;",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd_val("formatBlock", "h2");
                                },
                                "H2"
                            }
                            button {
                                class: "uikit-wysiwyg-btn text-font-btn",
                                r#type: "button",
                                title: "Paragraph",
                                style: "font-weight: 500; font-size: 11px;",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd_val("formatBlock", "p");
                                },
                                "P"
                            }
                            div { class: "uikit-wysiwyg-divider" }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Bullet List",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("insertUnorderedList");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "8", y1: "6", x2: "21", y2: "6" }
                                    line { x1: "8", y1: "12", x2: "21", y2: "12" }
                                    line { x1: "8", y1: "18", x2: "21", y2: "18" }
                                    line { x1: "3", y1: "6", x2: "3.01", y2: "6" }
                                    line { x1: "3", y1: "12", x2: "3.01", y2: "12" }
                                    line { x1: "3", y1: "18", x2: "3.01", y2: "18" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Numbered List",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("insertOrderedList");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "10", y1: "6", x2: "21", y2: "6" }
                                    line { x1: "10", y1: "12", x2: "21", y2: "12" }
                                    line { x1: "10", y1: "18", x2: "21", y2: "18" }
                                    path { d: "M4 6h1v4" }
                                    path { d: "M4 10h2" }
                                    path { d: "M6 6H4" }
                                }
                            }
                            div { class: "uikit-wysiwyg-divider" }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Align Left",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("justifyLeft");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "17", y1: "10", x2: "3", y2: "10" }
                                    line { x1: "21", y1: "6", x2: "3", y2: "6" }
                                    line { x1: "21", y1: "14", x2: "3", y2: "14" }
                                    line { x1: "17", y1: "18", x2: "3", y2: "18" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Align Center",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("justifyCenter");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "18", y1: "10", x2: "6", y2: "10" }
                                    line { x1: "21", y1: "6", x2: "3", y2: "6" }
                                    line { x1: "21", y1: "14", x2: "3", y2: "14" }
                                    line { x1: "18", y1: "18", x2: "6", y2: "18" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Align Right",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("justifyRight");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "21", y1: "10", x2: "7", y2: "10" }
                                    line { x1: "21", y1: "6", x2: "3", y2: "6" }
                                    line { x1: "21", y1: "14", x2: "3", y2: "14" }
                                    line { x1: "21", y1: "18", x2: "7", y2: "18" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn",
                                r#type: "button",
                                title: "Clear Formatting",
                                onmousedown: move |e| {
                                    e.prevent_default();
                                    exec_cmd("removeFormat");
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    path { d: "M18 12V4H6v2" }
                                    path { d: "M14 4v16" }
                                    path { d: "M18 20H10" }
                                    line { x1: "4", y1: "20", x2: "20", y2: "4" }
                                }
                            }
                            div { style: "flex-grow: 1;" }
                            button {
                                class: "uikit-wysiwyg-btn uikit-wysiwyg-action-btn",
                                r#type: "button",
                                title: "Cancel (Esc)",
                                onclick: move |_| {
                                    let mut editing = editing;
                                    editing.set(false);
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "var(--uikit-error)",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    line { x1: "18", y1: "6", x2: "6", y2: "18" }
                                    line { x1: "6", y1: "6", x2: "18", y2: "18" }
                                }
                            }
                            button {
                                class: "uikit-wysiwyg-btn uikit-wysiwyg-action-btn",
                                r#type: "button",
                                title: "Save (Ctrl+Enter)",
                                onclick: move |_| {
                                    let mut editing = editing;
                                    let mut local_value = local_value;
                                    let mut eval_handle = eval(&format!(
                                        r#"
                                        let el = document.getElementById("uikit-wysiwyg-editor-{}");
                                        if (el) {{
                                            dioxus.send(el.innerHTML);
                                        }} else {{
                                            dioxus.send("");
                                        }}
                                        "#,
                                        unique_id
                                    ));
                                    
                                    spawn(async move {
                                        if let Ok(html) = eval_handle.recv::<String>().await {
                                            let trimmed = html.trim();
                                            let clean_html = if trimmed == "<br>" || trimmed == "<p><br></p>" || trimmed == "<div><br></div>" {
                                                "".to_string()
                                            } else {
                                                html
                                            };
                                            local_value.set(clean_html.clone());
                                            onchange.call(clean_html);
                                        }
                                        editing.set(false);
                                    });
                                },
                                svg {
                                    width: "14",
                                    height: "14",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "var(--uikit-success)",
                                    stroke_width: "2.5",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    polyline { points: "20 6 9 17 4 12" }
                                }
                            }
                        }
                        div {
                            id: "uikit-wysiwyg-editor-{unique_id}",
                            class: "uikit-wysiwyg-editor",
                            contenteditable: true,
                            "data-placeholder": "{placeholder_str}",
                            dangerous_inner_html: "{local_value}",
                            onkeydown: move |e| {
                                let mut editing = editing;
                                let mut local_value = local_value;
                                if e.key() == Key::Escape {
                                    editing.set(false);
                                } else if e.key() == Key::Enter && e.modifiers().ctrl() {
                                    let mut eval_handle = eval(&format!(
                                        r#"
                                        let el = document.getElementById("uikit-wysiwyg-editor-{}");
                                        if (el) {{
                                            dioxus.send(el.innerHTML);
                                        }} else {{
                                            dioxus.send("");
                                        }}
                                        "#,
                                        unique_id
                                    ));
                                    
                                    spawn(async move {
                                        if let Ok(html) = eval_handle.recv::<String>().await {
                                            let trimmed = html.trim();
                                            let clean_html = if trimmed == "<br>" || trimmed == "<p><br></p>" || trimmed == "<div><br></div>" {
                                                "".to_string()
                                            } else {
                                                html
                                            };
                                            local_value.set(clean_html.clone());
                                            onchange.call(clean_html);
                                        }
                                        editing.set(false);
                                    });
                                }
                            }
                        }
                    }
                    }
                } else {
                    div {
                        class: "uikit-wysiwyg-viewer-wrapper",
                        ondoubleclick: move |_| {
                            if !disabled {
                                let mut editing = editing;
                                editing.set(true);
                            }
                        },
                        if local_value.read().is_empty() {
                            div {
                                class: "uikit-wysiwyg-viewer uikit-wysiwyg-viewer-empty",
                                "{placeholder_str}"
                            }
                        } else {
                            div {
                                class: "uikit-wysiwyg-viewer",
                                dangerous_inner_html: "{local_value}"
                            }
                        }
                        if !disabled {
                            span {
                                class: "uikit-wysiwyg-hint",
                                "Double-click to edit"
                            }
                        }
                    }
                }
            }
        }
    }
}
