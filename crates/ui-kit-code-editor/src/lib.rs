//! A code editor for the immediate UI, and what it is built on: a rope text
//! buffer, the languages it knows, syntax highlighting and colour themes.
//!
//! [`CodeEditor`] draws line numbers, syntax colours and a language picker
//! over a text area that keeps indentation and indents with spaces. How it
//! looks is a [`CodeEditorLook`]; the default is a dark one.

mod code;
mod editor;
mod highlight;
pub mod language;
mod text_buffer;
mod theme;

pub use code::Code;
pub use editor::{CodeEditor, CodeEditorLook, MIN_HEIGHT};
pub use highlight::{HighlightEngine, HighlightSpan};
pub use language::*;
pub use text_buffer::TextBuffer;
pub use theme::{Color, FontStyle, HighlightScope, Style, Theme};
