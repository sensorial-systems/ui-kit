use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum QuestionAnswer {
    Text(String),
    Choice(String),
    MultipleChoices(Vec<String>),
    Number(f64),
    None,
}

impl QuestionAnswer {
    pub fn is_empty(&self) -> bool {
        match self {
            QuestionAnswer::Text(s) => s.trim().is_empty(),
            QuestionAnswer::Choice(s) => s.trim().is_empty(),
            QuestionAnswer::MultipleChoices(v) => v.is_empty(),
            QuestionAnswer::None => true,
            QuestionAnswer::Number(_) => false,
        }
    }

    pub fn display_text(&self) -> String {
        match self {
            QuestionAnswer::Text(s) => s.clone(),
            QuestionAnswer::Choice(s) => s.clone(),
            QuestionAnswer::MultipleChoices(v) => v.join(", "),
            QuestionAnswer::Number(n) => n.to_string(),
            QuestionAnswer::None => "Não respondido".to_string(),
        }
    }
}
