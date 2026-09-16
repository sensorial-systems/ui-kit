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
    /// Media and task control.
    Play,
    Pause,
    /// A filled square: stop.
    Stop,
    Trash,
    Refresh,
    Folder,
    File,
    Save,
    /// A gear.
    Settings,
    /// A chip: a processor or a device.
    Cpu,
    Download,
    Upload,
    Flame,
    Snowflake,
    /// A lightning bolt.
    Zap,
    Search,
    Copy,
    ExternalLink,
    /// Three stacked sheets.
    Layers,
    /// A heartbeat line.
    Activity,
    /// A bulleted list.
    List,
    /// A prompt in a box.
    Terminal,
    /// A speedometer.
    Gauge,
    Clock,
    Undo,
    Info,
    AlertTriangle,
    /// A hollow circle.
    Circle,
    /// A database cylinder.
    Database,
    /// A hard drive.
    HardDrive,
    Power,
}

impl Icon {
    pub const ALL: [Icon; 47] = [
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
        Icon::Play,
        Icon::Pause,
        Icon::Stop,
        Icon::Trash,
        Icon::Refresh,
        Icon::Folder,
        Icon::File,
        Icon::Save,
        Icon::Settings,
        Icon::Cpu,
        Icon::Download,
        Icon::Upload,
        Icon::Flame,
        Icon::Snowflake,
        Icon::Zap,
        Icon::Search,
        Icon::Copy,
        Icon::ExternalLink,
        Icon::Layers,
        Icon::Activity,
        Icon::List,
        Icon::Terminal,
        Icon::Gauge,
        Icon::Clock,
        Icon::Undo,
        Icon::Info,
        Icon::AlertTriangle,
        Icon::Circle,
        Icon::Database,
        Icon::HardDrive,
        Icon::Power,
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
            Icon::Play => r##"<path d="M7 5.5v13l11-6.5z" fill="#000" stroke="none"/>"##,
            Icon::Pause => {
                r##"<g fill="#000" stroke="none"><rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/></g>"##
            }
            Icon::Stop => {
                r##"<rect x="6" y="6" width="12" height="12" rx="2" fill="#000" stroke="none"/>"##
            }
            Icon::Trash => {
                r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/>"#
            }
            Icon::Refresh => {
                r#"<path d="M20 11a8 8 0 0 0-14.5-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.5 4.5L20 16M20 20v-4h-4"/>"#
            }
            Icon::Folder => {
                r#"<path d="M3.5 7a2 2 0 0 1 2-2h4l2 2.5h7a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>"#
            }
            Icon::File => {
                r#"<path d="M14 3.5H7a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8.5zM14 3.5v5h5"/>"#
            }
            Icon::Save => {
                r#"<path d="M5 4.5h11l3.5 3.5v11a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1v-13.5a1 1 0 0 1 .5-1zM8 4.5v5h7v-5M8 20v-6h8v6"/>"#
            }
            Icon::Settings => {
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1"/>"#
            }
            Icon::Cpu => {
                r#"<rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9.5" y="9.5" width="5" height="5"/><path d="M9 2.5v3.5M15 2.5v3.5M9 18v3.5M15 18v3.5M2.5 9h3.5M2.5 15h3.5M18 9h3.5M18 15h3.5"/>"#
            }
            Icon::Download => r#"<path d="M12 4v11M7 10l5 5 5-5M4 19h16"/>"#,
            Icon::Upload => r#"<path d="M12 15V4M7 9l5-5 5 5M4 19h16"/>"#,
            Icon::Flame => {
                r#"<path d="M12 3c1 3 4 4.5 4 8.5a4 4 0 0 1-8 0c0-1.5.5-2.5 1.2-3.3.3 1.3 1 2 1.8 2.3C11 8 11 5.5 12 3z"/><path d="M8.5 13.5a3.5 3.5 0 0 0 7 0"/>"#
            }
            Icon::Snowflake => {
                r#"<path d="M12 2.5v19M3.8 7.25l16.4 9.5M3.8 16.75l16.4-9.5M12 2.5l-2.5 2.5M12 2.5l2.5 2.5M12 21.5l-2.5-2.5M12 21.5l2.5-2.5M3.8 7.25l3.4-.9M3.8 7.25l.9 3.4M20.2 16.75l-3.4.9M20.2 16.75l-.9-3.4M3.8 16.75l.9-3.4M3.8 16.75l3.4.9M20.2 7.25l-.9 3.4M20.2 7.25l-3.4-.9"/>"#
            }
            Icon::Zap => {
                r##"<path d="M13 2.5L4.5 13.5H11l-1 8L19.5 10.5H13z" fill="#000" stroke="none"/>"##
            }
            Icon::Search => r#"<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>"#,
            Icon::Copy => {
                r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4.5a1.5 1.5 0 0 1-1.5-1.5v-9A1.5 1.5 0 0 1 4.5 3h9A1.5 1.5 0 0 1 15 4.5V5"/>"#
            }
            Icon::ExternalLink => {
                r#"<path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>"#
            }
            Icon::Layers => {
                r#"<path d="M12 3.5l9 4.5-9 4.5-9-4.5zM3 12.5l9 4.5 9-4.5M3 16.5l9 4.5 9-4.5"/>"#
            }
            Icon::Activity => r#"<path d="M2.5 12h4l3-7 5 14 3-7h4"/>"#,
            Icon::List => {
                r##"<path d="M9 6h12M9 12h12M9 18h12"/><g fill="#000" stroke="none"><circle cx="4.5" cy="6" r="1.5"/><circle cx="4.5" cy="12" r="1.5"/><circle cx="4.5" cy="18" r="1.5"/></g>"##
            }
            Icon::Terminal => {
                r#"<rect x="3" y="4.5" width="18" height="15" rx="2"/><path d="M7 9l3.5 3L7 15M12.5 15H17"/>"#
            }
            Icon::Gauge => {
                r#"<path d="M4 16.5a8.5 8.5 0 1 1 16 0"/><path d="M12 15.5l4.2-5.2"/><circle cx="12" cy="16" r="1.5"/>"#
            }
            Icon::Clock => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#,
            Icon::Undo => r#"<path d="M4 9h10a5 5 0 0 1 0 10h-3M4 9l4-4M4 9l4 4"/>"#,
            Icon::Info => {
                r##"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5"/><circle cx="12" cy="8" r="1" fill="#000" stroke="none"/>"##
            }
            Icon::AlertTriangle => {
                r##"<path d="M12 3.5l9.5 16.5h-19z"/><path d="M12 10v4.5"/><circle cx="12" cy="17.2" r="1" fill="#000" stroke="none"/>"##
            }
            Icon::Circle => r#"<circle cx="12" cy="12" r="7.5"/>"#,
            Icon::Database => {
                r#"<ellipse cx="12" cy="6" rx="8" ry="3"/><path d="M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"/>"#
            }
            Icon::HardDrive => {
                r##"<rect x="3" y="9" width="18" height="10" rx="2"/><path d="M5 9l2.5-4h9L19 9"/><circle cx="17" cy="14" r="1" fill="#000" stroke="none"/><path d="M6.5 14h6"/>"##
            }
            Icon::Power => r#"<path d="M12 3v9M6.3 7.5a8 8 0 1 0 11.4 0"/>"#,
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
