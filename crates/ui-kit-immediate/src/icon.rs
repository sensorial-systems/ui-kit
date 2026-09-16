//! A small set of line icons, drawn from SVG so they scale cleanly and never
//! depend on what glyphs a font has.
use crate::svg::Svg;
use crate::Ui;
use std::sync::OnceLock;
use ui_kit_core::{Color, Rect};

/// A built-in icon. Each is a 24×24 SVG drawn with 1.5-unit round strokes, and
/// paints in whatever colour it is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    ArrowUp,
    ArrowDown,
    ArrowUpDown,
    /// Enter / submit.
    CornerDownLeft,
    Close,
    Plus,
    Minus,
    Check,
    Calendar,
    /// An open ring, for spinners and loading states.
    Loader,
    /// A small filled dot, for legends and status.
    Dot,
    Grip,
}

impl Icon {
    pub const ALL: [Icon; 16] = [
        Icon::ChevronDown,
        Icon::ChevronUp,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::ArrowUpDown,
        Icon::CornerDownLeft,
        Icon::Close,
        Icon::Plus,
        Icon::Minus,
        Icon::Check,
        Icon::Calendar,
        Icon::Loader,
        Icon::Dot,
        Icon::Grip,
    ];

    /// The icon's SVG source.
    pub fn source(self) -> String {
        let body = match self {
            Icon::ChevronDown => r#"<path d="M6 9l6 6 6-6"/>"#,
            Icon::ChevronUp => r#"<path d="M6 15l6-6 6 6"/>"#,
            Icon::ChevronLeft => r#"<path d="M15 6l-6 6 6 6"/>"#,
            Icon::ChevronRight => r#"<path d="M9 6l6 6-6 6"/>"#,
            Icon::ArrowUp => r#"<path d="M12 19V5M6 11l6-6 6 6"/>"#,
            Icon::ArrowDown => r#"<path d="M12 5v14M6 13l6 6 6-6"/>"#,
            Icon::ArrowUpDown => r#"<path d="M8 9l4-4 4 4M8 15l4 4 4-4"/>"#,
            Icon::CornerDownLeft => r#"<path d="M20 5v6a4 4 0 0 1-4 4H5M9 11l-4 4 4 4"/>"#,
            Icon::Close => r#"<path d="M6 6l12 12M18 6L6 18"/>"#,
            Icon::Plus => r#"<path d="M12 5v14M5 12h14"/>"#,
            Icon::Minus => r#"<path d="M5 12h14"/>"#,
            Icon::Check => r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#,
            Icon::Calendar => {
                r#"<rect x="3.5" y="5" width="17" height="15.5" rx="2.5"/><path d="M3.5 10h17M8 3v4M16 3v4"/>"#
            }
            Icon::Loader => r#"<path d="M20 12a8 8 0 1 1-8-8"/>"#,
            Icon::Dot => r##"<circle cx="12" cy="12" r="5" fill="#000" stroke="none"/>"##,
            Icon::Grip => {
                r##"<g fill="#000" stroke="none"><circle cx="9" cy="6" r="1.5"/><circle cx="15" cy="6" r="1.5"/><circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/><circle cx="9" cy="18" r="1.5"/><circle cx="15" cy="18" r="1.5"/></g>"##
            }
        };
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##
        )
    }

    /// The parsed icon, read once and shared.
    pub fn svg(self) -> &'static Svg {
        static CACHE: OnceLock<Vec<Svg>> = OnceLock::new();
        let all = CACHE.get_or_init(|| {
            Icon::ALL
                .iter()
                .map(|icon| Svg::parse(&icon.source()).expect("built-in icons are valid SVG"))
                .collect()
        });
        &all[Icon::ALL.iter().position(|icon| *icon == self).unwrap()]
    }

    /// Paint the icon in `color`, as large as fits in `rect`, centred.
    pub fn paint(self, ui: &mut Ui, rect: Rect, color: Color) {
        self.svg().paint_tinted(ui, rect, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_parses_and_draws_something() {
        for icon in Icon::ALL {
            assert!(!icon.svg().is_empty(), "{icon:?} draws nothing");
        }
    }
}
