//! What a language gives the editor beyond syntax colours: colours of its
//! own, the problems it finds, and completions as the code is typed.
//!
//! A [`LanguageSupport`] works on byte offsets into the text it is given;
//! the editor turns them into what it draws.

use crate::HighlightScope;
use std::ops::Range;

/// How bad a [`Diagnostic`] is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

/// A problem in the code: where it is, as bytes, and what it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub range: Range<usize>,
    pub message: String,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn error(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
            severity: Severity::Error,
        }
    }

    pub fn warning(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
            severity: Severity::Warning,
        }
    }
}

/// One thing the code at the caret could go on with.
#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    /// What the list shows, and what typing is matched against.
    pub label: String,
    /// What goes into the text in place of what was typed.
    pub insert: String,
    /// Where the caret goes, in characters into `insert`; its end if `None`.
    pub caret: Option<usize>,
    /// A word beside the label: a type, a category.
    pub detail: Option<String>,
    /// A line under the list while this one is chosen.
    pub documentation: Option<String>,
    /// What it is, for its colour: a type, a variable, a property.
    pub kind: HighlightScope,
}

impl Completion {
    /// A completion that inserts its label.
    pub fn new(label: impl Into<String>, kind: HighlightScope) -> Self {
        let label = label.into();
        Self {
            insert: label.clone(),
            label,
            caret: None,
            detail: None,
            documentation: None,
            kind,
        }
    }

    pub fn insert(mut self, insert: impl Into<String>, caret: Option<usize>) -> Self {
        self.insert = insert.into();
        self.caret = caret;
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn documentation(mut self, documentation: impl Into<String>) -> Self {
        self.documentation = Some(documentation.into());
        self
    }
}

/// What the code at the caret could go on with, and the bytes a chosen
/// completion replaces: the word typed so far, up to the caret.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Completions {
    pub replace: Range<usize>,
    pub items: Vec<Completion>,
}

/// A language the editor knows more of than its syntax colours.
pub trait LanguageSupport {
    /// The language's name, shown over the code.
    fn name(&self) -> &str;

    /// The colours of the code's parts, by byte range. They replace the
    /// syntax colours the editor would otherwise find.
    fn highlight(&self, _text: &str) -> Vec<(Range<usize>, HighlightScope)> {
        Vec::new()
    }

    /// The problems in the code.
    fn diagnostics(&self, _text: &str) -> Vec<Diagnostic> {
        Vec::new()
    }

    /// What could follow the code up to `cursor`, a byte offset: every
    /// candidate there, unfiltered -- the editor matches them against what
    /// was typed and ranks them, keeping the order given among equals.
    /// `explicit` when asked for (Ctrl+Space) rather than typed into.
    fn complete(&self, _text: &str, _cursor: usize, _explicit: bool) -> Option<Completions> {
        None
    }

    /// Characters that open the completions when typed, besides the
    /// characters of a word.
    fn triggers(&self) -> &[char] {
        &['.']
    }
}

/// How well `label` matches `typed`: `None` when it does not, and higher
/// the better. A label that starts with what was typed beats one where it
/// starts a later part (after a `.`, a `_`, a capital), which beats one
/// that only has its letters in order. Case does not count, except that an
/// exact case match edges ahead.
pub fn score(label: &str, typed: &str) -> Option<i32> {
    if typed.is_empty() {
        return Some(0);
    }
    let lower = label.to_lowercase();
    let wanted = typed.to_lowercase();
    let exact_case = i32::from(label.starts_with(typed));
    if lower.starts_with(&wanted) {
        return Some(3000 - label.len() as i32 + exact_case);
    }
    // The start of each part: after a separator, or a capital after a
    // lower-case letter.
    let chars: Vec<char> = label.chars().collect();
    let part_starts = (0..chars.len()).filter(|&i| {
        i > 0 && (matches!(chars[i - 1], '.' | '_' | ' ' | '-') || (chars[i].is_uppercase() && chars[i - 1].is_lowercase()))
    });
    for start in part_starts {
        let rest: String = chars[start..].iter().collect::<String>().to_lowercase();
        if rest.starts_with(&wanted) {
            return Some(2000 - label.len() as i32);
        }
    }
    // Letters in order, with a bonus for each that starts a part or follows
    // the one before.
    let mut wanted_chars = wanted.chars().peekable();
    let mut bonus = 0;
    let mut previous: Option<usize> = None;
    for (i, c) in lower.chars().enumerate() {
        if wanted_chars.peek() == Some(&c) {
            wanted_chars.next();
            if previous == Some(i.wrapping_sub(1)) || i == 0 {
                bonus += 2;
            }
            previous = Some(i);
        }
    }
    wanted_chars.peek().is_none().then(|| 1000 + bonus - label.len() as i32)
}

/// `items` that match `typed`, best first; equals keep their order.
pub fn rank(items: &[Completion], typed: &str) -> Vec<Completion> {
    let mut scored: Vec<(i32, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| score(&item.label, typed).map(|s| (s, index)))
        .collect();
    // Without anything typed, the order given is the ranking.
    if !typed.is_empty() {
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    }
    scored.into_iter().map(|(_, index)| items[index].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[Completion]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    #[test]
    fn a_prefix_beats_a_later_part_which_beats_letters_in_order() {
        let items: Vec<Completion> = ["Number.Add", "Add", "Vector.Add", "Mesh.Load", "Image.Adjust"]
            .into_iter()
            .map(|l| Completion::new(l, HighlightScope::Type))
            .collect();
        assert_eq!(labels(&rank(&items, "add")), ["Add", "Number.Add", "Vector.Add"]);
        assert_eq!(labels(&rank(&items, "adj")), ["Image.Adjust"]);
        assert_eq!(labels(&rank(&items, "nadd")), ["Number.Add"]);
        assert_eq!(labels(&rank(&items, "")).len(), 5, "nothing typed: everything, as given");
    }

    #[test]
    fn a_capital_starts_a_part() {
        assert!(score("VaeDecode", "dec").unwrap() > score("VaeDecode", "aed").unwrap_or(i32::MIN));
        assert!(score("radius", "rx").is_none());
    }
}
