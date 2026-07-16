use dioxus::prelude::*;
use dioxus::document::eval;
use super::{FormField, LabelLayout};

#[component]
pub fn OtpInput(
    #[props(into)] value: String,
    onchange: EventHandler<String>,
    #[props(default = 6)] length: usize,
    #[props(into, default)] label: Option<String>,
    #[props(default)] label_layout: LabelLayout,
    #[props(into, default)] alignment: Option<f32>,
    #[props(into, default)] error: Option<String>,
    #[props(into, default)] help_text: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    // Set up a listener for pasting code
    let mut paste_eval = use_signal(|| {
        eval(r#"
            document.addEventListener('paste', (e) => {
                let active = document.activeElement;
                if (active && active.classList.contains('uikit-otp-input-field')) {
                    e.preventDefault();
                    let text = (e.clipboardData || window.clipboardData).getData('text');
                    let digits = text.replace(/\D/g, '');
                    dioxus.send(digits);
                }
            });
        "#)
    });

    let onchange_clone = onchange.clone();
    use_future(move || async move {
        while let Ok(text) = paste_eval.write().recv::<String>().await {
            let limited: String = text.chars().take(length).collect();
            onchange_clone.call(limited.clone());
            let focus_idx = limited.len().min(length - 1);
            let _ = eval(&format!(
                r#"
                let el = document.getElementById("otp-input-{}");
                if (el) {{
                    el.focus();
                    el.select();
                }}
                "#,
                focus_idx
            ));
        }
    });

    // Parse the value into individual characters
    let val_chars: Vec<char> = value.chars().collect();

    rsx! {
        FormField {
            label: label,
            layout: label_layout,
            alignment: alignment,
            error: error,
            help_text: help_text,
            div {
                class: "uikit-otp-input-container",
                style: "display: flex; gap: 8px; align-items: center;",
                for i in 0..length {
                    {
                        let char_val = val_chars.get(i).copied().unwrap_or(' ');
                        let display_val = if char_val == ' ' { "".to_string() } else { char_val.to_string() };
                        let val_chars_onkeydown = val_chars.clone();
                        let val_chars_oninput = val_chars.clone();
                        rsx! {
                            input {
                                id: "otp-input-{i}",
                                class: "uikit-input uikit-otp-input-field",
                                r#type: "text",
                                inputmode: "numeric",
                                pattern: "[0-9]*",
                                value: "{display_val}",
                                disabled: disabled,
                                style: "width: 45px; height: 45px; text-align: center; font-size: 1.25rem; font-weight: 600; padding: 0;",
                                
                                onfocus: move |_| {
                                    let _ = eval(&format!(
                                        r#"
                                        let el = document.getElementById("otp-input-{}");
                                        if (el) el.select();
                                        "#,
                                        i
                                    ));
                                },

                                onkeydown: move |e: KeyboardEvent| {
                                    let key_str = e.key().to_string();
                                    match key_str.as_str() {
                                        "Backspace" => {
                                            let mut current_chars = val_chars_onkeydown.clone();
                                            while current_chars.len() < length {
                                                current_chars.push(' ');
                                            }
                                            if i < current_chars.len() && current_chars[i] != ' ' {
                                                current_chars[i] = ' ';
                                                let new_otp: String = current_chars.iter().filter(|&&c| c != ' ').collect();
                                                onchange.call(new_otp);
                                            } else if i > 0 {
                                                current_chars[i - 1] = ' ';
                                                let new_otp: String = current_chars.iter().filter(|&&c| c != ' ').collect();
                                                onchange.call(new_otp);
                                                let _ = eval(&format!(
                                                    r#"
                                                    let el = document.getElementById("otp-input-{}");
                                                    if (el) {{
                                                        el.focus();
                                                        el.select();
                                                    }}
                                                    "#,
                                                    i - 1
                                                ));
                                            }
                                        }
                                        "ArrowLeft" | "Left" => {
                                            if i > 0 {
                                                let _ = eval(&format!(
                                                    r#"
                                                    let el = document.getElementById("otp-input-{}");
                                                    if (el) {{
                                                        el.focus();
                                                        el.select();
                                                    }}
                                                    "#,
                                                    i - 1
                                                ));
                                            }
                                        }
                                        "ArrowRight" | "Right" => {
                                            if i + 1 < length {
                                                let _ = eval(&format!(
                                                    r#"
                                                    let el = document.getElementById("otp-input-{}");
                                                    if (el) {{
                                                        el.focus();
                                                        el.select();
                                                    }}
                                                    "#,
                                                    i + 1
                                                ));
                                            }
                                        }
                                        _ => {}
                                    }
                                },

                                oninput: move |e: FormEvent| {
                                    let input_val = e.value();
                                    let mut current_chars = val_chars_oninput.clone();
                                    while current_chars.len() < length {
                                        current_chars.push(' ');
                                    }
                                    
                                    if let Some(c) = input_val.chars().next_back() {
                                        if c.is_ascii_digit() {
                                            current_chars[i] = c;
                                            let new_otp: String = current_chars.iter().filter(|&&c| c != ' ').collect();
                                            onchange.call(new_otp);
                                            
                                            if i + 1 < length {
                                                let _ = eval(&format!(
                                                    r#"
                                                    let el = document.getElementById("otp-input-{}");
                                                    if (el) {{
                                                        el.focus();
                                                        el.select();
                                                    }}
                                                    "#,
                                                    i + 1
                                                ));
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
}
