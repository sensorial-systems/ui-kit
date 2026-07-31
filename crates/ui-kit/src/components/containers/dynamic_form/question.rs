use serde::{Deserialize, Serialize};
use super::condition::Condition;
use super::question_option::QuestionOption;
use super::question_type::QuestionType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub section: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub question_type: QuestionType,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<QuestionOption>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_examples: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_when: Option<Condition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_question_id: Option<String>,
}

impl Question {
    pub fn new(id: impl Into<String>, section: impl Into<String>, title: impl Into<String>, q_type: QuestionType) -> Self {
        Self {
            id: id.into(),
            section: section.into(),
            title: title.into(),
            description: None,
            question_type: q_type,
            required: false,
            options: None,
            placeholder: None,
            help_examples: None,
            visible_when: None,
            next_question_id: None,
        }
    }
}
