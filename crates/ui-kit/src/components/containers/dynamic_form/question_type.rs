use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    #[default]
    Statement,
    SingleChoice,
    MultipleChoice,
    ShortText,
    LongText,
    Email,
    Phone,
    Url,
    Number,
    Summary,
}

impl QuestionType {
    pub fn is_choice(&self) -> bool {
        matches!(self, QuestionType::SingleChoice | QuestionType::MultipleChoice)
    }

    pub fn is_text_input(&self) -> bool {
        matches!(
            self,
            QuestionType::ShortText
                | QuestionType::LongText
                | QuestionType::Email
                | QuestionType::Phone
                | QuestionType::Url
                | QuestionType::Number
        )
    }
}
