use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeadingLevel {
    H1,
    #[default]
    H2,
    H3,
    H4,
    H5,
    H6,
}

#[component]
pub fn Heading(
    #[props(default)] level: HeadingLevel,
    #[props(default)] bordered: bool,
    #[props(default)] muted: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] style: Option<String>,
    children: Element,
) -> Element {
    let base_class = "uikit-heading";
    let level_class = match level {
        HeadingLevel::H1 => "uikit-heading-h1",
        HeadingLevel::H2 => "uikit-heading-h2",
        HeadingLevel::H3 => "uikit-heading-h3",
        HeadingLevel::H4 => "uikit-heading-h4",
        HeadingLevel::H5 => "uikit-heading-h5",
        HeadingLevel::H6 => "uikit-heading-h6",
    };

    let bordered_class = if bordered {
        "uikit-heading-bordered"
    } else {
        ""
    };
    let muted_class = if muted { "uikit-heading-muted" } else { "" };
    let extra_class = class.unwrap_or_default();

    let class_name =
        format!("{base_class} {level_class} {bordered_class} {muted_class} {extra_class}");
    let style_attr = style.unwrap_or_default();

    match level {
        HeadingLevel::H1 => {
            rsx! { h1 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
        HeadingLevel::H2 => {
            rsx! { h2 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
        HeadingLevel::H3 => {
            rsx! { h3 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
        HeadingLevel::H4 => {
            rsx! { h4 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
        HeadingLevel::H5 => {
            rsx! { h5 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
        HeadingLevel::H6 => {
            rsx! { h6 { class: "{class_name}", style: "{style_attr}", {children} } }
        }
    }
}
