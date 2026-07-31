use std::collections::HashMap;
use super::question::Question;
use super::question_answer::QuestionAnswer;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormFlowEngine {
    pub questions: Vec<Question>,
    pub answers: HashMap<String, QuestionAnswer>,
    pub history: Vec<String>,
}

impl FormFlowEngine {
    pub fn new(questions: Vec<Question>) -> Self {
        let first_id = questions.first().map(|q| q.id.clone());
        let history = if let Some(id) = first_id { vec![id] } else { vec![] };
        Self {
            questions,
            answers: HashMap::new(),
            history,
        }
    }

    pub fn current_question(&self) -> Option<&Question> {
        let current_id = self.history.last()?;
        self.questions.iter().find(|q| &q.id == current_id)
    }

    pub fn is_visible(&self, question: &Question) -> bool {
        if let Some(ref cond) = question.visible_when {
            cond.evaluate(&self.answers)
        } else {
            true
        }
    }

    pub fn answer_question(&mut self, question_id: String, answer: QuestionAnswer) {
        self.answers.insert(question_id, answer);
        self.prune_invalid_branch_answers();
    }

    pub fn get_next_question_id(&self, current_question: &Question) -> Option<String> {
        // 1. Check if selected option overrides next_question_id
        if let Some(ans) = self.answers.get(&current_question.id) {
            if let QuestionAnswer::Choice(ref choice_id) = ans {
                if let Some(opts) = &current_question.options {
                    if let Some(opt) = opts.iter().find(|o| &o.id == choice_id) {
                        if let Some(ref override_id) = opt.next_question_id {
                            return Some(override_id.clone());
                        }
                    }
                }
            }
        }

        // 2. Check if question itself has fixed next_question_id
        if let Some(ref fixed_next) = current_question.next_question_id {
            return Some(fixed_next.clone());
        }

        // 3. Otherwise find next eligible question in array order that passes visible_when condition
        let idx = self.questions.iter().position(|q| q.id == current_question.id)?;
        for q in self.questions.iter().skip(idx + 1) {
            if self.is_visible(q) {
                return Some(q.id.clone());
            }
        }

        None
    }

    pub fn go_next(&mut self) -> bool {
        if let Some(current) = self.current_question().cloned() {
            if let Some(next_id) = self.get_next_question_id(&current) {
                self.history.push(next_id);
                return true;
            }
        }
        false
    }

    pub fn go_back(&mut self) -> bool {
        if self.history.len() > 1 {
            self.history.pop();
            self.prune_invalid_branch_answers();
            true
        } else {
            false
        }
    }

    pub fn jump_to(&mut self, question_id: String) {
        if let Some(idx) = self.history.iter().position(|id| id == &question_id) {
            self.history.truncate(idx + 1);
        } else {
            self.history.push(question_id);
        }
        self.prune_invalid_branch_answers();
    }

    pub fn prune_invalid_branch_answers(&mut self) {
        let invalid_ids: Vec<String> = self
            .questions
            .iter()
            .filter(|q| !self.is_visible(q))
            .map(|q| q.id.clone())
            .collect();

        for id in invalid_ids {
            self.answers.remove(&id);
        }
    }

    pub fn progress_percent(&self) -> f64 {
        let total_eligible = self.questions.iter().filter(|q| self.is_visible(q)).count();
        if total_eligible == 0 {
            return 100.0;
        }
        let current_step = self.history.len();
        ((current_step as f64) / (total_eligible as f64) * 100.0).min(100.0)
    }

    pub fn current_step_index(&self) -> usize {
        self.history.len()
    }

    pub fn total_eligible_questions(&self) -> usize {
        self.questions.iter().filter(|q| self.is_visible(q)).count()
    }

    pub fn is_last_question(&self) -> bool {
        if let Some(current) = self.current_question() {
            self.get_next_question_id(current).is_none()
        } else {
            true
        }
    }
}
