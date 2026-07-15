#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    #[default]
    Neutral,
    Black,
    White,
}

impl AppTheme {
    pub fn class_name(&self) -> &'static str {
        match self {
            AppTheme::Neutral => "uikit-theme-neutral",
            AppTheme::Black => "uikit-theme-black",
            AppTheme::White => "uikit-theme-white",
        }
    }
}
