use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_shortcut: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_question_id: Option<String>,
}

impl QuestionOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            key_shortcut: None,
            next_question_id: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.key_shortcut = Some(shortcut.into());
        self
    }

    pub fn with_next(mut self, next_id: impl Into<String>) -> Self {
        self.next_question_id = Some(next_id.into());
        self
    }
}
