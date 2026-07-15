use dioxus::prelude::*;
use ui_kit::*;

fn main() {
    dioxus::launch(App);
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
                        h1 { style: "margin: 0; font-size: 28px; font-weight: 700;", "Dioxus Component Gallery" }
                        p { style: "margin: 8px 0 0 0; color: var(--uikit-muted); font-size: 14px;", "A premium collection of reusable and highly customizable components." }
                    }
                    div {
                        style: "display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 14px; font-weight: 500; white-space: nowrap;", "Select Theme:" }
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
                            ]
                        }
                    }
                }

                // Main gallery content
                main {
                    style: "display: flex; flex-direction: column; gap: 40px;",

                    // Buttons Section
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        h2 { style: "margin: 0; font-size: 20px; border-left: 4px solid var(--uikit-primary); padding-left: 12px;", "1. Buttons" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px;",
                                div {
                                    h4 { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "Variants" }
                                    div {
                                        style: "display: flex; gap: 12px; flex-wrap: wrap;",
                                        Button { variant: ButtonVariant::Primary, "Primary" }
                                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                                        Button { variant: ButtonVariant::Outline, "Outline" }
                                        Button { variant: ButtonVariant::Text, "Text Button" }
                                    }
                                }
                                div {
                                    h4 { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "Sizes" }
                                    div {
                                        style: "display: flex; align-items: center; gap: 12px; flex-wrap: wrap;",
                                        Button { size: ButtonSize::Small, "Small" }
                                        Button { size: ButtonSize::Medium, "Medium" }
                                        Button { size: ButtonSize::Large, "Large" }
                                    }
                                }
                                div {
                                    h4 { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "States" }
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
                        h2 { style: "margin: 0; font-size: 20px; border-left: 4px solid var(--uikit-primary); padding-left: 12px;", "2. Form Controls" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px; max-width: 500px;",
                                div {
                                    Input {
                                        value: input_val.read().clone(),
                                        oninput: move |e: FormEvent| input_val.set(e.value()),
                                        label: "Username",
                                        placeholder: "Enter username...",
                                        error: input_err.read().clone(),
                                        help_text: "Must be at least 3 characters long."
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
                                    Select {
                                        value: select_val.read().clone(),
                                        onchange: move |val| select_val.set(val),
                                        options: select_options.clone(),
                                        label: "Preferred Language"
                                    }
                                }
                            }
                        }
                    }

                    // Display Components
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        h2 { style: "margin: 0; font-size: 20px; border-left: 4px solid var(--uikit-primary); padding-left: 12px;", "3. Feedback & Badges" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 24px;",
                                div {
                                    h4 { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "Badges (Variants, Sizes & Styles)" }
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
                                    h4 { style: "margin: 0 0 12px 0; color: var(--uikit-muted);", "Notifications" }
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
                            }
                        }
                    }

                    // Overlay / Interactive Modal
                    section {
                        style: "display: flex; flex-direction: column; gap: 16px;",
                        h2 { style: "margin: 0; font-size: 20px; border-left: 4px solid var(--uikit-primary); padding-left: 12px;", "4. Modals & Dialogs" }
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
                        h2 { style: "margin: 0; font-size: 20px; border-left: 4px solid var(--uikit-primary); padding-left: 12px;", "5. Metric & Data Visualization" }
                        Card {
                            div {
                                style: "display: flex; flex-direction: column; gap: 32px;",
                                
                                // Metric Cards Grid
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    h4 { style: "margin: 0 0 4px 0; color: var(--uikit-muted);", "Metric Cards (Directly using Card)" }
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
                                            div { class: "uikit-metric-value-container",
                                                Unit { value: "1,234", unit: "reqs" }
                                            }
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
                                                div { class: "uikit-metric-value-container",
                                                    Unit { value: "75.4", unit: "%" }
                                                }
                                                ProgressBar {
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
                                                    Badge {
                                                        variant: BadgeVariant::Success,
                                                        size: BadgeSize::Large,
                                                        borderless: true,
                                                        "Healthy"
                                                    }
                                                }
                                            },
                                            div { class: "uikit-metric-value-container",
                                                Unit { value: "Operational" }
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
                                                div { class: "uikit-metric-value-container",
                                                    Unit { value: "4.2", unit: "GB" }
                                                }
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
                                    h4 { style: "margin: 0; color: var(--uikit-muted);", "Progress Bars" }
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
                                    h4 { style: "margin: 0; color: var(--uikit-muted);", "Sparklines (Trend Lines)" }
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
                                    h4 { style: "margin: 0; color: var(--uikit-muted);", "Units (Reusable Values)" }
                                    div {
                                        style: "display: flex; gap: 40px; flex-wrap: wrap; align-items: center;",
                                        div {
                                            style: "font-size: 16px;",
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Small size (16px)" }
                                            Unit { value: "1,245", unit: "reqs" }
                                        }
                                        div {
                                            style: "font-size: 24px;",
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Medium size (24px)" }
                                            Unit { value: "75.4", unit: "%" }
                                        }
                                        div {
                                            style: "font-size: 36px;",
                                            span { style: "font-size: 12px; color: var(--uikit-muted); display: block; margin-bottom: 8px;", "Large size (36px)" }
                                            Unit { value: "4.2", unit: "GB" }
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
