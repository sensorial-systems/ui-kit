//! A step-by-step form whose questions can depend on earlier answers.
use super::button::{Button, ButtonVariant};
use super::feedback::ProgressBar;
use super::input::{Checkbox, TextInput};
use super::Label;
use crate::{Insets, Stack, Strip, Ui};
use ui_kit_core::Rect;

/// What a [`Question`] asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestionKind {
    /// Nothing: an introduction to read and continue past.
    Info,
    /// One of the options.
    Single(Vec<String>),
    /// Any of the options.
    Multiple(Vec<String>),
    /// Free text.
    Text { placeholder: String },
}

/// When a question is asked: only if an earlier single-choice question was
/// answered with a given option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
    pub question: usize,
    pub option: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub title: String,
    pub description: String,
    pub kind: QuestionKind,
    /// Whether Continue waits for an answer.
    pub required: bool,
    pub condition: Option<Condition>,
}

impl Question {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        kind: QuestionKind,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            kind,
            required: false,
            condition: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Only ask this when `question` was answered with `option`.
    pub fn when(mut self, question: usize, option: usize) -> Self {
        self.condition = Some(Condition { question, option });
        self
    }
}

/// An answer to a [`Question`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Answer {
    #[default]
    None,
    Single(usize),
    Multiple(Vec<bool>),
    Text(String),
}

/// Where a [`FormFlow`] is and what has been answered so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormFlowState {
    pub step: usize,
    pub answers: Vec<Answer>,
}

impl FormFlowState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// What happened in a [`FormFlow`] this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormFlowEvent {
    Submitted,
}

/// Questions shown one at a time, skipping any whose condition is not met,
/// with a progress bar, Back and Continue.
#[derive(Clone, Debug, PartialEq)]
pub struct FormFlow<'a> {
    questions: &'a [Question],
}

impl<'a> FormFlow<'a> {
    pub fn new(questions: &'a [Question]) -> Self {
        Self { questions }
    }

    /// Whether question `index` is asked, given the answers so far.
    pub fn is_asked(&self, state: &FormFlowState, index: usize) -> bool {
        self.questions[index]
            .condition
            .as_ref()
            .is_none_or(|c| state.answers.get(c.question) == Some(&Answer::Single(c.option)))
    }

    /// The questions asked, in order.
    pub fn asked(&self, state: &FormFlowState) -> Vec<usize> {
        (0..self.questions.len())
            .filter(|i| self.is_asked(state, *i))
            .collect()
    }

    fn answered(&self, state: &FormFlowState, index: usize) -> bool {
        match (&self.questions[index].kind, state.answers.get(index)) {
            (QuestionKind::Info, _) => true,
            (QuestionKind::Single(_), Some(Answer::Single(_))) => true,
            (QuestionKind::Multiple(_), Some(Answer::Multiple(chosen))) => chosen.contains(&true),
            (QuestionKind::Text { .. }, Some(Answer::Text(text))) => !text.trim().is_empty(),
            _ => false,
        }
    }

    /// Draw the current question in `rect`, with room above for a close button.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        state: &mut FormFlowState,
    ) -> Option<FormFlowEvent> {
        if self.questions.is_empty() {
            return None;
        }
        state.answers.resize(self.questions.len(), Answer::None);
        state.step = state.step.min(self.questions.len() - 1);
        let asked = self.asked(state);
        let position = asked.iter().position(|i| *i == state.step).unwrap_or(0);
        let question = &self.questions[state.step];

        let mut column = Strip::column(rect.inset_by(Insets::new(24.0, 20.0, 24.0, 28.0)), 8.0);
        Label::heading(20.0).show(
            ui,
            column.leading(52.0).split_left(450.0, 0.0).0,
            &question.title,
        );
        ProgressBar::new().show(
            ui,
            column.leading(5.0),
            (position + 1) as f32 / asked.len().max(1) as f32,
        );
        let mut buttons = Strip::row(column.trailing(38.0), 0.0);
        let (back, next) = (buttons.leading(110.0), buttons.trailing(150.0));
        let mut body = Stack::new(column.rest(), 8.0);
        let answer = &mut state.answers[state.step];
        match &question.kind {
            QuestionKind::Info => {
                Label::muted()
                    .size(16.0)
                    .show(ui, body.row(120.0), &question.description)
            }
            QuestionKind::Single(options) => {
                Label::muted()
                    .size(13.0)
                    .show(ui, body.row(35.0), &question.description);
                for (i, option) in options.iter().enumerate() {
                    let chosen = *answer == Answer::Single(i);
                    if Button::new(option)
                        .variant(if chosen {
                            ButtonVariant::Selected
                        } else {
                            ButtonVariant::Outline
                        })
                        .show(
                            ui,
                            &format!("{id}-{}-option-{i}", state.step),
                            body.row(38.0),
                        )
                        .clicked
                    {
                        *answer = Answer::Single(i);
                    }
                }
            }
            QuestionKind::Multiple(options) => {
                Label::muted()
                    .size(13.0)
                    .show(ui, body.row(42.0), &question.description);
                if !matches!(answer, Answer::Multiple(_)) {
                    *answer = Answer::Multiple(vec![false; options.len()]);
                }
                if let Answer::Multiple(chosen) = answer {
                    chosen.resize(options.len(), false);
                    for (i, option) in options.iter().enumerate() {
                        Checkbox::new().show(
                            ui,
                            &format!("{id}-{}-check-{i}", state.step),
                            body.row(36.0),
                            option,
                            &mut chosen[i],
                        );
                    }
                }
            }
            QuestionKind::Text { placeholder } => {
                Label::muted()
                    .size(13.0)
                    .show(ui, body.row(48.0), &question.description);
                if !matches!(answer, Answer::Text(_)) {
                    *answer = Answer::Text(String::new());
                }
                if let Answer::Text(text) = answer {
                    TextInput::new(placeholder).show(
                        ui,
                        &format!("{id}-{}-text", state.step),
                        body.row(126.0),
                        text,
                    );
                }
            }
        }

        if position > 0
            && Button::new("Back")
                .variant(ButtonVariant::Outline)
                .show(ui, &format!("{id}-back"), back)
                .clicked
        {
            state.step = asked[position - 1];
            return None;
        }
        let last = position + 1 == asked.len();
        let ready = !question.required || self.answered(state, state.step);
        if Button::new(if last { "Submit" } else { "Continue" })
            .disabled(!ready)
            .show(ui, &format!("{id}-next"), next)
            .clicked
        {
            if last {
                return Some(FormFlowEvent::Submitted);
            }
            // The answer just given may change which questions follow.
            let asked = self.asked(state);
            if let Some(following) = asked.iter().find(|i| **i > state.step) {
                state.step = *following;
            }
        }
        None
    }
}
