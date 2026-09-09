//! Renderer-independent formatted text and editing operations.
use crate::TextAlign;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextFormat {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub heading: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub format: TextFormat,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RichText {
    pub runs: Vec<TextRun>,
    pub align: TextAlign,
}
impl RichText {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            runs: vec![TextRun {
                text: text.into(),
                format: TextFormat::default(),
            }],
            ..Default::default()
        }
    }
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
    pub fn len(&self) -> usize {
        self.runs.iter().map(|r| r.text.chars().count()).sum()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn cells(&self) -> Vec<(char, TextFormat)> {
        self.runs
            .iter()
            .flat_map(|r| r.text.chars().map(move |c| (c, r.format)))
            .collect()
    }
    fn set_cells(&mut self, cells: Vec<(char, TextFormat)>) {
        self.runs.clear();
        for (c, format) in cells {
            if let Some(last) = self.runs.last_mut().filter(|r| r.format == format) {
                last.text.push(c);
            } else {
                self.runs.push(TextRun {
                    text: c.into(),
                    format,
                });
            }
        }
    }
    pub fn to_html(&self) -> String {
        fn escape(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        }
        let mut html = format!(
            "<div style=\"white-space:pre-wrap;text-align:{}\">",
            match self.align {
                TextAlign::Left => "left",
                TextAlign::Center => "center",
                TextAlign::Right => "right",
            }
        );
        for run in &self.runs {
            let f = run.format;
            html+=&format!("<span style=\"font-weight:{};font-style:{};text-decoration:{} {};font-size:{}em\">{}</span>",if f.bold||f.heading>0{700}else{400},if f.italic{"italic"}else{"normal"},if f.underline{"underline"}else{""},if f.strike{"line-through"}else{""},match f.heading{1=>2.0,2=>1.5,_=>1.0},escape(&run.text));
        }
        html + "</div>"
    }
}
#[derive(Clone, Copy, Debug)]
pub enum FormatAction {
    Bold,
    Italic,
    Underline,
    Strike,
    Heading(u8),
    Clear,
}
#[derive(Clone, Debug)]
pub struct RichTextEditor {
    pub document: RichText,
    pub cursor: usize,
    pub anchor: usize,
    pub typing: TextFormat,
    undo: Vec<RichText>,
    redo: Vec<RichText>,
}
impl RichTextEditor {
    fn previous(&self) -> usize {
        let text = self.document.text();
        let mut count = 0;
        let mut previous = 0;
        for g in text.graphemes(true) {
            if count >= self.cursor {
                break;
            }
            previous = count;
            count += g.chars().count();
        }
        previous
    }
    fn next(&self) -> usize {
        let text = self.document.text();
        let mut count = 0;
        for g in text.graphemes(true) {
            count += g.chars().count();
            if count > self.cursor {
                return count;
            }
        }
        count
    }
    pub fn new(document: RichText) -> Self {
        let cursor = document.len();
        let typing = document
            .runs
            .last()
            .map_or(TextFormat::default(), |r| r.format);
        Self {
            document,
            cursor,
            anchor: cursor,
            typing,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub fn selection(&self) -> std::ops::Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.document.len();
    }
    pub fn place(&mut self, index: usize, extend: bool) {
        self.cursor = index.min(self.document.len());
        if !extend {
            self.anchor = self.cursor;
        }
        let cells = self.document.cells();
        self.typing = cells
            .get(self.cursor.saturating_sub(1))
            .map_or(TextFormat::default(), |(_, f)| *f);
    }
    pub fn move_by(&mut self, delta: isize, extend: bool) {
        let selected = self.selection();
        let next = if !extend && !selected.is_empty() {
            if delta < 0 {
                selected.start
            } else {
                selected.end
            }
        } else {
            if delta < 0 {
                self.previous()
            } else {
                self.next()
            }
        };
        self.place(next, extend);
    }
    fn checkpoint(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn insert(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.checkpoint();
        let mut cells = self.document.cells();
        let range = self.selection();
        let start = range.start;
        cells.splice(range, text.chars().map(|c| (c, self.typing)));
        self.document.set_cells(cells);
        self.cursor = start + text.chars().count();
        self.anchor = self.cursor;
    }
    pub fn backspace(&mut self) {
        if self.selection().is_empty() {
            if self.cursor == 0 {
                return;
            }
            self.anchor = self.previous();
        }
        self.delete();
    }
    pub fn delete(&mut self) {
        if self.selection().is_empty() {
            if self.cursor == self.document.len() {
                return;
            }
            self.anchor = self.next();
        }
        self.checkpoint();
        let mut cells = self.document.cells();
        let range = self.selection();
        self.cursor = range.start;
        cells.drain(range);
        self.document.set_cells(cells);
        self.anchor = self.cursor;
    }
    pub fn format(&mut self, action: FormatAction) {
        let range = self.selection();
        let cells = self.document.cells();
        let current = if range.is_empty() {
            self.typing
        } else {
            cells[range.start].1
        };
        let mut format = current;
        match action {
            FormatAction::Bold => format.bold = !current.bold,
            FormatAction::Italic => format.italic = !current.italic,
            FormatAction::Underline => format.underline = !current.underline,
            FormatAction::Strike => format.strike = !current.strike,
            FormatAction::Heading(h) => format.heading = h.min(2),
            FormatAction::Clear => format = TextFormat::default(),
        };
        self.typing = format;
        if range.is_empty() {
            return;
        }
        self.checkpoint();
        let mut cells = cells;
        for (_, f) in &mut cells[range] {
            match action {
                FormatAction::Bold => f.bold = format.bold,
                FormatAction::Italic => f.italic = format.italic,
                FormatAction::Underline => f.underline = format.underline,
                FormatAction::Strike => f.strike = format.strike,
                FormatAction::Heading(_) => f.heading = format.heading,
                FormatAction::Clear => *f = format,
            }
        }
        self.document.set_cells(cells);
    }
    pub fn align(&mut self, align: TextAlign) {
        if self.document.align != align {
            self.checkpoint();
            self.document.align = align;
        }
    }
    pub fn undo(&mut self) {
        if let Some(doc) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.document, doc));
            self.place(self.cursor, false);
        }
    }
    pub fn redo(&mut self) {
        if let Some(doc) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.document, doc));
            self.place(self.cursor, false);
        }
    }
    pub fn list(&mut self, ordered: bool) {
        self.checkpoint();
        let mut cells = self.document.cells();
        let range = self.selection();
        let start = cells[..range.start]
            .iter()
            .rposition(|(c, _)| *c == '\n')
            .map_or(0, |i| i + 1);
        let mut positions = vec![start];
        for i in start..range.end {
            if cells[i].0 == '\n' && i + 1 < cells.len() {
                positions.push(i + 1);
            }
        }
        for (i, pos) in positions.into_iter().enumerate().rev() {
            let prefix = if ordered {
                format!("{}. ", i + 1)
            } else {
                "• ".into()
            };
            cells.splice(pos..pos, prefix.chars().map(|c| (c, self.typing)));
        }
        self.document.set_cells(cells);
        self.place(self.document.len(), false);
    }
}
