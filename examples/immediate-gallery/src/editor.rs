use super::*;
use ui_kit_core::{FormatAction, RichText, RichTextEditor, TextFormat, TextRun};
use ui_kit_immediate::immediate::{RichTextPaint, TextNavigation};
pub struct Editor {
    pub saved: RichText,
    pub draft: RichTextEditor,
    pub editing: bool,
    pub time: f32,
    layout: ui_kit_wgpu::RichTextLayout,
    last_click: f32,
    previous_down: bool,
    just_opened: bool,
}
impl Default for Editor {
    fn default() -> Self {
        let mut saved = RichText::default();
        for (text, bold, italic) in [
            ("Hello ", false, false),
            ("World", true, false),
            ("! This is a ", false, false),
            ("WYSIWYG", false, true),
            (
                " editor.\nDouble-click this text block to edit formatting.",
                false,
                false,
            ),
        ] {
            saved.runs.push(TextRun {
                text: text.into(),
                format: TextFormat {
                    bold,
                    italic,
                    ..Default::default()
                },
            });
        }
        Self {
            draft: RichTextEditor::new(saved.clone()),
            saved,
            editing: false,
            time: 0.0,
            layout: Default::default(),
            last_click: -10.0,
            previous_down: false,
            just_opened: false,
        }
    }
}
impl Editor {
    pub fn begin(&mut self) {
        self.draft = RichTextEditor::new(self.saved.clone());
        self.editing = true;
        self.just_opened = true;
    }
    pub fn cancel(&mut self) {
        self.editing = false;
        self.draft = RichTextEditor::new(self.saved.clone());
    }
    pub fn save(&mut self) {
        self.saved = self.draft.document.clone();
        self.editing = false;
    }
    pub(super) fn draw(&mut self, p: &mut Painter, rect: Rect) {
        if !self.editing {
            let response = p.interact(
                "rich-preview",
                Rect {
                    width: rect.width - 80.0,
                    ..rect
                },
            );
            if response.clicked {
                if self.time - self.last_click < 0.4 {
                    self.begin();
                }
                self.last_click = self.time;
            }
            p.box_(rect, p.palette.card, p.palette.border, 8.0);
            p.ui.style = Style {
                foreground: p.palette.fg,
                font_size: 16.0,
                text_padding: 0.0,
                ..p.base()
            };
            p.ui.rich_text(
                p.screen(r(
                    rect.x + 12.0,
                    rect.y + 12.0,
                    rect.width - 92.0,
                    rect.height - 24.0,
                )),
                RichTextPaint {
                    document: self.saved.clone(),
                    cursor: None,
                    anchor: 0,
                },
            );
            if p.button(
                "rich-edit",
                r(rect.x + rect.width - 70.0, rect.y + 12.0, 58.0, 30.0),
                "Edit",
                1,
                false,
            ) {
                self.begin();
            }
            return;
        }
        p.box_(rect, p.palette.card, p.palette.border, 8.0);
        let actions = [
            ("B", FormatAction::Bold),
            ("I", FormatAction::Italic),
            ("U", FormatAction::Underline),
            ("S", FormatAction::Strike),
            ("H1", FormatAction::Heading(1)),
            ("H2", FormatAction::Heading(2)),
            ("P", FormatAction::Heading(0)),
            ("Clear", FormatAction::Clear),
        ];
        for (i, (label, action)) in actions.iter().enumerate() {
            if p.button(
                &format!("rich-format-{i}"),
                r(rect.x + 8.0 + i as f32 * 48.0, rect.y + 8.0, 44.0, 30.0),
                label,
                1,
                false,
            ) {
                self.draft.format(*action);
            }
            p.ui.commands.last_mut().unwrap().style.text_padding = 2.0;
        }
        for (i, label) in [
            "• List", "1. List", "Left", "Center", "Right", "Undo", "Redo",
        ]
        .iter()
        .enumerate()
        {
            if p.button(
                &format!("rich-tool-{i}"),
                r(rect.x + 8.0 + i as f32 * 66.0, rect.y + 44.0, 62.0, 30.0),
                label,
                1,
                false,
            ) {
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
            p.ui.commands.last_mut().unwrap().style.text_padding = 3.0;
        }
        let body = r(
            rect.x + 12.0,
            rect.y + 88.0,
            rect.width - 24.0,
            rect.height - 140.0,
        );
        let response = p.interact("rich-body", body);
        if self.just_opened {
            p.ui.request_focus("rich-body");
            self.just_opened = false;
        }
        if response.active {
            if let Some(pt) = p.input.pointer {
                let at = self.layout.hit(
                    &self.draft.document,
                    body.width,
                    body.height,
                    16.0,
                    pt.x - body.x,
                    pt.y - p.screen(body).y,
                );
                self.draft
                    .place(at, self.previous_down || p.input.extend_selection);
            }
        }
        self.previous_down = p.input.down;
        let owns_focus = p.ui.focused("rich-body")
            || p.ui.focused("rich-edit")
            || (0..8).any(|i| p.ui.focused(&format!("rich-format-{i}")))
            || (0..7).any(|i| p.ui.focused(&format!("rich-tool-{i}")));
        if owns_focus {
            if p.input.select_all {
                self.draft.select_all();
            }
            if p.input.undo {
                self.draft.undo();
            }
            if p.input.redo {
                self.draft.redo();
            }
            if p.input.backspace {
                self.draft.backspace();
            }
            if p.input.delete {
                self.draft.delete();
            }
            if let Some(nav) = p.input.navigation {
                let extend = p.input.extend_selection;
                match nav {
                    TextNavigation::Left => self.draft.move_by(-1, extend),
                    TextNavigation::Right => self.draft.move_by(1, extend),
                    TextNavigation::Home => {
                        let text = self.draft.document.text();
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
                        let text = self.draft.document.text();
                        let pos = text
                            .chars()
                            .skip(self.draft.cursor)
                            .position(|c| c == '\n')
                            .map_or(len, |i| self.draft.cursor + i);
                        self.draft.place(pos, extend)
                    }
                    _ => {
                        let at = self.layout.vertical(
                            &self.draft.document,
                            body.width,
                            body.height,
                            16.0,
                            self.draft.cursor,
                            matches!(nav, TextNavigation::Down),
                        );
                        self.draft.place(at, extend);
                    }
                }
            }
            if !p.input.text.is_empty() {
                self.draft.insert(&p.input.text);
            }
            if p.input.activate && !p.input.save && p.ui.focused("rich-body") {
                self.draft.insert("\n");
            }
        }
        p.ui.style = Style {
            foreground: p.palette.fg,
            font_size: 16.0,
            text_padding: 0.0,
            ..p.base()
        };
        p.ui.rich_text(
            p.screen(body),
            RichTextPaint {
                document: self.draft.document.clone(),
                cursor: if owns_focus {
                    Some(self.draft.cursor)
                } else {
                    None
                },
                anchor: self.draft.anchor,
            },
        );
        if p.button(
            "rich-cancel",
            r(
                rect.x + rect.width - 184.0,
                rect.y + rect.height - 42.0,
                80.0,
                30.0,
            ),
            "Cancel",
            2,
            false,
        ) {
            self.cancel();
        }
        if p.button(
            "rich-save",
            r(
                rect.x + rect.width - 94.0,
                rect.y + rect.height - 42.0,
                80.0,
                30.0,
            ),
            "Save",
            0,
            false,
        ) || (owns_focus && p.input.save)
        {
            self.save();
        }
    }
}
