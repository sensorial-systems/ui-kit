use serde::{Deserialize, Serialize};

/// Represents a color in RGB format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Self { r, g, b })
    }

    pub fn to_css(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

/// Font style for text rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Bold,
    Italic,
    BoldItalic,
}

/// A style definition for syntax highlighting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub fg: Color,
    pub bg: Option<Color>,
    pub font_style: FontStyle,
}

impl Style {
    pub fn new(fg: Color) -> Self {
        Self {
            fg,
            bg: None,
            font_style: FontStyle::Normal,
        }
    }

    pub fn with_bg(mut self, bg: Color) -> Self {
        self.bg = Some(bg);
        self
    }

    pub fn with_font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = font_style;
        self
    }
}

/// Semantic highlight scopes matching Tree-sitter highlight names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HighlightScope {
    Keyword,
    Function,
    Type,
    Variable,
    Property,
    Constant,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Label,
    Attribute,
    Constructor,
    Namespace,
    Default,
}

impl HighlightScope {
    /// Maps Tree-sitter highlight names to semantic scopes.
    pub fn from_tree_sitter_name(name: &str) -> Self {
        match name {
            "keyword" => Self::Keyword,
            "function" | "function.method" => Self::Function,
            "type" | "type.builtin" => Self::Type,
            "variable" | "variable.parameter" | "variable.builtin" => Self::Variable,
            "property" => Self::Property,
            "constant" | "constant.builtin" => Self::Constant,
            "string" | "string.special" => Self::String,
            "number" => Self::Number,
            "comment" => Self::Comment,
            "operator" => Self::Operator,
            "punctuation" | "punctuation.bracket" | "punctuation.delimiter" => Self::Punctuation,
            "label" => Self::Label,
            "attribute" => Self::Attribute,
            "constructor" => Self::Constructor,
            "namespace" => Self::Namespace,
            _ => Self::Default,
        }
    }
}

/// A theme mapping highlight scopes to styles.
#[derive(Clone, Debug)]
pub struct Theme {
    styles: std::collections::HashMap<HighlightScope, Style>,
    default_style: Style,
}

impl Theme {
    pub fn new(default_style: Style) -> Self {
        Self {
            styles: std::collections::HashMap::new(),
            default_style,
        }
    }

    pub fn set_style(&mut self, scope: HighlightScope, style: Style) {
        self.styles.insert(scope, style);
    }

    pub fn get_style(&self, scope: HighlightScope) -> &Style {
        self.styles.get(&scope).unwrap_or(&self.default_style)
    }

    /// The style the theme gives `scope` itself, if it gives it one.
    pub fn styled(&self, scope: HighlightScope) -> Option<&Style> {
        self.styles.get(&scope)
    }

    /// Creates a default dark theme.
    pub fn dark() -> Self {
        let mut theme = Self::new(Style::new(Color::new(204, 204, 204)));

        theme.set_style(
            HighlightScope::Keyword,
            Style::new(Color::new(197, 134, 192)).with_font_style(FontStyle::Bold),
        );
        theme.set_style(
            HighlightScope::Function,
            Style::new(Color::new(220, 220, 170)),
        );
        theme.set_style(HighlightScope::Type, Style::new(Color::new(78, 201, 176)));
        theme.set_style(
            HighlightScope::Variable,
            Style::new(Color::new(156, 220, 254)),
        );
        theme.set_style(
            HighlightScope::Property,
            Style::new(Color::new(156, 220, 254)),
        );
        theme.set_style(
            HighlightScope::Constant,
            Style::new(Color::new(79, 193, 255)),
        );
        theme.set_style(
            HighlightScope::String,
            Style::new(Color::new(206, 145, 120)),
        );
        theme.set_style(
            HighlightScope::Number,
            Style::new(Color::new(181, 206, 168)),
        );
        theme.set_style(
            HighlightScope::Comment,
            Style::new(Color::new(106, 153, 85)).with_font_style(FontStyle::Italic),
        );
        theme.set_style(
            HighlightScope::Operator,
            Style::new(Color::new(204, 204, 204)),
        );
        theme.set_style(
            HighlightScope::Punctuation,
            Style::new(Color::new(204, 204, 204)),
        );

        theme
    }

    /// Creates a default light theme.
    pub fn light() -> Self {
        let mut theme = Self::new(Style::new(Color::new(0, 0, 0)));

        theme.set_style(
            HighlightScope::Keyword,
            Style::new(Color::new(0, 0, 255)).with_font_style(FontStyle::Bold),
        );
        theme.set_style(
            HighlightScope::Function,
            Style::new(Color::new(121, 94, 38)),
        );
        theme.set_style(HighlightScope::Type, Style::new(Color::new(38, 127, 153)));
        theme.set_style(HighlightScope::Variable, Style::new(Color::new(0, 16, 128)));
        theme.set_style(HighlightScope::String, Style::new(Color::new(163, 21, 21)));
        theme.set_style(HighlightScope::Number, Style::new(Color::new(9, 134, 88)));
        theme.set_style(
            HighlightScope::Comment,
            Style::new(Color::new(0, 128, 0)).with_font_style(FontStyle::Italic),
        );

        theme
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF0000").unwrap();
        assert_eq!(color, Color::new(255, 0, 0));
    }

    #[test]
    fn test_color_to_css() {
        let color = Color::new(255, 128, 64);
        assert_eq!(color.to_css(), "rgb(255, 128, 64)");
    }

    #[test]
    fn test_theme_scope_resolution() {
        let theme = Theme::dark();
        let style = theme.get_style(HighlightScope::Keyword);
        assert_eq!(style.font_style, FontStyle::Bold);
    }

    #[test]
    fn test_highlight_scope_mapping() {
        assert_eq!(
            HighlightScope::from_tree_sitter_name("keyword"),
            HighlightScope::Keyword
        );
        assert_eq!(
            HighlightScope::from_tree_sitter_name("function.method"),
            HighlightScope::Function
        );
        assert_eq!(
            HighlightScope::from_tree_sitter_name("unknown"),
            HighlightScope::Default
        );
    }
}
