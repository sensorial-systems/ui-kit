//! The editor: line numbers, syntax colours and a language picker over a
//! text area that keeps indentation and indents with spaces.

use crate::{HighlightEngine, HighlightScope, LanguageRegistry, Theme};
use std::ops::Range;
use std::sync::OnceLock;
use ui_kit_immediate::{
    Color, CursorIcon, FontFamily, Paint, Placement, Rect, Response, Style, TextAlign, TextArea, TextOverflow, Ui,
    Vec2,
};

/// The editor's least height.
pub const MIN_HEIGHT: f32 = 100.0;

const fn rgb(hex: u32) -> Color {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    ]
}

const TRANSPARENT: Color = [0.0; 4];

/// How the editor looks: its colours, its text and the syntax theme.
#[derive(Clone, Debug)]
pub struct CodeEditorLook {
    /// The colour of each highlighted scope. A scope the theme gives no
    /// style of its own shows in [`foreground`](Self::foreground).
    pub theme: Theme,
    /// The code: its size, and its lines' height as a multiple of it.
    pub font_size: f32,
    pub line: f32,
    pub foreground: Color,
    pub background: Color,
    pub border: Color,
    /// The rules under the toolbar and beside the line numbers.
    pub rule: Color,
    pub line_numbers: Color,
    pub selection: Color,
    pub caret: Color,
    pub toolbar: Color,
    pub toolbar_text: Color,
    /// The language picker's box, border and text.
    pub picker: Color,
    pub picker_border: Color,
    pub picker_text: Color,
    /// The list the picker opens: its box, its text, and the highlighted
    /// option.
    pub menu: Color,
    pub menu_border: Color,
    pub menu_text: Color,
    pub menu_highlight: Color,
    pub menu_highlight_text: Color,
}

impl CodeEditorLook {
    /// Light text on near black, with a dark toolbar and picker.
    pub fn dark() -> Self {
        let foreground = rgb(0xd4d4d4);
        Self {
            theme: Theme::dark(),
            font_size: 13.0,
            line: 1.5,
            foreground,
            background: rgb(0x1e1e1e),
            border: rgb(0x444444),
            rule: rgb(0x333333),
            line_numbers: rgb(0x858585),
            selection: [0.2, 0.6, 1.0, 0.4],
            caret: foreground,
            toolbar: rgb(0x252526),
            toolbar_text: rgb(0xcccccc),
            picker: rgb(0x3c3c3c),
            picker_border: rgb(0x555555),
            picker_text: rgb(0xeeeeee),
            menu: rgb(0x1a1a1a),
            menu_border: rgb(0x767676),
            menu_text: rgb(0xffffff),
            menu_highlight: rgb(0x1967d2),
            menu_highlight_text: rgb(0xffffff),
        }
    }

    fn line_height(&self) -> f32 {
        self.font_size * self.line
    }

    /// How `scope` shows, if not in the foreground colour.
    fn paint(&self, scope: HighlightScope) -> Option<Paint> {
        if scope == HighlightScope::Default {
            return None;
        }
        let style = self.theme.styled(scope)?;
        let color = [style.fg.r as f32 / 255.0, style.fg.g as f32 / 255.0, style.fg.b as f32 / 255.0, 1.0];
        let (bold, italic) = match style.font_style {
            crate::FontStyle::Normal => (false, false),
            crate::FontStyle::Bold => (true, false),
            crate::FontStyle::Italic => (false, true),
            crate::FontStyle::BoldItalic => (true, true),
        };
        Some(Paint { color, bold, italic })
    }
}

impl Default for CodeEditorLook {
    fn default() -> Self {
        Self::dark()
    }
}

/// A run of text in one family, size and colour, with its line's height.
struct Text {
    size: f32,
    family: FontFamily,
    color: Color,
    line_height: f32,
}

impl Text {
    fn style(&self, align: TextAlign) -> Style {
        Style {
            fill: TRANSPARENT,
            stroke: TRANSPARENT,
            stroke_width: 0.0,
            foreground: self.color,
            font_size: self.size,
            font_weight: 400,
            font_family: self.family.clone(),
            italic: false,
            letter_spacing: 0.0,
            line_height: Some(self.line_height / self.size.max(1.0)),
            text_padding: 0.0,
            text_align: align,
            overflow: TextOverflow::Clip,
            ..Style::default()
        }
    }

    fn width(&self, ui: &Ui, text: &str) -> f32 {
        ui.text_width(text, &self.style(TextAlign::Left))
    }

    /// One line, vertically centred in `rect`.
    fn paint(&self, ui: &mut Ui, rect: Rect, text: &str, align: TextAlign) {
        if !text.is_empty() {
            ui.with_style(self.style(align), |ui| ui.label(rect, text));
        }
    }
}

/// A box with a fill and an optional border.
fn frame(ui: &mut Ui, rect: Rect, fill: Color, border: Option<Color>, radius: f32) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let style = Style {
        fill,
        stroke: border.unwrap_or(TRANSPARENT),
        stroke_width: if border.is_some() { 1.0 } else { 0.0 },
        radius,
        ..Style::default()
    };
    ui.with_style(style, |ui| ui.paint("panel", "", rect, 0.0, Response::default(), false));
}

fn fill(ui: &mut Ui, rect: Rect, color: Color) {
    frame(ui, rect, color, None, 0.0);
}

/// The toolbar's label: 11px, in the interface's sans serif.
fn label_text(look: &CodeEditorLook) -> Text {
    Text {
        size: 11.0,
        family: FontFamily::Named("Arial".into()),
        color: look.toolbar_text,
        line_height: 12.0,
    }
}

/// The picker's text: 12px.
fn picker_text(look: &CodeEditorLook) -> Text {
    Text {
        size: 12.0,
        family: FontFamily::SansSerif,
        color: look.picker_text,
        line_height: 15.0,
    }
}

/// The picker's padding across and down.
const PICKER_PAD: (f32, f32) = (4.0, 2.0);

fn picker_height(look: &CodeEditorLook) -> f32 {
    picker_text(look).line_height + 2.0 * (PICKER_PAD.1 + 1.0)
}

fn picker_width(ui: &Ui, look: &CodeEditorLook, names: &[&str]) -> f32 {
    let text = picker_text(look);
    let widest = names.iter().map(|n| text.width(ui, n)).fold(0.0, f32::max);
    widest.ceil() + 2.0 * PICKER_PAD.0 + 2.0
}

/// The language picker: a box showing the chosen language that opens a list
/// of them under it. Returns whether the choice changed.
fn picker(ui: &mut Ui, id: &str, rect: Rect, options: &[&str], selected: &mut usize, look: &CodeEditorLook) -> bool {
    let response = ui.interact(id, rect);
    if response.hovered {
        ui.set_cursor(CursorIcon::Pointer);
    }
    frame(ui, rect, look.picker, Some(look.picker_border), 2.0);
    let text = picker_text(look);
    let text_rect = Rect::new(
        rect.x + PICKER_PAD.0 + 1.0,
        rect.y,
        (rect.width - 2.0 * PICKER_PAD.0 - 2.0).max(0.0),
        rect.height,
    );
    text.paint(ui, text_rect, options.get(*selected).copied().unwrap_or_default(), TextAlign::Left);
    if response.clicked {
        ui.toggle_popup(id);
    }

    let row = text.size * 1.15 + 2.0;
    let size = Vec2::new(rect.width, row * options.len() as f32 + 2.0);
    let backing = Style {
        fill: look.menu,
        stroke: look.menu_border,
        stroke_width: 1.0,
        radius: 0.0,
        ..Style::default()
    };
    let saved = std::mem::replace(&mut ui.theme.popup, backing);
    let shadow = std::mem::replace(&mut ui.theme.palette.shadow, [0.0, 0.0, 0.0, 0.25]);
    let chosen = ui.popup(id, rect, size, Placement::Below, |ui, area| {
        let mut chosen = None;
        for (index, option) in options.iter().enumerate() {
            let row_rect = Rect::new(area.x + 1.0, area.y + 1.0 + index as f32 * row, area.width - 2.0, row);
            let response = ui.interact(&format!("{id}.option.{index}"), row_rect);
            let highlighted = response.hovered || (index == *selected && !ui.pointer_over(area));
            if highlighted {
                fill(ui, row_rect, look.menu_highlight);
            }
            let text = Text {
                color: if highlighted { look.menu_highlight_text } else { look.menu_text },
                ..picker_text(look)
            };
            text.paint(ui, Rect::new(row_rect.x + 4.0, row_rect.y, row_rect.width - 8.0, row), option, TextAlign::Left);
            if response.clicked {
                chosen = Some(index);
            }
        }
        chosen
    });
    ui.theme.popup = saved;
    ui.theme.palette.shadow = shadow;
    match chosen.flatten() {
        Some(index) => {
            ui.close_popup(id);
            let changed = index != *selected;
            *selected = index;
            changed
        }
        None => false,
    }
}

/// What the editor keeps between frames.
#[derive(Clone, Default)]
struct State {
    /// The language picked, over the one detected.
    language: Option<String>,
    /// The text last highlighted, in which language, and its colours.
    highlighted: Option<(String, String, Vec<(Range<usize>, Paint)>)>,
}

fn default_look() -> &'static CodeEditorLook {
    static LOOK: OnceLock<CodeEditorLook> = OnceLock::new();
    LOOK.get_or_init(CodeEditorLook::dark)
}

/// A code editor over a string.
#[derive(Clone, Copy, Default)]
pub struct CodeEditor<'a> {
    /// The text can be read and copied but not changed.
    pub readonly: bool,
    /// The language to assume when the picker has not chosen one and the
    /// code does not say.
    pub language: Option<&'a str>,
    /// How it looks; the dark look when none is given.
    pub look: Option<&'a CodeEditorLook>,
}

impl<'a> CodeEditor<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn language(mut self, language: &'a str) -> Self {
        self.language = Some(language);
        self
    }

    pub fn look(mut self, look: &'a CodeEditorLook) -> Self {
        self.look = Some(look);
        self
    }

    fn the_look(&self) -> &'a CodeEditorLook {
        match self.look {
            Some(look) => look,
            None => default_look(),
        }
    }

    /// The id of the text area inside the editor `id`.
    pub fn text_id(id: &str) -> String {
        format!("{id}.text")
    }

    /// How wide the editor is at least: its toolbar, its border.
    pub fn width(&self, ui: &Ui) -> f32 {
        let look = self.the_look();
        let names: Vec<&str> = LanguageRegistry::get().languages().iter().map(|l| l.name).collect();
        16.0 + label_text(look).width(ui, "Language:") + 8.0 + picker_width(ui, look, &names) + 2.0
    }

    /// Draw the editor into `rect`. Returns whether the text changed.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, text: &mut String) -> bool {
        let look = self.the_look();
        let mut state: State = ui.data(id).unwrap_or_default();
        let registry = LanguageRegistry::get();
        let language = state
            .language
            .clone()
            .or_else(|| registry.detect_from_code(text).map(|l| l.id.to_string()))
            .or_else(|| self.language.map(str::to_string))
            .unwrap_or_else(|| registry.default_language().id.to_string());

        frame(ui, rect, look.background, Some(look.border), 0.0);
        let inner = Rect::new(rect.x + 1.0, rect.y + 1.0, (rect.width - 2.0).max(0.0), (rect.height - 2.0).max(0.0));

        // The toolbar: the language picker.
        let label = label_text(look);
        let toolbar_height = label.line_height.max(picker_height(look)) + 8.0;
        let toolbar = Rect::new(inner.x, inner.y, inner.width, toolbar_height);
        let mut changed = false;
        ui.with_clip(inner, |ui| {
            fill(ui, toolbar, look.toolbar);
            fill(ui, Rect::new(toolbar.x, toolbar.bottom(), toolbar.width, 1.0), look.rule);
            let label_width = label.width(ui, "Language:");
            label.paint(ui, Rect::new(toolbar.x + 8.0, toolbar.y, label_width, toolbar.height), "Language:", TextAlign::Left);
            let languages = registry.languages();
            let names: Vec<&str> = languages.iter().map(|l| l.name).collect();
            let mut selected = languages.iter().position(|l| l.id == language).unwrap_or(0);
            let at = Rect::new(
                toolbar.x + 8.0 + label_width + 8.0,
                toolbar.y + 4.0,
                picker_width(ui, look, &names),
                picker_height(look),
            );
            if picker(ui, &format!("{id}.language"), at, &names, &mut selected, look) {
                state.language = Some(languages[selected].id.to_string());
            }

            // The editor: line numbers, then the text.
            let area = Rect::new(inner.x, toolbar.bottom() + 1.0, inner.width, (inner.bottom() - toolbar.bottom() - 1.0).max(0.0));
            let line_count = text.lines().count().max(1) + usize::from(text.ends_with('\n'));
            let digits = line_count.to_string().len() as f32;
            let gutter_width = (digits * 8.0 + 16.0).max(35.0);
            let gutter = Rect::new(area.x, area.y, gutter_width, area.height);
            let view = Rect::new(gutter.right(), area.y, (area.width - gutter_width).max(0.0), area.height);

            // The text is coloured again only when it changed.
            let cache = std::cell::RefCell::new(state.highlighted.take());
            let highlight = |value: &str| -> Vec<(Range<usize>, Paint)> {
                let mut cache = cache.borrow_mut();
                if let Some((last, lang, spans)) = &*cache {
                    if last == value && *lang == language {
                        return spans.clone();
                    }
                }
                let spans: Vec<(Range<usize>, Paint)> = HighlightEngine::new_with_language(&language)
                    .map(|engine| {
                        engine
                            .spans(value)
                            .into_iter()
                            .filter_map(|(range, scope)| look.paint(scope).map(|paint| (range, paint)))
                            .collect()
                    })
                    .unwrap_or_default();
                *cache = Some((value.to_string(), language.clone(), spans.clone()));
                spans
            };
            let code = Text {
                size: look.font_size,
                family: FontFamily::Monospace,
                color: look.foreground,
                line_height: look.line_height(),
            };
            let editor = TextArea {
                wrap: false,
                tab_spaces: Some(4),
                auto_indent: true,
                highlight: Some(&highlight),
                padding: (8.0, 8.0),
                line_height: code.line_height,
                selection: look.selection,
                caret: look.caret,
                readonly: self.readonly,
            };
            let response = ui.with_style(code.style(TextAlign::Left), |ui| editor.show(ui, &Self::text_id(id), view, text));
            changed = response.changed;
            state.highlighted = cache.into_inner();

            // The numbers scroll with the text.
            fill(ui, gutter, look.background);
            fill(ui, Rect::new(gutter.right() - 1.0, gutter.y, 1.0, gutter.height), look.rule);
            let numbers = Text {
                color: look.line_numbers,
                ..code
            };
            let line = numbers.line_height;
            ui.with_clip(gutter, |ui| {
                let first = (response.scroll.y / line).floor().max(0.0) as usize;
                let shown = (gutter.height / line).ceil() as usize + 1;
                for index in first..(first + shown).min(line_count) {
                    let y = gutter.y + 8.0 + index as f32 * line - response.scroll.y;
                    let row = Rect::new(gutter.x, y, gutter.width - 1.0 - 8.0, line);
                    numbers.paint(ui, row, &(index + 1).to_string(), TextAlign::Right);
                }
            });
        });
        ui.set_data(id, state);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_kit_immediate::Input;

    #[test]
    fn the_dark_look_colours_what_the_highlighter_finds() {
        let look = CodeEditorLook::dark();
        let keyword = look.paint(HighlightScope::Keyword).unwrap();
        assert_eq!(keyword.color, rgb(0xc586c0));
        assert!(keyword.bold);
        assert!(look.paint(HighlightScope::Comment).unwrap().italic);
        assert!(look.paint(HighlightScope::Default).is_none(), "plain text is the foreground");
    }

    #[test]
    fn the_editor_draws_and_keeps_its_text() {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        let mut text = String::from("fn main() {}\n");
        let changed = CodeEditor::new().language("wgsl").show(&mut ui, "editor", Rect::new(0.0, 0.0, 400.0, 300.0), &mut text);
        let commands = ui.end();
        assert!(!changed);
        assert_eq!(text, "fn main() {}\n");
        assert!(commands.iter().any(|c| c.text.contains("Language:")));
        assert!(CodeEditor::new().width(&Ui::default()) > 0.0);
    }
}
