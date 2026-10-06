use crate::TextBuffer;
use crate::theme::HighlightScope;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

/// Represents a highlighted span of text with a semantic scope.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightSpan {
    pub start: usize, // byte offset
    pub end: usize,   // byte offset
    pub scope: HighlightScope,
}

impl HighlightSpan {
    pub fn new(start: usize, end: usize, scope: HighlightScope) -> Self {
        Self { start, end, scope }
    }
}

/// The highlighting engine using syntect (WASM-compatible).
///
/// Syntect is a pure Rust syntax highlighting library that works in WASM.
/// It uses TextMate grammars and themes for accurate highlighting.
#[derive(Clone)]
pub struct HighlightEngine {
    syntax_set: &'static SyntaxSet,
    theme: &'static Theme,
    syntax: &'static SyntaxReference,
}

impl HighlightEngine {
    /// Creates a new highlighting engine for Rust code.
    pub fn new_rust() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_language("rs")
    }

    /// Creates a new highlighting engine for a specific language.
    pub fn new_with_language(lang_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let syntax_set = SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines());
        let theme_set = THEME_SET.get_or_init(|| ThemeSet::load_defaults());

        let theme = &theme_set.themes["base16-ocean.dark"];

        let registry = crate::language::LanguageRegistry::get();
        let lang_def = registry.find_by_id(lang_id);

        let syntax = if let Some(def) = lang_def {
            syntax_set
                .find_syntax_by_extension(def.id)
                .or_else(|| syntax_set.find_syntax_by_extension(def.generic_extension))
        } else {
            syntax_set.find_syntax_by_extension(lang_id)
        }
        .ok_or_else(|| format!("Syntax for '{}' not found", lang_id))?;

        Ok(Self {
            syntax_set,
            theme,
            syntax,
        })
    }

    /// Parses the text buffer and generates a syntax tree.
    /// Note: Syntect doesn't maintain a persistent tree, so this is a no-op.
    pub fn parse(&mut self, _buffer: &TextBuffer) -> Result<(), Box<dyn std::error::Error>> {
        // Syntect doesn't require pre-parsing
        Ok(())
    }

    /// Applies an incremental edit to the syntax tree.
    /// Note: Syntect re-highlights on demand, so this is a no-op.
    pub fn edit(
        &mut self,
        _buffer: &TextBuffer,
        _start_byte: usize,
        _old_end_byte: usize,
        _new_end_byte: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Syntect doesn't maintain state, so no incremental updates needed
        Ok(())
    }

    /// Generates highlight spans for the entire buffer.
    pub fn highlight(
        &mut self,
        buffer: &TextBuffer,
    ) -> Result<Vec<HighlightSpan>, Box<dyn std::error::Error>> {
        let text = buffer.to_string();
        let mut highlighter = HighlightLines::new(self.syntax, self.theme);
        let mut spans = Vec::new();
        let mut byte_offset = 0;

        for line in LinesWithEndings::from(&text) {
            let line_highlights = highlighter.highlight_line(line, self.syntax_set)?;

            for (style, text_segment) in line_highlights {
                let start = byte_offset;
                let end = byte_offset + text_segment.len();

                // Convert syntect style to our semantic scope
                let scope = style_to_scope(&style);

                if start < end {
                    spans.push(HighlightSpan::new(start, end, scope));
                }

                byte_offset = end;
            }
        }

        Ok(spans)
    }

    /// Generates highlight spans for a specific line.
    pub fn highlight_line(
        &mut self,
        buffer: &TextBuffer,
        line_idx: usize,
    ) -> Result<Vec<HighlightSpan>, Box<dyn std::error::Error>> {
        let line_text = buffer.line(line_idx).ok_or("Line not found")?;
        let line_start = buffer.line_to_byte(line_idx).ok_or("Invalid line index")?;

        let mut highlighter = HighlightLines::new(self.syntax, self.theme);
        let line_highlights = highlighter.highlight_line(&line_text, self.syntax_set)?;

        let mut spans = Vec::new();
        let mut byte_offset = line_start;

        for (style, text_segment) in line_highlights {
            let start = byte_offset;
            let end = byte_offset + text_segment.len();

            let scope = style_to_scope(&style);

            if start < end {
                spans.push(HighlightSpan::new(start, end, scope));
            }

            byte_offset = end;
        }

        Ok(spans)
    }

    /// The scope of each styled run of `code`, by byte range.
    pub fn spans(&self, code: &str) -> Vec<(std::ops::Range<usize>, HighlightScope)> {
        let mut highlighter = HighlightLines::new(self.syntax, self.theme);
        let mut spans = Vec::new();
        let mut offset = 0;
        for line in LinesWithEndings::from(code) {
            if let Ok(ranges) = highlighter.highlight_line(line, self.syntax_set) {
                for (style, text) in ranges {
                    spans.push((offset..offset + text.len(), style_to_scope(&style)));
                    offset += text.len();
                }
            } else {
                offset += line.len();
            }
        }
        spans
    }

    /// Highlights the code and returns HTML with classes.
    pub fn highlight_to_html(&self, code: &str) -> String {
        let mut highlighter = HighlightLines::new(self.syntax, self.theme);
        let mut html = String::new();

        // Setup initial colors/style for the container if needed, but here we just return spans

        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, self.syntax_set) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        let scope = style_to_scope(&style);
                        let class_name = match scope {
                            HighlightScope::Keyword => "keyword",
                            HighlightScope::String => "string",
                            HighlightScope::Comment => "comment",
                            HighlightScope::Number => "number",
                            HighlightScope::Function => "function",
                            HighlightScope::Type => "type",
                            // Add other scopes if you have CSS classes for them
                            _ => "",
                        };

                        let escaped_text = Self::html_escape(text);

                        if class_name.is_empty() {
                            html.push_str(&escaped_text);
                        } else {
                            html.push_str(&format!(
                                "<span class=\"{}\">{}</span>",
                                class_name, escaped_text
                            ));
                        }
                    }
                }
                Err(_) => {
                    html.push_str(&Self::html_escape(line));
                }
            }
        }

        html
    }

    /// Escapes HTML special characters in a string.
    pub fn html_escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

/// Converts a syntect Style to our semantic HighlightScope.
/// This is a heuristic based on the foreground color and font style.
fn style_to_scope(style: &SyntectStyle) -> HighlightScope {
    // Syntect doesn't provide semantic scopes directly, so we use color heuristics

    let fg = style.foreground;

    // Keywords are often bold or a specific color (Purple in base16-ocean: ~180, 142, 173)
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        return HighlightScope::Keyword;
    }
    if fg.r > 160 && fg.b > 160 && fg.g < 160 {
        return HighlightScope::Keyword;
    }

    // Strings are often a specific color (reddish/orange)
    if fg.r > 180 && fg.g < 150 && fg.b < 150 {
        return HighlightScope::String;
    }

    // Comments are often italic or gray
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        return HighlightScope::Comment;
    }

    // Numbers are often a specific color (greenish)
    if fg.g > 180 && fg.r < 150 && fg.b < 200 {
        return HighlightScope::Number;
    }

    // Functions are often yellowish or blueish (Blue in base16-ocean: ~143, 161, 179)
    if (fg.r > 200 && fg.g > 200 && fg.b < 180) || (fg.b > 160 && fg.r < 160 && fg.g > 130) {
        return HighlightScope::Function;
    }

    // Types are often cyan/blue
    if fg.b > 180 && fg.r < 150 && fg.g > 150 {
        return HighlightScope::Type;
    }

    HighlightScope::Default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_engine_creation() {
        let engine = HighlightEngine::new_rust();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_basic_highlighting() {
        let mut engine = HighlightEngine::new_rust().unwrap();
        let buffer = TextBuffer::from_text("fn main() {\n    println!(\"Hello\");\n}");

        engine.parse(&buffer).unwrap();
        let spans = engine.highlight(&buffer).unwrap();

        // Syntect should return highlights
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_to_html() {
        let engine = HighlightEngine::new_rust().unwrap();
        let html = engine.highlight_to_html("fn main()");
        println!("Generated HTML: {}", html);
        assert!(
            html.contains("keyword"),
            "HTML does not contain 'keyword'. Got: {}",
            html
        );
    }
}
