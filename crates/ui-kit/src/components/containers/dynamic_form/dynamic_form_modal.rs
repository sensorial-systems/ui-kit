use super::form_flow_engine::FormFlowEngine;
use super::question::Question;
use super::question_answer::QuestionAnswer;
use super::question_type::QuestionType;
use crate::components::info::{Badge, BadgeSize, BadgeVariant, ProgressBar};
use crate::components::input::{Button, ButtonSize, ButtonVariant, SelectableButton, TextInput};
use dioxus::prelude::*;

#[component]
pub fn DynamicFormModal(
    open: bool,
    onclose: EventHandler<()>,
    onsubmit: EventHandler<String>,
    questions: Vec<Question>,
    #[props(default = "en".to_string())] lang: String,
) -> Element {
    if !open {
        return rsx! {};
    }

    let is_pt = lang.to_lowercase().starts_with("pt");

    let text_press_continue = if is_pt {
        "Pressione Continuar para prosseguir."
    } else {
        "Press Continue to proceed."
    };
    let text_type_answer = if is_pt {
        "Digite sua resposta..."
    } else {
        "Type your answer..."
    };
    let text_describe_details = if is_pt {
        "Descreva com o máximo de detalhes possível..."
    } else {
        "Describe in as much detail as possible..."
    };
    let text_summary_desc = if is_pt {
        "Confira o resumo das suas respostas abaixo antes de enviar."
    } else {
        "Review your answers below before submitting."
    };
    let text_not_answered = if is_pt {
        "Não respondido"
    } else {
        "Not answered"
    };
    let text_edit = if is_pt { "Editar" } else { "Edit" };
    let text_back = if is_pt { "← Voltar" } else { "← Back" };
    let text_confirm_submit = if is_pt {
        "Confirmar e Enviar →"
    } else {
        "Confirm & Submit →"
    };
    let text_continue = if is_pt {
        "Continuar ↵"
    } else {
        "Continue ↵"
    };

    let mut engine = use_signal(|| FormFlowEngine::new(questions.clone()));

    // Keep questions up to date if props change
    use_effect(use_reactive(&questions, move |q_list| {
        if engine.read().questions != q_list {
            engine.set(FormFlowEngine::new(q_list));
        }
    }));

    let current_q = engine.read().current_question().cloned();
    let progress = engine.read().progress_percent();
    let step_idx = engine.read().current_step_index();
    let total_steps = engine.read().total_eligible_questions();
    let can_go_back = engine.read().history.len() > 1;

    let handle_next = move |_| {
        let is_last = engine.read().is_last_question();
        if is_last {
            let answers_map = engine.read().answers.clone();
            let json_str = serde_json::to_string_pretty(&answers_map).unwrap_or_default();
            onsubmit.call(json_str);
        } else {
            engine.write().go_next();
        }
    };

    let handle_back = move |_| {
        engine.write().go_back();
    };

    let current_answer = current_q
        .as_ref()
        .and_then(|q| engine.read().answers.get(&q.id).cloned());

    rsx! {
        div {
            class: "uikit-modal-backdrop",
            style: "z-index: 2000; display: flex; align-items: center; justify-content: center; background: rgba(0, 0, 0, 0.75); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); padding: 20px;",
            onclick: move |_| onclose.call(()),

            div {
                class: "uikit-modal-content",
                style: "max-width: 760px; width: 100%; max-height: 90vh; display: flex; flex-direction: column; overflow: hidden; border-radius: 16px; border: 1px solid var(--uikit-border); background: var(--uikit-bg); box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4); animation: uikit-fade-in 0.25s ease-out;",
                onclick: move |e| e.stop_propagation(),

                // Top Progress Bar & Header
                div {
                    style: "padding: 20px 24px 12px 24px; border-bottom: 1px solid var(--uikit-border); display: flex; flex-direction: column; gap: 12px;",
                    div {
                        style: "display: flex; align-items: center; justify-content: space-between;",
                        div {
                            style: "display: flex; align-items: center; gap: 10px;",
                            Badge { variant: BadgeVariant::Info, size: BadgeSize::Normal, "{step_idx} → {total_steps}" }
                            if let Some(ref q) = current_q {
                                span { style: "font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; color: var(--uikit-primary);", "{q.section}" }
                            }
                        }
                        button {
                            style: "background: transparent; border: none; font-size: 24px; cursor: pointer; color: var(--uikit-fg); line-height: 1; padding: 4px 8px; border-radius: 6px;",
                            onclick: move |_| onclose.call(()),
                            "×"
                        }
                    }
                    ProgressBar { value: progress, max: 100.0 }
                }

                // Question Body Container
                div {
                    style: "flex: 1; overflow-y: auto; padding: 32px 28px; display: flex; flex-direction: column; gap: 24px;",

                    if let Some(ref q) = current_q {
                        // Title & Description
                        div {
                            style: "display: flex; flex-direction: column; gap: 8px;",
                            h2 { style: "font-size: 22px; font-weight: 800; color: var(--uikit-fg); line-height: 1.3; margin: 0;", "{q.title}" }
                            if let Some(ref desc) = q.description {
                                p { style: "font-size: 14px; color: var(--uikit-muted); line-height: 1.5; margin: 0;", "{desc}" }
                            }
                        }

                        // Question Input Control Render
                        match q.question_type {
                            QuestionType::Statement => rsx! {
                                div {
                                    style: "padding: 16px; background: rgba(99, 102, 241, 0.06); border: 1px solid var(--uikit-primary); border-radius: 10px; font-size: 15px; color: var(--uikit-fg);",
                                    "{text_press_continue}"
                                }
                            },
                            QuestionType::SingleChoice => rsx! {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    if let Some(ref opts) = q.options {
                                        for (idx, opt) in opts.iter().enumerate() {
                                            {
                                                let opt_id = opt.id.clone();
                                                let q_id = q.id.clone();
                                                let is_selected = matches!(current_answer, Some(QuestionAnswer::Choice(ref c)) if c == &opt_id);
                                                let shortcut_label = format!("{}", (b'A' + idx as u8) as char);
                                                let opt_label = opt.label.clone();
                                                let opt_desc = opt.description.clone();

                                                rsx! {
                                                    div {
                                                        key: "{opt_id}",
                                                        style: "display: flex; align-items: center; justify-content: space-between;",
                                                        SelectableButton {
                                                            label: format!("[{}] {}", shortcut_label, opt_label),
                                                            selected: is_selected,
                                                            onselect: move |_| {
                                                                engine.write().answer_question(q_id.clone(), QuestionAnswer::Choice(opt_id.clone()));
                                                            },
                                                        }
                                                        if let Some(ref d) = opt_desc {
                                                            span { style: "font-size: 12px; color: var(--uikit-muted); font-style: italic;", "{d}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            QuestionType::MultipleChoice => rsx! {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 12px;",
                                    if let Some(ref opts) = q.options {
                                        for (idx, opt) in opts.iter().enumerate() {
                                            {
                                                let opt_id = opt.id.clone();
                                                let q_id = q.id.clone();
                                                let is_selected = matches!(current_answer, Some(QuestionAnswer::MultipleChoices(ref list)) if list.contains(&opt_id));
                                                let shortcut_label = format!("{}", (b'A' + idx as u8) as char);
                                                let opt_label = opt.label.clone();
                                                let opt_desc = opt.description.clone();

                                                rsx! {
                                                    div {
                                                        key: "{opt_id}",
                                                        style: "display: flex; align-items: center; justify-content: space-between;",
                                                        SelectableButton {
                                                            label: format!("[{}] {}", shortcut_label, opt_label),
                                                            selected: is_selected,
                                                            onselect: move |_| {
                                                                let mut current_ans = match engine.read().answers.get(&q_id) {
                                                                    Some(QuestionAnswer::MultipleChoices(v)) => v.clone(),
                                                                    _ => Vec::new(),
                                                                };
                                                                if current_ans.contains(&opt_id) {
                                                                    current_ans.retain(|x| x != &opt_id);
                                                                } else {
                                                                    current_ans.push(opt_id.clone());
                                                                }
                                                                engine.write().answer_question(q_id.clone(), QuestionAnswer::MultipleChoices(current_ans));
                                                            },
                                                        }
                                                        if let Some(ref d) = opt_desc {
                                                            span { style: "font-size: 12px; color: var(--uikit-muted); font-style: italic;", "{d}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            QuestionType::ShortText | QuestionType::Email | QuestionType::Phone | QuestionType::Url | QuestionType::Number => rsx! {
                                {
                                    let q_id = q.id.clone();
                                    let text_val = match current_answer {
                                        Some(QuestionAnswer::Text(ref s)) => s.clone(),
                                        _ => String::new(),
                                    };
                                    let ph_val = q.placeholder.clone().unwrap_or_else(|| text_type_answer.to_string());
                                    rsx! {
                                        TextInput {
                                            value: text_val,
                                            placeholder: ph_val,
                                            oninput: move |e: FormEvent| {
                                                engine.write().answer_question(q_id.clone(), QuestionAnswer::Text(e.value()));
                                            },
                                        }
                                    }
                                }
                            },
                            QuestionType::LongText => rsx! {
                                {
                                    let q_id = q.id.clone();
                                    let text_val = match current_answer {
                                        Some(QuestionAnswer::Text(ref s)) => s.clone(),
                                        _ => String::new(),
                                    };
                                    let ph_val = q.placeholder.clone().unwrap_or_else(|| text_describe_details.to_string());
                                    rsx! {
                                        textarea {
                                            class: "uikit-input",
                                            style: "min-height: 120px; resize: vertical; padding: 12px; font-size: 15px;",
                                            placeholder: ph_val,
                                            value: "{text_val}",
                                            oninput: move |e| {
                                                engine.write().answer_question(q_id.clone(), QuestionAnswer::Text(e.value()));
                                            },
                                        }
                                    }
                                }
                            },
                            QuestionType::Summary => rsx! {
                                div {
                                    style: "display: flex; flex-direction: column; gap: 16px;",
                                    p { style: "font-size: 14px; color: var(--uikit-muted); margin: 0;", "{text_summary_desc}" }

                                    div {
                                        style: "display: flex; flex-direction: column; gap: 12px; background: rgba(99, 102, 241, 0.05); padding: 16px; border-radius: 10px; border: 1px solid var(--uikit-border);",
                                        for q_item in engine.read().questions.iter() {
                                            if engine.read().is_visible(q_item) && q_item.question_type != QuestionType::Statement && q_item.question_type != QuestionType::Summary {
                                                {
                                                    let ans = engine.read().answers.get(&q_item.id).cloned().unwrap_or(QuestionAnswer::None);
                                                    let q_id_edit = q_item.id.clone();
                                                    let ans_text = if ans == QuestionAnswer::None { text_not_answered.to_string() } else { ans.display_text() };
                                                    rsx! {
                                                        div {
                                                            key: "{q_item.id}",
                                                            style: "display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 1px dashed var(--uikit-border); padding-bottom: 8px;",
                                                            div {
                                                                style: "display: flex; flex-direction: column; gap: 2px;",
                                                                span { style: "font-size: 12px; font-weight: 700; color: var(--uikit-primary);", "{q_item.section}" }
                                                                span { style: "font-size: 14px; font-weight: 600; color: var(--uikit-fg);", "{q_item.title}" }
                                                                span { style: "font-size: 13px; color: var(--uikit-muted);", "{ans_text}" }
                                                            }
                                                            Button {
                                                                variant: ButtonVariant::Text,
                                                                size: ButtonSize::Small,
                                                                onclick: move |_| engine.write().jump_to(q_id_edit.clone()),
                                                                "{text_edit}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }

                // Footer Controls & Navigation
                div {
                    style: "padding: 16px 24px; border-top: 1px solid var(--uikit-border); display: flex; align-items: center; justify-content: space-between; gap: 16px; background: var(--uikit-card-bg);",
                    div {
                        if can_go_back {
                            Button {
                                variant: ButtonVariant::Secondary,
                                size: ButtonSize::Medium,
                                onclick: handle_back,
                                "{text_back}"
                            }
                        }
                    }
                    div {
                        style: "display: flex; align-items: center; gap: 12px;",
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Medium,
                            onclick: handle_next,
                            if engine.read().is_last_question() {
                                "{text_confirm_submit}"
                            } else {
                                "{text_continue}"
                            }
                        }
                    }
                }
            }
        }
    }
}
