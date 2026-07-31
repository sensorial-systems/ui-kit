use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::question_answer::QuestionAnswer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum Condition {
    Equals {
        question_id: String,
        value: String,
    },
    Includes {
        question_id: String,
        value: String,
    },
    And {
        conditions: Vec<Condition>,
    },
    Or {
        conditions: Vec<Condition>,
    },
    Not {
        condition: Box<Condition>,
    },
}

impl Condition {
    pub fn evaluate(&self, answers: &HashMap<String, QuestionAnswer>) -> bool {
        match self {
            Condition::Equals { question_id, value } => {
                if let Some(ans) = answers.get(question_id) {
                    match ans {
                        QuestionAnswer::Choice(val) => val == value,
                        QuestionAnswer::Text(val) => val == value,
                        QuestionAnswer::MultipleChoices(vals) => vals.contains(value),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Condition::Includes { question_id, value } => {
                if let Some(ans) = answers.get(question_id) {
                    match ans {
                        QuestionAnswer::MultipleChoices(vals) => vals.contains(value),
                        QuestionAnswer::Choice(val) => val.contains(value),
                        QuestionAnswer::Text(val) => val.contains(value),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Condition::And { conditions } => conditions.iter().all(|c| c.evaluate(answers)),
            Condition::Or { conditions } => conditions.iter().any(|c| c.evaluate(answers)),
            Condition::Not { condition } => !condition.evaluate(answers),
        }
    }
}
