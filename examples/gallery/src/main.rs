use dioxus::prelude::*;
use ui_kit::*;

fn main() {
    dioxus::launch(App);
}

fn get_circle_connection(fx: f64, fy: f64, tx: f64, ty: f64, is_straight: bool) -> (f64, f64, f64, f64) {
    let dx = tx - fx;
    let dy = ty - fy;
    let dist = (dx * dx + dy * dy).sqrt();
    let from_radius = 35.0;
    // Marker reference points sit on their tips, so both path endpoints belong
    // exactly on the circle boundary—no arrow-length compensation is needed.
    let to_radius = 35.0;
    if is_straight {
        if dist > 0.01 {
            let nx = dx / dist;
            let ny = dy / dist;
            (fx + nx * from_radius, fy + ny * from_radius, tx - nx * to_radius, ty - ny * to_radius)
        } else {
            (fx, fy, tx, ty)
        }
    } else {
        (fx + from_radius, fy, tx - to_radius, ty)
    }
}

#[component]
fn App() -> Element {
    let mut theme_sig = use_signal(AppTheme::default);

    // Interactive states for components
    let mut btn_loading = use_signal(|| false);
    let mut input_val = use_signal(|| "".to_string());
    let mut input_err = use_signal(|| None::<String>);
    let mut checkbox_val = use_signal(|| false);
    let mut switch_val = use_signal(|| true);
    let mut select_val = use_signal(|| "rust".to_string());
    let mut modal_open = use_signal(|| false);
    let mut otp_val = use_signal(|| "".to_string());
    let mut slider_val = use_signal(|| 50.0);
    let mut datetime_val = use_signal(|| "2026-07-16 18:00".to_string());
    let mut wysiwyg_val = use_signal(|| "<p>Hello <b>World</b>! This is a <i>WYSIWYG</i> editor.</p><p>Double-click this text block to edit formatting.</p>".to_string());

    let mut flow_active = use_signal(|| Some("build".to_string()));
    let mut tree_active = use_signal(|| Some("root".to_string()));
    let mut net_active = use_signal(|| Some("db".to_string()));

    let flow_nodes = vec![
        (
            GraphNodeData { id: "checkout".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-success)".to_string()), border: None, background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-header",
                    span { class: "uikit-graph-node-title", "1. Checkout" }
                    span { class: "uikit-graph-node-status-indicator uikit-status-success" }
                }
                span { class: "uikit-graph-node-desc", "Source Code" }
            }
        ),
        (
            GraphNodeData { id: "lint".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-success)".to_string()), border: None, background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-header",
                    span { class: "uikit-graph-node-title", "2a. Lint & Format" }
                    span { class: "uikit-graph-node-status-indicator uikit-status-success" }
                }
                span { class: "uikit-graph-node-desc", "Cargo clippy" }
            }
        ),
        (
            GraphNodeData { id: "test".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-success)".to_string()), border: None, background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-header",
                    span { class: "uikit-graph-node-title", "2b. Unit Tests" }
                    span { class: "uikit-graph-node-status-indicator uikit-status-success" }
                }
                span { class: "uikit-graph-node-desc", "Cargo test" }
            }
        ),
        (
            GraphNodeData { id: "build".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-primary)".to_string()), border: None, background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-header",
                    span { class: "uikit-graph-node-title", "3. Build Artifact" }
                    span { class: "uikit-graph-node-status-indicator uikit-status-primary" }
                }
                span { class: "uikit-graph-node-desc", "Cargo build --release" }
            }
        ),
        (
            GraphNodeData { id: "deploy".to_string(), x: 0.0, y: 0.0, color: None, border: None, background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-header",
                    span { class: "uikit-graph-node-title", "4. Deploy" }
                    span { class: "uikit-graph-node-status-indicator uikit-status-primary" }
                }
                span { class: "uikit-graph-node-desc", "Deploy to AWS" }
            }
        ),
    ];
    let flow_edges = vec![
        GraphEdgeData { from: "checkout".to_string(), to: "lint".to_string(), label: None, edge_type: EdgeType::Orthogonal, color: Some("var(--uikit-success)".to_string()), animated: true, arrow: ArrowHead::End },
        GraphEdgeData { from: "checkout".to_string(), to: "test".to_string(), label: None, edge_type: EdgeType::Orthogonal, color: Some("var(--uikit-success)".to_string()), animated: true, arrow: ArrowHead::End },
        GraphEdgeData { from: "lint".to_string(), to: "build".to_string(), label: Some("Passed".to_string()), edge_type: EdgeType::Orthogonal, color: Some("var(--uikit-primary)".to_string()), animated: false, arrow: ArrowHead::End },
        GraphEdgeData { from: "test".to_string(), to: "build".to_string(), label: Some("Passed".to_string()), edge_type: EdgeType::Orthogonal, color: Some("var(--uikit-primary)".to_string()), animated: false, arrow: ArrowHead::End },
        GraphEdgeData { from: "build".to_string(), to: "deploy".to_string(), label: None, edge_type: EdgeType::Orthogonal, color: None, animated: false, arrow: ArrowHead::End },
    ];

    let tree_nodes = vec![
        (
            GraphNodeData { id: "root".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-primary)".to_string()), border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", style: "font-size: 20px; font-weight: 750; text-align: center;", "Core App" }
            }
        ),
        (
            GraphNodeData { id: "ui".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-info)".to_string()), border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", "UI Layer" }
            }
        ),
        (
            GraphNodeData { id: "db".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-warning)".to_string()), border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", "Database" }
            }
        ),
        (
            GraphNodeData { id: "api".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-success)".to_string()), border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", "GraphQL API" }
            }
        ),
        (
            GraphNodeData { id: "views".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Views & Pages" }
            }
        ),
        (
            GraphNodeData { id: "components".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: None, shape: NodeShape::Box, selected: false },
            rsx! {
                span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Shared Parts" }
            }
        ),
        (
            GraphNodeData { id: "web".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: Some("transparent".to_string()), shape: NodeShape::Plain, selected: false },
            rsx! { span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Web Views" } }
        ),
        (
            GraphNodeData { id: "mobile".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: Some("transparent".to_string()), shape: NodeShape::Plain, selected: false },
            rsx! { span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Mobile Views" } }
        ),
        (
            GraphNodeData { id: "forms".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: Some("transparent".to_string()), shape: NodeShape::Plain, selected: false },
            rsx! { span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Form Controls" } }
        ),
        (
            GraphNodeData { id: "navigation".to_string(), x: 0.0, y: 0.0, color: None, border: Some("none".to_string()), background_color: Some("transparent".to_string()), shape: NodeShape::Plain, selected: false },
            rsx! { span { class: "uikit-graph-node-title", style: "font-weight: 500; font-size: 13px;", "Navigation" } }
        ),
    ];
    let tree_edges = vec![
        GraphEdgeData { from: "root".to_string(), to: "ui".to_string(), label: None, edge_type: EdgeType::Bezier, color: Some("var(--uikit-info)".to_string()), animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "root".to_string(), to: "db".to_string(), label: None, edge_type: EdgeType::OrganicCurved, color: Some("var(--uikit-warning)".to_string()), animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "root".to_string(), to: "api".to_string(), label: None, edge_type: EdgeType::OrganicCurved, color: Some("var(--uikit-success)".to_string()), animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "ui".to_string(), to: "views".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "ui".to_string(), to: "components".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "views".to_string(), to: "web".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "views".to_string(), to: "mobile".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "components".to_string(), to: "forms".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
        GraphEdgeData { from: "components".to_string(), to: "navigation".to_string(), label: None, edge_type: EdgeType::Bezier, color: None, animated: false, arrow: ArrowHead::None },
    ];

    let net_nodes = vec![
        (
            GraphNodeData { id: "gateway".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-info)".to_string()), border: None, background_color: None, shape: NodeShape::Circle, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-title",
                    title: "Gateway",
                    "Gateway"
                }
            }
        ),
        (
            GraphNodeData { id: "auth".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-success)".to_string()), border: None, background_color: None, shape: NodeShape::Circle, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-title",
                    title: "Auth Svc",
                    "Auth Svc"
                }
            }
        ),
        (
            GraphNodeData { id: "payment".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-error)".to_string()), border: None, background_color: None, shape: NodeShape::Circle, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-title",
                    title: "Billing",
                    "Billing"
                }
            }
        ),
        (
            GraphNodeData { id: "db".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-warning)".to_string()), border: None, background_color: None, shape: NodeShape::Circle, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-title",
                    title: "Shared DB",
                    "Shared DB"
                }
            }
        ),
        (
            GraphNodeData { id: "cache".to_string(), x: 0.0, y: 0.0, color: Some("var(--uikit-primary)".to_string()), border: None, background_color: None, shape: NodeShape::Circle, selected: false },
            rsx! {
                div {
                    class: "uikit-graph-node-title",
                    title: "Redis",
                    "Redis"
                }
            }
        ),
    ];
    let net_edges = vec![
        GraphEdgeData { from: "gateway".to_string(), to: "auth".to_string(), label: Some("Active".to_string()), edge_type: EdgeType::Straight, color: Some("var(--uikit-info)".to_string()), animated: true, arrow: ArrowHead::Both },
        GraphEdgeData { from: "gateway".to_string(), to: "payment".to_string(), label: None, edge_type: EdgeType::Straight, color: Some("var(--uikit-error)".to_string()), animated: false, arrow: ArrowHead::End },
        GraphEdgeData { from: "auth".to_string(), to: "db".to_string(), label: None, edge_type: EdgeType::Straight, color: Some("var(--uikit-success)".to_string()), animated: false, arrow: ArrowHead::End },
        GraphEdgeData { from: "payment".to_string(), to: "db".to_string(), label: None, edge_type: EdgeType::Straight, color: Some("var(--uikit-warning)".to_string()), animated: false, arrow: ArrowHead::End },
        GraphEdgeData { from: "db".to_string(), to: "cache".to_string(), label: Some("Sync".to_string()), edge_type: EdgeType::Straight, color: Some("var(--uikit-primary)".to_string()), animated: true, arrow: ArrowHead::Both },
        GraphEdgeData { from: "gateway".to_string(), to: "cache".to_string(), label: None, edge_type: EdgeType::Straight, color: None, animated: false, arrow: ArrowHead::End },
    ];

    let select_options = vec![
        ("rust".to_string(), "Rust".to_string()),
        ("typescript".to_string(), "TypeScript".to_string()),
        ("python".to_string(), "Python".to_string()),
    ];

    // Validate input reactively
    use_effect(move || {
        let val = input_val.read();
        if val.is_empty() {
            input_err.set(None);
        } else if val.len() < 3 {
            input_err.set(Some("Must be at least 3 characters".to_string()));
        } else {
            input_err.set(None);
        }
    });

    rsx! {
        ThemeProvider { theme: theme_sig,
            div {
                style: "max-width: 1000px; margin: 0 auto; padding: 40px 20px; display: flex; flex-direction: column; gap: 40px;",

                // Header with title and theme selector
                header {
                    style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--uikit-border); padding-bottom: 20px; flex-wrap: wrap; gap: 20px;",
                    div {
                        Heading { level: HeadingLevel::H1, "Dioxus Component Gallery" }
                        p { style: "margin: 8px 0 0 0; color: var(--uikit-muted); font-size: 14px;", "A premium collection of reusable and highly customizable components." }
                    }
                    div {
                        style: "max-width: 260px; flex-grow: 1;",
                        Select {
                            value: theme_sig.read().class_name().to_string(),
                            onchange: move |val: String| {
                                if val == "uikit-theme-neutral" {
                                    theme_sig.set(AppTheme::Neutral);
                                } else if val == "uikit-theme-black" {
                                    theme_sig.set(AppTheme::Black);
                                } else if val == "uikit-theme-white" {
                                    theme_sig.set(AppTheme::White);
                                }
                            },
                            options: vec![
                                ("uikit-theme-neutral".to_string(), "Neutral".to_string()),
                                ("uikit-theme-black".to_string(), "Black".to_string()),
                                ("uikit-theme-white".to_string(), "White".to_string()),
                            ],
                            label: "Select Theme",
                            label_layout: LabelLayout::Top
                        }
                    }
                }

                // Main gallery content
                main {
                    style: "display: flex; flex-direction: column; gap: 40px;",

                    // Buttons Section
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "1. Buttons" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px;",
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "Variants" }
                                    div {
                                        style: "display: flex; gap: 12px; flex-wrap: wrap;",
                                        Button { variant: ButtonVariant::Primary, "Primary" }
                                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                                        Button { variant: ButtonVariant::Outline, "Outline" }
                                        Button { variant: ButtonVariant::Text, "Text Button" }
                                    }
                                }
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "Sizes" }
                                    div {
                                        style: "display: flex; align-items: center; gap: 12px; flex-wrap: wrap;",
                                        Button { size: ButtonSize::Small, "Small" }
                                        Button { size: ButtonSize::Medium, "Medium" }
                                        Button { size: ButtonSize::Large, "Large" }
                                    }
                                }
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "States" }
                                    div {
                                        style: "display: flex; gap: 12px; flex-wrap: wrap; align-items: center;",
                                        Button { disabled: true, "Disabled" }
                                        Button { loading: true, "Loading" }
                                        Button {
                                            loading: *btn_loading.read(),
                                            onclick: move |_| {
                                                btn_loading.set(true);
                                            },
                                            "Click to Load"
                                        }
                                        if *btn_loading.read() {
                                            Button {
                                                variant: ButtonVariant::Text,
                                                onclick: move |_| {
                                                    btn_loading.set(false);
                                                },
                                                "Reset"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Form Controls Section
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "2. Form Controls" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px; max-width: 500px;",
                                div {
                                    TextInput {
                                        value: input_val.read().clone(),
                                        oninput: move |e: FormEvent| input_val.set(e.value()),
                                        label: "Username",
                                        label_layout: LabelLayout::Left,
                                        alignment: 140.0,
                                        placeholder: "Enter username...",
                                        error: input_err.read().clone(),
                                        help_text: "Must be at least 3 characters long.",
                                    }
                                }
                                div {
                                    TextInput {
                                        value: "user@example.com".to_string(),
                                        oninput: move |_| {},
                                        label: "Email",
                                        label_layout: LabelLayout::Left,
                                        alignment: 140.0,
                                        placeholder: "Enter email...",
                                    }
                                }
                                div {
                                    Select {
                                        value: select_val.read().clone(),
                                        onchange: move |val| select_val.set(val),
                                        options: select_options.clone(),
                                        label: "Preferred Language",
                                        label_layout: LabelLayout::Left,
                                        alignment: 140.0,
                                    }
                                }
                                div {
                                    style: "display: flex; gap: 24px; flex-wrap: wrap;",
                                    Checkbox {
                                        checked: *checkbox_val.read(),
                                        onchange: move |val| checkbox_val.set(val),
                                        label: "Accept terms and conditions"
                                    }
                                    Checkbox {
                                        checked: true,
                                        onchange: move |_| {},
                                        disabled: true,
                                        label: "Disabled Checkbox (Checked)"
                                    }
                                }
                                div {
                                    style: "display: flex; gap: 24px; flex-wrap: wrap;",
                                    Switch {
                                        checked: *switch_val.read(),
                                        onchange: move |val| switch_val.set(val),
                                        label: "Enable notifications"
                                    }
                                    Switch {
                                        checked: false,
                                        onchange: move |_| {},
                                        disabled: true,
                                        label: "Disabled Switch"
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px; border-top: 1px dashed var(--uikit-border); padding-top: 16px;",
                                    OtpInput {
                                        value: otp_val.read().clone(),
                                        onchange: move |val: String| otp_val.set(val),
                                        length: 6,
                                        label: "One-Time Password (OTP)",
                                        label_layout: LabelLayout::Top,
                                        help_text: format!("Current value in parent state: '{}'", otp_val.read()),
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px; border-top: 1px dashed var(--uikit-border); padding-top: 16px;",
                                    span { class: "uikit-input-label", "Slider (Value: {slider_val.read():.1})" }
                                    Slider {
                                        value: *slider_val.read(),
                                        min: 0.0,
                                        max: 100.0,
                                        step: 0.5,
                                        on_change: move |val| slider_val.set(val),
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px;",
                                    span { class: "uikit-input-label", "Disabled Slider" }
                                    Slider {
                                        value: 30.0,
                                        min: 0.0,
                                        max: 100.0,
                                        disabled: true,
                                        on_change: move |_| {},
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px; border-top: 1px dashed var(--uikit-border); padding-top: 16px;",
                                    DateTimePicker {
                                        value: datetime_val.read().clone(),
                                        on_change: move |val| datetime_val.set(val),
                                        label: "Appointment Date & Time",
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px;",
                                    DateTimePicker {
                                        value: "2026-07-16 14:00".to_string(),
                                        on_change: move |_| {},
                                        label: "Disabled Date & Time Picker",
                                        disabled: true,
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px; border-top: 1px dashed var(--uikit-border); padding-top: 16px;",
                                    EditableText {
                                        value: wysiwyg_val.read().clone(),
                                        onchange: move |val| wysiwyg_val.set(val),
                                        label: "WYSIWYG Rich Text Editor (Double-click to edit)",
                                        label_layout: LabelLayout::Top,
                                        help_text: "Supports formatting (Bold, Italic, Underline, Strikethrough), Paragraph/Headings, Lists, Alignments. Ctrl+Enter to save, Esc to cancel.",
                                    }
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 8px; border-top: 1px dashed var(--uikit-border); padding-top: 16px;",
                                    span { class: "uikit-input-label", "Live HTML Output" }
                                    pre {
                                        style: "padding: 10px; background-color: var(--uikit-muted-bg); border: 1px solid var(--uikit-border); border-radius: var(--uikit-radius-md); font-family: monospace; font-size: 12px; overflow-x: auto; white-space: pre-wrap; margin: 0; color: var(--uikit-muted);",
                                        "{wysiwyg_val.read()}"
                                    }
                                }
                            }
                        }
                    }

                    // Display Components
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "3. Feedback & Badges" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px;",
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "Badges (Variants, Sizes & Styles)" }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 16px;",
                                        div {
                                            style: "display: flex; gap: 10px; flex-wrap: wrap; align-items: center;",
                                            span { style: "font-size: 13px; color: var(--uikit-muted); margin-right: 8px;", "Normal:" }
                                            Badge { variant: BadgeVariant::Default, "Default" }
                                            Badge { variant: BadgeVariant::Success, "Success" }
                                            Badge { variant: BadgeVariant::Warning, "Warning" }
                                            Badge { variant: BadgeVariant::Error, "Error" }
                                            Badge { variant: BadgeVariant::Info, "Info" }
                                        }
                                        div {
                                            style: "display: flex; gap: 10px; flex-wrap: wrap; align-items: center;",
                                            span { style: "font-size: 13px; color: var(--uikit-muted); margin-right: 8px;", "Large & Borderless (Metrics Style):" }
                                            Badge { variant: BadgeVariant::Default, size: BadgeSize::Large, borderless: true, "Default" }
                                            Badge { variant: BadgeVariant::Success, size: BadgeSize::Large, borderless: true, "Success" }
                                            Badge { variant: BadgeVariant::Warning, size: BadgeSize::Large, borderless: true, "Warning" }
                                            Badge { variant: BadgeVariant::Error, size: BadgeSize::Large, borderless: true, "Error" }
                                            Badge { variant: BadgeVariant::Info, size: BadgeSize::Large, borderless: true, "Info" }
                                        }
                                    }
                                }
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "Notifications" }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 12px;",
                                        Notification {
                                            variant: NotificationVariant::Info,
                                            title: "System Update",
                                            "A new software update is available. Please upgrade."
                                        }
                                        Notification {
                                            variant: NotificationVariant::Success,
                                            title: "Operation Successful",
                                            "Your settings have been saved correctly."
                                        }
                                        Notification {
                                            variant: NotificationVariant::Warning,
                                            title: "Low Disk Space",
                                            "Your storage is almost full. Clean some space."
                                        }
                                        Notification {
                                            variant: NotificationVariant::Error,
                                            title: "Connection Failed",
                                            "Unable to connect to the database. Please try again."
                                        }
                                    }
                                }
                                div {
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 12px;", "Spinners (Sizes & Variants)" }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 20px;",
                                        div {
                                            style: "display: flex; gap: 24px; flex-wrap: wrap; align-items: center;",
                                            span { style: "font-size: 13px; color: var(--uikit-muted); min-width: 80px;", "Sizes:" }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { size: SpinnerSize::Small }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Small" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { size: SpinnerSize::Medium }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Medium" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { size: SpinnerSize::Large }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Large" } }
                                        }
                                        div {
                                            style: "display: flex; gap: 24px; flex-wrap: wrap; align-items: center;",
                                            span { style: "font-size: 13px; color: var(--uikit-muted); min-width: 80px;", "Variants:" }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Primary }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Primary" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Secondary }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Secondary" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Success }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Success" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Warning }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Warning" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Error }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Error" } }
                                            div { style: "display: flex; align-items: center; gap: 8px;", Spinner { variant: SpinnerVariant::Info }, span { style: "font-size: 13px; color: var(--uikit-muted);", "Info" } }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Overlay / Interactive Modal
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "4. Modals & Dialogs" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 12px; align-items: flex-start;",
                                p { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "Click below to trigger a dialog box with background blur and close interactions." }
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| modal_open.set(true),
                                    "Open Interactive Modal"
                                }
                            }
                        }
                    }

                    // Metric & Data Visualization Section
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "5. Metric & Data Visualization" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 32px;",

                                // Metric Cards Grid
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    Heading { level: HeadingLevel::H4, muted: true, style: "margin-bottom: 4px;", "Metric Cards (Directly using Card)" }
                                    div {
                                        style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 20px;",

                                        // Counter Card
                                        Card {
                                            shadowed: true,
                                            hoverable: true,
                                            header: rsx! {
                                                div { style: "display: flex; justify-content: space-between; align-items: center; width: 100%;",
                                                    span { class: "uikit-metric-label", "Requests Count" }
                                                }
                                            },
                                            Unit { value: "1,234", unit: "reqs" }
                                        }

                                        // Gauge Card
                                        Card {
                                            shadowed: true,
                                            hoverable: true,
                                            header: rsx! {
                                                div { style: "display: flex; justify-content: space-between; align-items: center; width: 100%;",
                                                    span { class: "uikit-metric-label", "CPU Usage" }
                                                }
                                            },
                                            div { style: "display: flex; flex-direction: column; gap: 8px;",
                                                Unit { value: "75.4", unit: "%" }
                                                Gauge {
                                                    value: 75.4,
                                                    min_label: "0.0",
                                                    max_label: "100.0"
                                                }
                                            }
                                        }

                                        // Status Card
                                        Card {
                                            shadowed: true,
                                            hoverable: true,
                                            header: rsx! {
                                                div { style: "display: flex; justify-content: space-between; align-items: center; width: 100%;",
                                                    span { class: "uikit-metric-label", "System Status" }
                                                }
                                            },
                                            Badge {
                                                variant: BadgeVariant::Success,
                                                size: BadgeSize::Large,
                                                borderless: true,
                                                "Healthy"
                                            }
                                        }

                                        // TimeSeries Card
                                        Card {
                                            shadowed: true,
                                            hoverable: true,
                                            header: rsx! {
                                                div { style: "display: flex; justify-content: space-between; align-items: center; width: 100%;",
                                                    span { class: "uikit-metric-label", "Memory Load" }
                                                }
                                            },
                                            div { style: "display: flex; flex-direction: column; gap: 12px;",
                                                Unit { value: "4.2", unit: "GB" }
                                                Sparkline {
                                                    data: vec![1.2, 1.5, 2.0, 1.8, 2.4, 3.1, 2.8, 3.5, 4.2],
                                                    fill: true
                                                }
                                            }
                                        }
                                    }
                                }

                                // Raw Progress Bars Demo
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    Heading { level: HeadingLevel::H4, muted: true, "Progress Bars" }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 16px; max-width: 400px;",
                                        ProgressBar { value: 30.0 }
                                        ProgressBar {
                                            value: 65.0,
                                            min_label: "Start",
                                            max_label: "Goal"
                                        }
                                    }
                                }

                                // Raw Sparklines Demo
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    Heading { level: HeadingLevel::H4, muted: true, "Sparklines (Trend Lines)" }
                                    div {
                                        style: "display: flex; gap: 24px; flex-wrap: wrap;",
                                        div {
                                            style: "flex: 1; min-width: 200px;",
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Default Sparkline" }
                                            Sparkline { data: vec![10.0, 15.0, 8.0, 25.0, 18.0, 30.0] }
                                        }
                                        div {
                                            style: "flex: 1; min-width: 200px;",
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Filled Sparkline" }
                                            Sparkline { data: vec![30.0, 25.0, 40.0, 35.0, 50.0, 45.0, 60.0], fill: true }
                                        }
                                    }
                                }

                                // Raw Units Demo
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    Heading { level: HeadingLevel::H4, muted: true, "Units (Reusable Values)" }
                                    div {
                                        style: "display: flex; gap: 40px; flex-wrap: wrap; align-items: center;",
                                        div {
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Small size (14px)" }
                                            Unit { value: "1,245", unit: "reqs", size: UnitSize::Small }
                                        }
                                        div {
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Medium size (20px)" }
                                            Unit { value: "75.4", unit: "%", size: UnitSize::Medium }
                                        }
                                        div {
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Large size (28px)" }
                                            Unit { value: "4.2", unit: "GB", size: UnitSize::Large }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 9. Graph Components Section
                    section {
                        style: "display: flex; flex-direction: column; gap: 20px;",
                        Heading { level: HeadingLevel::H2, bordered: true, "9. Graph Components" }
                        p {
                            style: "color: var(--uikit-muted); font-size: 14px; margin-top: -8px;",
                            "Interactive graph visualization components utilizing custom CSS grids, animated SVGs, and level-based auto-layout. Click nodes to select/focus."
                        }

                        div {
                            style: "display: flex; flex-direction: column; gap: 32px;",

                            // FlowGraph Showcase
                            Card {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    Heading { level: HeadingLevel::H3, "Flow Graph (CI/CD Pipeline)" }
                                    p { style: "font-size: 13px; color: var(--uikit-muted);", "orthogonal flow lines, automated stage sorting, and glowing animated active edges." }
                                    FlowGraph {
                                        nodes: flow_nodes,
                                        edges: flow_edges,
                                        active_node_id: flow_active.read().clone(),
                                        on_node_click: move |id| flow_active.set(Some(id))
                                    }
                                }
                            }

                            // HierarchyGraph Showcase
                            Card {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    Heading { level: HeadingLevel::H3, "Hierarchy Graph (Mind Map / Org Structure)" }
                                    p { style: "font-size: 13px; color: var(--uikit-muted);", "XMind-style two-sided topics with smooth branches and configurable canvas and node surfaces." }
                                    HierarchyGraph {
                                        nodes: tree_nodes,
                                        edges: tree_edges,
                                        root_edge_type: Some(EdgeType::OrganicCurved),
                                        border: Some("none".to_string()),
                                        background_color: Some("var(--uikit-bg)".to_string()),
                                        active_node_id: tree_active.read().clone(),
                                        on_node_click: move |id| tree_active.set(Some(id))
                                    }
                                }
                            }

                            // NetworkGraph Showcase
                            Card {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    Heading { level: HeadingLevel::H3, "Network Graph (Mesh Topology)" }
                                    p { style: "font-size: 13px; color: var(--uikit-muted);", "Hub-and-spoke automatic circular projection with direct straight mesh links." }
                                    NetworkGraph {
                                        nodes: net_nodes,
                                        edges: net_edges,
                                        active_node_id: net_active.read().clone(),
                                        on_node_click: move |id| net_active.set(Some(id))
                                    }
                                }
                            }

                            // Node & Edge Variants Showcase
                            Card {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 24px;",
                                    Heading { level: HeadingLevel::H3, "Node & Edge Shapes Playground" }
                                    p { style: "font-size: 13px; color: var(--uikit-muted); margin-top: -8px;", "A playground showing all Node shapes (with default and colored styling) and Edge routing shapes." }

                                    // Node Shapes Showcase
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 12px;",
                                        Heading { level: HeadingLevel::H4, muted: true, "Node Shapes & Colors" }
                                        div {
                                            style: "width: 100%; overflow: auto; display: flex; align-items: center; justify-content: center; padding: 12px;",
                                            div {
                                                class: "uikit-graph-container uikit-graph-grid",
                                                style: "position: relative; width: 900px; height: 180px; flex-shrink: 0;",
                                                
                                                // Default Shapes
                                                Node { id: "n-def-p".to_string(), x: 120.0, y: 50.0, color: None, shape: NodeShape::Pill, span { "Default Pill" } }
                                                Node { id: "n-def-b".to_string(), x: 300.0, y: 50.0, color: None, shape: NodeShape::Box, div { style: "text-align: center;", span { style: "font-weight: 600;", "Default Box" } } }
                                                Node { id: "n-def-pl".to_string(), x: 480.0, y: 50.0, color: None, shape: NodeShape::Plain, span { style: "font-size: 13px;", "Default Plain" } }
                                                Node { id: "n-def-c".to_string(), x: 640.0, y: 50.0, color: None, shape: NodeShape::Circle, div { style: "text-align: center; font-size: 11px;", "Default Circle" } }

                                                // Colored Shapes (using different custom colors)
                                                Node { id: "n-pri-p".to_string(), x: 120.0, y: 130.0, color: Some("var(--uikit-success)".to_string()), shape: NodeShape::Pill, span { "Success Pill" } }
                                                Node { id: "n-pri-b".to_string(), x: 300.0, y: 130.0, color: Some("var(--uikit-warning)".to_string()), shape: NodeShape::Box, div { style: "text-align: center;", span { style: "font-weight: 600;", "Warning Box" } } }
                                                Node { id: "n-pri-c".to_string(), x: 640.0, y: 130.0, color: Some("var(--uikit-error)".to_string()), shape: NodeShape::Circle, div { style: "text-align: center; font-size: 11px;", "Error Circle" } }
                                            }
                                        }
                                    }

                                    // Edge Shapes Preview
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 12px;",
                                        Heading { level: HeadingLevel::H4, muted: true, "Edge Routing Shapes (Straight, Bezier, Orthogonal, Curved Orthogonal, Organic Curved)" }
                                        {
                                            let (s1_x, s1_y, e1_x, e1_y) = get_circle_connection(50.0, 50.0, 200.0, 150.0, true);
                                            let (s2_x, s2_y, e2_x, e2_y) = get_circle_connection(270.0, 50.0, 420.0, 150.0, false);
                                            let (s3_x, s3_y, e3_x, e3_y) = get_circle_connection(490.0, 50.0, 640.0, 150.0, false);
                                            let (s4_x, s4_y, e4_x, e4_y) = get_circle_connection(710.0, 50.0, 860.0, 150.0, false);
                                            
                                            rsx! {
                                                div {
                                                    style: "width: 100%; overflow: auto; display: flex; align-items: center; justify-content: center; padding: 12px;",
                                                    div {
                                                        class: "uikit-graph-container uikit-graph-grid",
                                                        style: "position: relative; width: 1120px; height: 210px; flex-shrink: 0;",
                                                        svg {
                                                            class: "uikit-graph-svg",
                                                            style: "width: 100%; height: 100%; pointer-events: none;",
                                                            view_box: "0 0 1120 210",
                                                            EdgeDefs {}
                                                            
                                                            // 1. Straight Edge
                                                            Edge { from_x: s1_x, from_y: s1_y, to_x: e1_x, to_y: e1_y, edge_type: EdgeType::Straight, arrow: ArrowHead::End, label: Some("Straight".to_string()), color: Some("var(--uikit-success)".to_string()) }
                                                            
                                                            // 2. Bezier Edge
                                                            Edge { from_x: s2_x, from_y: s2_y, to_x: e2_x, to_y: e2_y, edge_type: EdgeType::Bezier, arrow: ArrowHead::End, label: Some("Bezier".to_string()), color: Some("var(--uikit-info)".to_string()) }
                                                            
                                                            // 3. Orthogonal Edge
                                                            Edge { from_x: s3_x, from_y: s3_y, to_x: e3_x, to_y: e3_y, edge_type: EdgeType::Orthogonal, arrow: ArrowHead::End, label: Some("Orthogonal".to_string()), color: Some("var(--uikit-warning)".to_string()) }
                                                            
                                                            // 4. Curved Orthogonal Edge
                                                            Edge { from_x: s4_x, from_y: s4_y, to_x: e4_x, to_y: e4_y, edge_type: EdgeType::CurvedOrthogonal, arrow: ArrowHead::End, label: Some("Curved Ortho".to_string()), color: Some("var(--uikit-error)".to_string()) }

                                                            // 5. Long boundary-anchored XMind curve
                                                            Edge { from_x: 970.0, from_y: 85.0, to_x: 1035.0, to_y: 150.0, from_normal: Some((0.0, 1.0)), to_normal: Some((-1.0, 0.0)), edge_type: EdgeType::OrganicCurved, arrow: ArrowHead::End, label: Some("Organic Curved".to_string()), color: Some("var(--uikit-primary)".to_string()) }
                                                        }
                                                        
                                                        // Labeled Endpoint Nodes
                                                        Node { id: "ep-s1".to_string(), x: 50.0, y: 50.0, color: None, shape: NodeShape::Circle, span { "Start" } }
                                                        Node { id: "ep-e1".to_string(), x: 200.0, y: 150.0, color: Some("var(--uikit-success)".to_string()), shape: NodeShape::Circle, span { "End" } }
                                                        
                                                        Node { id: "ep-s2".to_string(), x: 270.0, y: 50.0, color: None, shape: NodeShape::Circle, span { "Start" } }
                                                        Node { id: "ep-e2".to_string(), x: 420.0, y: 150.0, color: Some("var(--uikit-info)".to_string()), shape: NodeShape::Circle, span { "End" } }
                                                        
                                                        Node { id: "ep-s3".to_string(), x: 490.0, y: 50.0, color: None, shape: NodeShape::Circle, span { "Start" } }
                                                        Node { id: "ep-e3".to_string(), x: 640.0, y: 150.0, color: Some("var(--uikit-warning)".to_string()), shape: NodeShape::Circle, span { "End" } }
                                                        
                                                        Node { id: "ep-s4".to_string(), x: 710.0, y: 50.0, color: None, shape: NodeShape::Circle, span { "Start" } }
                                                        Node { id: "ep-e4".to_string(), x: 860.0, y: 150.0, color: Some("var(--uikit-error)".to_string()), shape: NodeShape::Circle, span { "End" } }

                                                        Node { id: "ep-s5".to_string(), x: 970.0, y: 50.0, color: None, shape: NodeShape::Circle, span { "Start" } }
                                                        Node { id: "ep-e5".to_string(), x: 1070.0, y: 150.0, color: Some("var(--uikit-primary)".to_string()), shape: NodeShape::Circle, span { "End" } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Modal Component
            Modal {
                open: *modal_open.read(),
                onclose: move |_| modal_open.set(false),
                title: "Confirm Action",
                footer: rsx! {
                    Button {
                        variant: ButtonVariant::Outline,
                        onclick: move |_| modal_open.set(false),
                        "Cancel"
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| modal_open.set(false),
                        "Confirm"
                    }
                },
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",
                    p { "Are you sure you want to perform this action? This operation will affect the active configuration." }
                    Notification {
                        variant: NotificationVariant::Warning,
                        "This operation cannot be undone."
                    }
                }
            }
        }
    }
}
