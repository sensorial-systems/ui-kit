use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::theme::base;
use crate::immediate::{RichTextPaint, TextNavigation};
use crate::layout::{HorizontalAlignment, VerticalAlignment};
use crate::{Insets, Strip, Ui};
use ui_kit_core::{FormatAction, Rect, RichText, RichTextEditor, Style, TextAlign};

/// What an [`EditableText`] needs to know about laid-out text to place the
/// caret: which character is at a point, and which is a line above or below.
///
/// The renderer that shapes the text answers this; `ui_kit_wgpu` provides one.
pub trait RichTextMetrics {
    /// The character index nearest `(x, y)`, relative to the text's top left.
    fn hit(&mut self, doc: &RichText, width: f32, height: f32, size: f32, x: f32, y: f32) -> usize;

    /// The character index a line below (`down`) or above `position`.
    fn vertical(
        &mut self,
        doc: &RichText,
        width: f32,
        height: f32,
        size: f32,
        position: usize,
        down: bool,
    ) -> usize;
}

/// A block of rich text that is read until double-clicked or Edit is pressed,
/// then edited with a formatting toolbar and saved or cancelled.
pub struct EditableText {
    /// The committed text.
    pub saved: RichText,
    /// The text being edited.
    pub draft: RichTextEditor,
    pub editing: bool,
    last_click: f32,
    previous_down: bool,
    just_opened: bool,
    /// The word a double press selected, which a drag after it grows from
    /// a word at a time.
    word: Option<(usize, usize)>,
}

/// How big the text is drawn.
const SIZE: f32 = 16.0;

const FORMATS: [(&str, FormatAction); 8] = [
    ("B", FormatAction::Bold),
    ("I", FormatAction::Italic),
    ("U", FormatAction::Underline),
    ("S", FormatAction::Strike),
    ("H1", FormatAction::Heading(1)),
    ("H2", FormatAction::Heading(2)),
    ("P", FormatAction::Heading(0)),
    ("Clear", FormatAction::Clear),
];
const TOOLS: [&str; 7] = [
    "Bullets", "Numbers", "Left", "Center", "Right", "Undo", "Redo",
];

impl EditableText {
    pub fn new(text: RichText) -> Self {
        Self {
            draft: RichTextEditor::new(text.clone()),
            saved: text,
            editing: false,
            last_click: -10.0,
            previous_down: false,
            just_opened: false,
            word: None,
        }
    }

    /// How tall it is: short while reading, room for the toolbars while editing.
    pub fn height(&self) -> f32 {
        if self.editing {
            300.0
        } else {
            80.0
        }
    }

    /// Start editing a copy of the saved text.
    pub fn begin(&mut self) {
        self.draft = RichTextEditor::new(self.saved.clone());
        self.editing = true;
        self.just_opened = true;
    }

    /// Stop editing and forget the changes.
    pub fn cancel(&mut self) {
        self.editing = false;
        self.draft = RichTextEditor::new(self.saved.clone());
    }

    /// Stop editing and keep the changes.
    pub fn save(&mut self) {
        self.saved = self.draft.document.clone();
        self.editing = false;
    }

    fn text_style(ui: &Ui) -> Style {
        Style {
            foreground: ui.theme.palette.text,
            font_size: SIZE,
            text_padding: 0.0,
            ..base(ui)
        }
    }

    /// Draw the text in `rect`, `time` seconds in (for double-clicks).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        metrics: &mut dyn RichTextMetrics,
        time: f32,
    ) {
        let p = ui.theme.palette;
        panel(ui, rect, p.surface, p.border, 8.0);
        if !self.editing {
            let response = ui.interact(&format!("{id}-preview"), rect.split_right(80.0, 0.0).0);
            if response.clicked {
                if time - self.last_click < 0.4 {
                    self.begin();
                }
                self.last_click = time;
            }
            let style = Self::text_style(ui);
            let paint = RichTextPaint {
                document: self.saved.clone(),
                cursor: None,
                anchor: 0,
            };
            ui.with_style(style, |ui| {
                ui.rich_text(rect.inset_by(Insets::new(12.0, 12.0, 80.0, 12.0)), paint)
            });
            let edit = rect.inset(12.0).align(
                58.0,
                30.0,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
            );
            if Button::new("Edit")
                .variant(ButtonVariant::Secondary)
                .show(ui, &format!("{id}-edit"), edit)
                .clicked
            {
                self.begin();
            }
            return;
        }

        // Two toolbars over the body, and the commit buttons under it.
        let mut toolbars = Strip::column(rect.inset(8.0), 6.0);
        let mut formats = Strip::row(toolbars.leading(30.0), 4.0);
        for (i, (label, action)) in FORMATS.iter().enumerate() {
            if Button::new(label)
                .variant(ButtonVariant::Secondary)
                .padding(2.0)
                .show(ui, &format!("{id}-format-{i}"), formats.leading(44.0))
                .clicked
            {
                self.draft.format(*action);
            }
        }
        let mut tools = Strip::row(toolbars.leading(30.0), 4.0);
        for (i, label) in TOOLS.iter().enumerate() {
            if Button::new(label)
                .variant(ButtonVariant::Secondary)
                .padding(3.0)
                .font_size(13.0)
                .show(ui, &format!("{id}-tool-{i}"), tools.leading(62.0))
                .clicked
            {
                match i {
                    0 => self.draft.list(false),
                    1 => self.draft.list(true),
                    2 => self.draft.align(TextAlign::Left),
                    3 => self.draft.align(TextAlign::Center),
                    4 => self.draft.align(TextAlign::Right),
                    5 => self.draft.undo(),
                    _ => self.draft.redo(),
                }
            }
        }
        let body = rect.inset_by(Insets::new(12.0, 88.0, 12.0, 52.0));
        let mut commit = Strip::row(
            rect.inset_by(Insets::new(12.0, 0.0, 14.0, 12.0))
                .split_bottom(30.0, 0.0)
                .1,
            10.0,
        );
        let save = commit.trailing(80.0);
        let cancel = commit.trailing(80.0);

        let body_id = format!("{id}-body");
        let response = ui.interact(&body_id, body);
        if self.just_opened {
            ui.request_focus(&body_id);
            self.just_opened = false;
        }
        let input = ui.input().clone();
        if response.active {
            if let Some(pt) = input.pointer {
                let at = metrics.hit(
                    &self.draft.document,
                    body.width,
                    body.height,
                    SIZE,
                    pt.x - body.x,
                    pt.y - body.y,
                );
                // A double press selects a word and a drag after it whole
                // words; a triple press the line.
                if ui.pressed() {
                    self.word = None;
                    match ui.click_count() {
                        2 => self.word = Some(self.draft.select_word(at)),
                        count if count >= 3 => self.draft.select_line(at),
                        _ => self.draft.place(at, input.extend_selection),
                    }
                } else if let Some(word) = self.word {
                    self.draft.select_words(word, at);
                } else if ui.click_count() < 3 {
                    self.draft
                        .place(at, self.previous_down || input.extend_selection);
                }
            }
        }
        self.previous_down = input.down;
        let owns_focus = ui.focused(&body_id)
            || (0..FORMATS.len()).any(|i| ui.focused(&format!("{id}-format-{i}")))
            || (0..TOOLS.len()).any(|i| ui.focused(&format!("{id}-tool-{i}")));
        if owns_focus {
            if input.select_all {
                self.draft.select_all();
            }
            if input.undo {
                self.draft.undo();
            }
            if input.redo {
                self.draft.redo();
            }
            if input.backspace {
                self.draft.backspace();
            }
            if input.delete {
                self.draft.delete();
            }
            if let Some(nav) = input.navigation {
                let extend = input.extend_selection;
                let text = self.draft.document.text();
                match nav {
                    TextNavigation::Left => self.draft.move_by(-1, extend),
                    TextNavigation::Right => self.draft.move_by(1, extend),
                    TextNavigation::Home => {
                        let pos = text
                            .chars()
                            .take(self.draft.cursor)
                            .collect::<Vec<_>>()
                            .iter()
                            .rposition(|c| *c == '\n')
                            .map_or(0, |i| i + 1);
                        self.draft.place(pos, extend)
                    }
                    TextNavigation::End => {
                        let len = self.draft.document.len();
                        let pos = text
                            .chars()
                            .skip(self.draft.cursor)
                            .position(|c| c == '\n')
                            .map_or(len, |i| self.draft.cursor + i);
                        self.draft.place(pos, extend)
                    }
                    _ => {
                        let at = metrics.vertical(
                            &self.draft.document,
                            body.width,
                            body.height,
                            SIZE,
                            self.draft.cursor,
                            matches!(nav, TextNavigation::Down),
                        );
                        self.draft.place(at, extend);
                    }
                }
            }
            if !input.text.is_empty() {
                self.draft.insert(&input.text);
            }
            if input.activate && !input.save && ui.focused(&body_id) {
                self.draft.insert("\n");
            }
        }
        let style = Self::text_style(ui);
        let paint = RichTextPaint {
            document: self.draft.document.clone(),
            cursor: owns_focus.then_some(self.draft.cursor),
            anchor: self.draft.anchor,
        };
        ui.with_style(style, |ui| ui.rich_text(body, paint));
        if Button::new("Cancel")
            .variant(ButtonVariant::Outline)
            .show(ui, &format!("{id}-cancel"), cancel)
            .clicked
        {
            self.cancel();
        }
        if Button::new("Save")
            .show(ui, &format!("{id}-save"), save)
            .clicked
            || (owns_focus && input.save)
        {
            self.save();
        }
    }
}
