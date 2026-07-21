#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    Black,
    #[default]
    White,
}

impl AppTheme {
    pub fn class_name(&self) -> &'static str {
        match self {
            AppTheme::Black => "uikit-theme-black",
            AppTheme::White => "uikit-theme-white",
        }
    }
}
