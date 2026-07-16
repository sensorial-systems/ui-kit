use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelLayout {
    #[default]
    Top,
    Left,
}

impl LabelLayout {
    pub fn class_name(&self) -> &'static str {
        match self {
            LabelLayout::Top => "uikit-field-layout-top",
            LabelLayout::Left => "uikit-field-layout-left",
        }
    }
}

#[component]
pub fn FormField(
    #[props(into, default)] label: Option<String>,
    #[props(default)] layout: LabelLayout,
    #[props(into, default)] alignment: Option<f32>,
    #[props(into, default)] error: Option<String>,
    #[props(into, default)] help_text: Option<String>,
    children: Element,
) -> Element {
    let layout_class = layout.class_name();
    let has_error = error.is_some();
    let error_class = if has_error { "uikit-input-error uikit-field-error" } else { "" };
    let has_extra = error.is_some() || help_text.is_some();
    let extra_class = if has_extra { "uikit-field-has-extra" } else { "" };

    let label_style = if let Some(width) = alignment {
        format!("min-width: {}px;", width)
    } else {
        "".to_string()
    };

    rsx! {
        div {
            class: "uikit-field-container {layout_class} {error_class} {extra_class}",
            if let Some(ref label_text) = label {
                label { 
                    class: "uikit-input-label", 
                    style: "{label_style}",
                    "{label_text}" 
                }
            }
            div {
                class: "uikit-field-content",
                {children}
                if let Some(ref err_msg) = error {
                    span { class: "uikit-input-err-text", "{err_msg}" }
                } else if let Some(ref help_msg) = help_text {
                    span { class: "uikit-input-help", "{help_msg}" }
                }
            }
        }
    }
}
