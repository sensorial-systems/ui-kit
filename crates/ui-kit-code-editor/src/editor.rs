//! The editor: line numbers, syntax colours and a language picker over a
//! text area that keeps indentation and indents with spaces.

use crate::support::{rank, Completion, Completions, Diagnostic, LanguageSupport, Severity};
use crate::{HighlightEngine, HighlightScope, LanguageRegistry, Theme};
use std::cell::RefCell;
use std::ops::Range;
use std::sync::OnceLock;
use ui_kit_immediate::{
    Color, CursorIcon, FontFamily, Key, Paint, Placement, Rect, Response, Style, TextAlign, TextArea, TextNavigation,
    TextOverflow, Ui, Vec2,
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
    /// What a language's checker found: the line under an error, under a
    /// warning, and the strip along the bottom that names them.
    pub error: Color,
    pub warning: Color,
    pub status: Color,
    pub status_text: Color,
    /// A completion's word beside it, and the line about the chosen one.
    pub detail: Color,
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
            menu_highlight: rgb(0x04395e),
            menu_highlight_text: rgb(0xffffff),
            error: rgb(0xf14c4c),
            warning: rgb(0xcca700),
            status: rgb(0x252526),
            status_text: rgb(0x9d9d9d),
            detail: rgb(0x8b8b8b),
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
        Some(Paint { color, bold, italic, underline: None })
    }
}

impl Default for CodeEditorLook {
    fn default() -> Self {
        Self::dark()
    }
}

/// A run of text in one family, size and colour, with its line's height.
#[derive(Clone)]
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
    /// The completions open at the caret.
    completion: Option<Open>,
    /// Where the caret was, in characters: a completion list closes when it
    /// moves without the text changing.
    cursor: usize,
}

/// A completion list open at the caret.
#[derive(Clone, Default)]
struct Open {
    /// What the language offered, and those of them that match what was
    /// typed, best first.
    offered: Completions,
    shown: Vec<Completion>,
    selected: usize,
    /// The first row in view.
    first: usize,
}

/// How many completions show at once.
const COMPLETION_ROWS: usize = 10;

/// The byte offset of the character offset `index` of `text`.
fn byte_of(text: &str, index: usize) -> usize {
    text.char_indices().nth(index).map_or(text.len(), |(byte, _)| byte)
}

/// The character offset of the byte offset `byte` of `text`, or of the
/// character it falls inside.
fn char_of(text: &str, byte: usize) -> usize {
    text.char_indices().take_while(|(at, _)| *at < byte).count()
}

/// `range` if it lies on character boundaries of `text`, inside it.
fn sound(text: &str, range: &Range<usize>) -> Option<Range<usize>> {
    let ok = range.start < range.end && range.end <= text.len() && text.is_char_boundary(range.start) && text.is_char_boundary(range.end);
    ok.then(|| range.clone())
}

/// The colours of `text` with the problems' lines under them: the colours
/// split where a problem starts or ends, and plain text a problem covers
/// given the foreground. Ranges off the text's characters are dropped.
fn underlined(text: &str, colours: Vec<(Range<usize>, Paint)>, problems: &[(Range<usize>, Color)], foreground: Color) -> Vec<(Range<usize>, Paint)> {
    let mut colours: Vec<(Range<usize>, Paint)> = colours.into_iter().filter_map(|(r, p)| sound(text, &r).map(|r| (r, p))).collect();
    let problems: Vec<(Range<usize>, Color)> = problems.iter().filter_map(|(r, c)| sound(text, r).map(|r| (r, *c))).collect();
    if problems.is_empty() {
        return colours;
    }
    colours.sort_by_key(|(range, _)| range.start);
    // Every byte covered once: the colours, and plain text between them.
    let mut covered = Vec::new();
    let mut at = 0;
    for (range, paint) in colours {
        let start = range.start.max(at);
        if start >= range.end {
            continue;
        }
        if start > at {
            covered.push((at..start, Paint::from(foreground)));
        }
        covered.push((start..range.end, paint));
        at = range.end;
    }
    if at < text.len() {
        covered.push((at..text.len(), Paint::from(foreground)));
    }
    let mut out = Vec::new();
    for (range, paint) in covered {
        let mut cuts = vec![range.start, range.end];
        for (problem, _) in &problems {
            for cut in [problem.start, problem.end] {
                if cut > range.start && cut < range.end {
                    cuts.push(cut);
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let piece = pair[0]..pair[1];
            // The first problem over it wins: errors are listed first.
            let under = problems.iter().find(|(p, _)| p.start < piece.end && p.end > piece.start).map(|(_, c)| *c);
            out.push((piece, Paint { underline: under, ..paint }));
        }
    }
    out
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
    /// A language the editor knows more of: its colours replace the syntax
    /// colours and its name the language picker, and it finds problems --
    /// underlined, and named along the bottom -- and offers completions.
    pub support: Option<&'a dyn LanguageSupport>,
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

    pub fn support(mut self, support: &'a dyn LanguageSupport) -> Self {
        self.support = Some(support);
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

    /// Where the caret of the editor `id` was left, in characters.
    pub fn cursor(ui: &Ui, id: &str) -> usize {
        ui.data::<State>(id).map_or(0, |state| state.cursor)
    }

    /// The id of the completion list of the editor `id`.
    pub fn completion_id(id: &str) -> String {
        format!("{id}.completion")
    }

    /// How wide the editor is at least: its toolbar, its border.
    pub fn width(&self, ui: &Ui) -> f32 {
        let look = self.the_look();
        if let Some(support) = self.support {
            return 16.0 + label_text(look).width(ui, support.name()) + 2.0;
        }
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

        // The toolbar: the language's name, or the language picker.
        let label = label_text(look);
        let toolbar_height = label.line_height.max(picker_height(look)) + 8.0;
        let toolbar = Rect::new(inner.x, inner.y, inner.width, toolbar_height);
        // The strip that names the problems found, under the code.
        let status_height = if self.support.is_some() { label.line_height + 10.0 } else { 0.0 };
        let mut changed = false;
        ui.with_clip(inner, |ui| {
            fill(ui, toolbar, look.toolbar);
            fill(ui, Rect::new(toolbar.x, toolbar.bottom(), toolbar.width, 1.0), look.rule);
            match self.support {
                Some(support) => {
                    let at = Rect::new(toolbar.x + 8.0, toolbar.y, (toolbar.width - 16.0).max(0.0), toolbar.height);
                    label.paint(ui, at, support.name(), TextAlign::Left);
                }
                None => {
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
                }
            }

            // The editor: line numbers, then the text.
            let area = Rect::new(
                inner.x,
                toolbar.bottom() + 1.0,
                inner.width,
                (inner.bottom() - status_height - toolbar.bottom() - 1.0).max(0.0),
            );
            let line_count = text.lines().count().max(1) + usize::from(text.ends_with('\n'));
            let digits = line_count.to_string().len() as f32;
            let gutter_width = (digits * 8.0 + 16.0).max(35.0);
            let gutter = Rect::new(area.x, area.y, gutter_width, area.height);
            let view = Rect::new(gutter.right(), area.y, (area.width - gutter_width).max(0.0), area.height);

            // The text is coloured again only when it changed -- unless a
            // language colours it, which keeps what it found itself. The
            // problems found are kept for the strip along the bottom.
            let problems = RefCell::new(Vec::<Diagnostic>::new());
            let cache = RefCell::new(state.highlighted.take());
            let highlight = |value: &str| -> Vec<(Range<usize>, Paint)> {
                if let Some(support) = self.support {
                    let colours = support
                        .highlight(value)
                        .into_iter()
                        .filter_map(|(range, scope)| look.paint(scope).map(|paint| (range, paint)))
                        .collect();
                    let mut found = support.diagnostics(value);
                    found.sort_by_key(|d| d.severity != Severity::Error);
                    let lines: Vec<(Range<usize>, Color)> = found
                        .iter()
                        .map(|d| {
                            let colour = if d.severity == Severity::Error { look.error } else { look.warning };
                            (visible(value, &d.range), colour)
                        })
                        .collect();
                    *problems.borrow_mut() = found;
                    return underlined(value, colours, &lines, look.foreground);
                }
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

            // With completions open, the keys that choose among them are
            // theirs: the text area does not see them.
            let text_id = Self::text_id(id);
            let popup_id = Self::completion_id(id);
            let focused = ui.focused(&text_id);
            let completing = self.support.filter(|_| focused && !self.readonly);
            if state.completion.is_some() && (completing.is_none() || !ui.is_popup_open(&popup_id)) {
                state.completion = None;
                ui.close_popup(&popup_id);
            }
            let input = ui.input().clone();
            let open = state.completion.is_some();
            let invoke = completing.is_some() && input.modifiers.command() && input.keys.contains(&Key::Space);
            let mut step = 0i32;
            let mut accept = None;
            if let Some(list) = &state.completion {
                match input.navigation {
                    Some(TextNavigation::Up) => step = -1,
                    Some(TextNavigation::Down) => step = 1,
                    _ => {}
                }
                if (input.activate && input.keys.contains(&Key::Enter)) || (input.tab && ui.consume_key(Key::Tab)) {
                    accept = Some(list.selected);
                }
                if ui.consume_key(Key::Escape) {
                    state.completion = None;
                    ui.close_popup(&popup_id);
                }
            }
            let response = ui.with_style(code.style(TextAlign::Left), |ui| {
                ui.with_input(
                    |input| {
                        if open {
                            if matches!(input.navigation, Some(TextNavigation::Up | TextNavigation::Down)) {
                                input.navigation = None;
                            }
                            input.activate = false;
                            input.tab = false;
                            input.keys.retain(|k| !matches!(k, Key::Enter | Key::Tab | Key::Up | Key::Down | Key::Escape));
                        }
                        if invoke {
                            input.text.clear();
                        }
                    },
                    |ui| editor.show(ui, &text_id, view, text),
                )
            });
            changed = response.changed;
            state.highlighted = cache.into_inner();
            let mut cursor = response.cursor;

            if let Some(support) = completing {
                let mut ask = None;
                if let Some(list) = state.completion.as_mut().filter(|_| step != 0) {
                    let count = list.shown.len().max(1) as i32;
                    list.selected = (list.selected as i32 + step).rem_euclid(count) as usize;
                }
                if let Some(index) = accept {
                    if let Some(list) = state.completion.take() {
                        ui.close_popup(&popup_id);
                        if let Some(item) = list.shown.get(index) {
                            cursor = complete_with(ui, &text_id, text, &list.offered, item, cursor);
                            changed = true;
                            ask = after_trigger(support, text, cursor).then_some(false);
                        }
                    }
                } else if invoke {
                    ask = Some(true);
                } else if response.changed {
                    let typed = input.text.chars().last();
                    if let Some(c) = typed {
                        if c.is_alphanumeric() || c == '_' || support.triggers().contains(&c) {
                            ask = Some(false);
                        }
                    } else if (input.backspace || input.delete) && open {
                        ask = Some(false);
                    }
                    if ask.is_none() {
                        state.completion = None;
                        ui.close_popup(&popup_id);
                    }
                } else if state.completion.is_some() && cursor != state.cursor {
                    state.completion = None;
                    ui.close_popup(&popup_id);
                }
                if let Some(explicit) = ask {
                    let previous = state.completion.take();
                    state.completion = offer(support, text, cursor, explicit, previous);
                    if state.completion.is_some() {
                        ui.open_popup(&popup_id);
                    } else {
                        ui.close_popup(&popup_id);
                    }
                }

                // The list, under the caret; a click on one takes it.
                let anchor = Rect::new(response.caret.x, response.caret.y, 1.0, response.caret.height);
                if let Some(list) = state.completion.as_mut() {
                    match completion_list(ui, &popup_id, anchor, list, look) {
                        None => state.completion = None,
                        Some(Some(index)) => {
                            let list = state.completion.take().unwrap_or_default();
                            ui.close_popup(&popup_id);
                            if let Some(item) = list.shown.get(index) {
                                cursor = complete_with(ui, &text_id, text, &list.offered, item, cursor);
                                changed = true;
                            }
                            ui.request_focus(&text_id);
                        }
                        Some(None) => {}
                    }
                }
            }
            state.cursor = cursor;

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

            // The problem at the caret, or the first, along the bottom.
            if self.support.is_some() {
                let strip = Rect::new(inner.x, inner.bottom() - status_height, inner.width, status_height);
                fill(ui, strip, look.status);
                fill(ui, Rect::new(strip.x, strip.y, strip.width, 1.0), look.rule);
                let found = problems.borrow();
                let at = byte_of(text, cursor);
                let here = found
                    .iter()
                    .find(|d| d.range.start <= at && at <= d.range.end)
                    .or_else(|| found.first());
                let room = Rect::new(strip.x + 8.0, strip.y + 1.0, (strip.width - 16.0).max(0.0), strip.height - 1.0);
                let count = match found.len() {
                    0 => String::new(),
                    1 => "1 problem".to_string(),
                    n => format!("{n} problems"),
                };
                let count_text = Text { color: look.status_text, ..label_text(look) };
                let count_width = count_text.width(ui, &count);
                count_text.paint(ui, room, &count, TextAlign::Right);
                let message_room = Rect::new(room.x, room.y, (room.width - count_width - 12.0).max(0.0), room.height);
                match here {
                    Some(problem) => {
                        let line = text[..problem.range.start.min(text.len())].matches('\n').count() + 1;
                        let colour = if problem.severity == Severity::Error { look.error } else { look.warning };
                        let message = Text { color: colour, ..label_text(look) };
                        message.paint(ui, message_room, &format!("Ln {line}: {}", problem.message), TextAlign::Left);
                    }
                    None => count_text.paint(ui, message_room, "No problems", TextAlign::Left),
                }
            }
        });
        ui.set_data(id, state);
        changed
    }
}

/// A problem's range, widened to a character when it is empty, so its line
/// shows.
fn visible(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = range.start.min(text.len());
    let end = range.end.min(text.len()).max(start);
    if start < end {
        return start..end;
    }
    let next = text[start..].chars().next().filter(|c| *c != '\n').map(|c| start + c.len_utf8());
    let previous = text[..start].chars().next_back().filter(|c| *c != '\n').map(|c| start - c.len_utf8());
    match (next, previous) {
        (Some(next), _) => start..next,
        (None, Some(previous)) => previous..start,
        _ => start..end,
    }
}

/// Put `item` in place of what it completes, up to and past the caret at
/// `cursor`; where the caret goes, in characters.
fn complete_with(ui: &mut Ui, text_id: &str, text: &mut String, offered: &Completions, item: &Completion, cursor: usize) -> usize {
    let start = char_of(text, offered.replace.start).min(cursor);
    let end = char_of(text, offered.replace.end).max(cursor);
    TextArea::replace(ui, text_id, text, start..end, &item.insert, item.caret);
    let inserted = item.insert.chars().count();
    start + item.caret.unwrap_or(inserted).min(inserted)
}

/// Whether the character before the caret opens completions.
fn after_trigger(support: &dyn LanguageSupport, text: &str, cursor: usize) -> bool {
    cursor > 0 && text.chars().nth(cursor - 1).is_some_and(|c| support.triggers().contains(&c))
}

/// What the language offers at `cursor`, matched against what was typed,
/// with `previous`'s choice kept if it is still there; nothing when nothing
/// matches, or the one match is what was typed.
fn offer(support: &dyn LanguageSupport, text: &str, cursor: usize, explicit: bool, previous: Option<Open>) -> Option<Open> {
    let at = byte_of(text, cursor);
    let offered = support.complete(text, at, explicit)?;
    let start = offered.replace.start.min(at);
    let typed = if text.is_char_boundary(start) { &text[start..at] } else { "" };
    if typed.is_empty() && !explicit && !after_trigger(support, text, cursor) {
        return None;
    }
    let shown = rank(&offered.items, typed);
    let done = !explicit && shown.len() == 1 && shown[0].insert == typed;
    if shown.is_empty() || done {
        return None;
    }
    let selected = previous
        .and_then(|p| p.shown.get(p.selected).map(|c| c.label.clone()))
        .and_then(|label| shown.iter().position(|c| c.label == label))
        .unwrap_or(0);
    Some(Open { offered, shown, selected, first: 0 })
}

/// The completion list under the caret. `None` once it closed, by a press
/// outside it or Escape; otherwise the completion clicked, if any.
fn completion_list(ui: &mut Ui, id: &str, anchor: Rect, list: &mut Open, look: &CodeEditorLook) -> Option<Option<usize>> {
    let row = (look.font_size * 1.65).round();
    let rows = list.shown.len().min(COMPLETION_ROWS);
    if list.selected < list.first {
        list.first = list.selected;
    }
    if list.selected >= list.first + rows {
        list.first = list.selected + 1 - rows;
    }
    let label = Text {
        size: look.font_size,
        family: FontFamily::Monospace,
        color: look.menu_text,
        line_height: row,
    };
    let detail = Text {
        size: look.font_size - 1.5,
        family: FontFamily::SansSerif,
        color: look.detail,
        line_height: row,
    };
    let widest = list
        .shown
        .iter()
        .map(|c| label.width(ui, &c.label) + c.detail.as_deref().map_or(0.0, |d| detail.width(ui, d) + 24.0))
        .fold(0.0, f32::max);
    let width = (widest + 20.0).clamp(220.0, 560.0);
    let documentation = list.shown.get(list.selected).and_then(|c| c.documentation.clone());
    let documentation_height = if documentation.is_some() { row + 6.0 } else { 0.0 };
    let size = Vec2::new(width, rows as f32 * row + 2.0 + documentation_height);
    let backing = Style {
        fill: look.menu,
        stroke: look.menu_border,
        stroke_width: 1.0,
        radius: 3.0,
        ..Style::default()
    };
    let saved = std::mem::replace(&mut ui.theme.popup, backing);
    let shadow = std::mem::replace(&mut ui.theme.palette.shadow, [0.0, 0.0, 0.0, 0.35]);
    let result = ui.popup(id, anchor, size, Placement::Below, |ui, area| {
        let wheel = ui.take_scroll(area);
        if wheel.y != 0.0 {
            let last = list.shown.len().saturating_sub(rows);
            list.first = if wheel.y > 0.0 { (list.first + 1).min(last) } else { list.first.saturating_sub(1) };
        }
        let mut chosen = None;
        for slot in 0..rows {
            let index = list.first + slot;
            let Some(item) = list.shown.get(index) else { break };
            let at = Rect::new(area.x + 1.0, area.y + 1.0 + slot as f32 * row, area.width - 2.0, row);
            let row_id = format!("{id}.{slot}");
            let response = ui.interact(&row_id, at);
            ui.untabbable(&row_id);
            if response.hovered {
                ui.set_cursor(CursorIcon::Pointer);
            }
            if index == list.selected {
                fill(ui, at, look.menu_highlight);
            }
            let colour = look.paint(item.kind).map_or(look.menu_text, |p| p.color);
            let text = Text { color: colour, ..label.clone() };
            text.paint(ui, Rect::new(at.x + 8.0, at.y, at.width - 16.0, row), &item.label, TextAlign::Left);
            if let Some(word) = &item.detail {
                detail.paint(ui, Rect::new(at.x + 8.0, at.y, at.width - 16.0, row), word, TextAlign::Right);
            }
            if response.clicked {
                chosen = Some(index);
            }
        }
        if let Some(line) = &documentation {
            let top = area.y + 1.0 + rows as f32 * row;
            fill(ui, Rect::new(area.x + 1.0, top + 2.0, area.width - 2.0, 1.0), look.menu_border);
            let note = Text { color: look.detail, ..detail.clone() };
            note.paint(ui, Rect::new(area.x + 8.0, top + 4.0, area.width - 16.0, row), line, TextAlign::Left);
        }
        chosen
    });
    ui.theme.popup = saved;
    ui.theme.palette.shadow = shadow;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_kit_immediate::Input;

    /// Node names after `= `, and a problem wherever `bad` is written.
    struct Nodes;

    impl LanguageSupport for Nodes {
        fn name(&self) -> &str {
            "Nodes"
        }

        fn diagnostics(&self, text: &str) -> Vec<Diagnostic> {
            text.match_indices("bad").map(|(at, _)| Diagnostic::error(at..at + 3, "bad is bad")).collect()
        }

        fn complete(&self, text: &str, cursor: usize, _explicit: bool) -> Option<Completions> {
            let start = text[..cursor].rfind(|c: char| !(c.is_alphanumeric() || c == '.')).map_or(0, |i| i + 1);
            let items = ["Number", "Number.Add", "Vector"]
                .into_iter()
                .map(|name| Completion::new(name, HighlightScope::Type).insert(format!("{name}()"), Some(name.len() + 1)))
                .collect();
            Some(Completions { replace: start..cursor, items })
        }
    }

    fn frame(ui: &mut Ui, text: &mut String, input: Input) -> bool {
        ui.begin(input);
        let changed = CodeEditor::new().support(&Nodes).show(ui, "editor", Rect::new(0.0, 0.0, 400.0, 300.0), text);
        ui.end();
        changed
    }

    fn typed(text: &str) -> Input {
        Input { text: text.into(), ..Input::default() }
    }

    fn focused() -> (Ui, String) {
        let mut ui = Ui::default();
        ui.set_text_measure(|text, _| text.chars().count() as f32 * 8.0);
        let mut text = String::new();
        frame(&mut ui, &mut text, Input::default());
        ui.request_focus(&CodeEditor::text_id("editor"));
        frame(&mut ui, &mut text, Input::default());
        (ui, text)
    }

    #[test]
    fn typing_opens_completions_and_enter_takes_the_chosen_one() {
        let (mut ui, mut text) = focused();
        frame(&mut ui, &mut text, typed("x = "));
        assert!(!ui.is_popup_open(&CodeEditor::completion_id("editor")), "a space opens nothing");
        frame(&mut ui, &mut text, typed("N"));
        assert!(ui.is_popup_open(&CodeEditor::completion_id("editor")));
        // "Number", then "Number.Add": Vector does not match.
        frame(&mut ui, &mut text, Input { navigation: Some(TextNavigation::Down), keys: vec![Key::Down], ..Input::default() });
        assert_eq!(text, "x = N", "Down chose, it did not move the caret");
        let changed = frame(&mut ui, &mut text, Input { activate: true, keys: vec![Key::Enter], ..Input::default() });
        assert!(changed);
        assert_eq!(text, "x = Number.Add()", "Enter took the completion, not a line break");
        frame(&mut ui, &mut text, typed("1"));
        assert_eq!(text, "x = Number.Add(1)", "the caret went inside the parentheses");
    }

    #[test]
    fn escape_closes_completions_and_ctrl_space_opens_them() {
        let (mut ui, mut text) = focused();
        frame(&mut ui, &mut text, typed("Vec"));
        let popup = CodeEditor::completion_id("editor");
        assert!(ui.is_popup_open(&popup));
        frame(&mut ui, &mut text, Input { keys: vec![Key::Escape], ..Input::default() });
        assert!(!ui.is_popup_open(&popup));
        let control = ui_kit_immediate::Modifiers { control: true, ..Default::default() };
        frame(&mut ui, &mut text, Input { keys: vec![Key::Space], modifiers: control, text: " ".into(), ..Input::default() });
        assert!(ui.is_popup_open(&popup));
        assert_eq!(text, "Vec", "Ctrl+Space typed no space");
        frame(&mut ui, &mut text, Input { tab: true, keys: vec![Key::Tab], ..Input::default() });
        assert_eq!(text, "Vector()");
    }

    #[test]
    fn problems_are_underlined_and_named_along_the_bottom() {
        let (mut ui, mut text) = focused();
        text.push_str("ok
bad");
        ui.begin(Input::default());
        CodeEditor::new().support(&Nodes).show(&mut ui, "editor", Rect::new(0.0, 0.0, 400.0, 300.0), &mut text);
        let commands = ui.end();
        assert!(commands.iter().any(|c| c.text.contains("Ln 2: bad is bad")));
        assert!(commands.iter().any(|c| c.text == "1 problem"));
        let colours = underlined("ok bad", vec![(0..2, Paint::from([1.0; 4]))], &[(3..6, [1.0, 0.0, 0.0, 1.0])], [0.5; 4]);
        let under: Vec<_> = colours.iter().filter(|(_, p)| p.underline.is_some()).map(|(r, _)| r.clone()).collect();
        assert_eq!(under, vec![3..6]);
    }

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
