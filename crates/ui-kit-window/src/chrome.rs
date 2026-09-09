//! A window that paints its own frame.
//!
//! An application that hides the operating system title bar has to answer three
//! questions itself: where the buttons are, which of them the pointer is over,
//! and which part of the frame starts a move or a resize. None of that is
//! specific to a renderer, so it lives here, next to the rest of the shared
//! model: the immediate host paints it, Dioxus renders it as DOM, and a native
//! window asks [`WindowChrome::hit`] what the pointer is on.
use serde::{Deserialize, Serialize};
use ui_kit_core::{button, row, text, Point, Rect, View};

/// A title bar button.
///
/// The IDs are part of the shared contract. Both hosts emit events under these
/// exact strings, which is what lets one description drive either of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}
impl WindowControl {
    pub const ALL: [Self; 3] = [Self::Minimize, Self::Maximize, Self::Close];
    pub fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window.minimize",
            Self::Maximize => "window.maximize",
            Self::Close => "window.close",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    /// The accessible name, which changes with the window state.
    pub fn label(self, maximized: bool) -> &'static str {
        match self {
            Self::Minimize => "Minimize",
            Self::Maximize if maximized => "Restore",
            Self::Maximize => "Maximize",
            Self::Close => "Close",
        }
    }
    /// A glyph from the system font, so no icon set has to ship with the kit.
    pub fn glyph(self, maximized: bool) -> &'static str {
        match self {
            Self::Minimize => "\u{2013}",
            Self::Maximize if maximized => "\u{2750}",
            Self::Maximize => "\u{25a1}",
            Self::Close => "\u{2715}",
        }
    }
}

/// The side or corner a resize drag grows from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}
impl ResizeEdge {
    /// The axes involved, as `(horizontal, vertical)` in -1/0/+1 form. Hosts map
    /// this onto their own direction enum without a match arm per host.
    pub fn axes(self) -> (i8, i8) {
        match self {
            Self::North => (0, -1),
            Self::South => (0, 1),
            Self::East => (1, 0),
            Self::West => (-1, 0),
            Self::NorthEast => (1, -1),
            Self::NorthWest => (-1, -1),
            Self::SouthEast => (1, 1),
            Self::SouthWest => (-1, 1),
        }
    }
    fn from_axes(horizontal: i8, vertical: i8) -> Option<Self> {
        Some(match (horizontal, vertical) {
            (0, -1) => Self::North,
            (0, 1) => Self::South,
            (1, 0) => Self::East,
            (-1, 0) => Self::West,
            (1, -1) => Self::NorthEast,
            (-1, -1) => Self::NorthWest,
            (1, 1) => Self::SouthEast,
            (-1, 1) => Self::SouthWest,
            _ => return None,
        })
    }
}

/// What sits under a point on the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRegion {
    /// The application content.
    Client,
    /// Empty title bar. Pressing here moves the window.
    Drag,
    Control(WindowControl),
    Resize(ResizeEdge),
}

/// What a host should do with a title bar activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    Minimize,
    ToggleMaximize,
    Close,
}
impl From<WindowControl> for WindowAction {
    fn from(control: WindowControl) -> Self {
        match control {
            WindowControl::Minimize => Self::Minimize,
            WindowControl::Maximize => Self::ToggleMaximize,
            WindowControl::Close => Self::Close,
        }
    }
}

/// The measurements and state of a self-drawn window frame.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowChrome {
    pub title: String,
    pub title_bar_height: f32,
    pub control_width: f32,
    /// How far in from an edge a press starts a resize instead of a click.
    pub resize_border: f32,
    pub radius: f32,
    pub maximized: bool,
    pub resizable: bool,
    /// Buttons, laid out left to right at the trailing end of the bar.
    pub controls: Vec<WindowControl>,
}
impl Default for WindowChrome {
    fn default() -> Self {
        Self {
            title: String::new(),
            title_bar_height: 38.0,
            control_width: 46.0,
            resize_border: 6.0,
            radius: 10.0,
            maximized: false,
            resizable: true,
            controls: WindowControl::ALL.into(),
        }
    }
}
impl WindowChrome {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }
    pub fn maximized(mut self, value: bool) -> Self {
        self.maximized = value;
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }
    pub fn controls(mut self, value: impl IntoIterator<Item = WindowControl>) -> Self {
        self.controls = value.into_iter().collect();
        self
    }
    /// A maximized window has no rounded corners and no resize border: it sits
    /// flush with the screen, where both would leave a gap or swallow clicks.
    pub fn effective_radius(&self) -> f32 {
        if self.maximized {
            0.0
        } else {
            self.radius
        }
    }
    fn effective_border(&self) -> f32 {
        if self.resizable && !self.maximized {
            self.resize_border.max(0.0)
        } else {
            0.0
        }
    }
    pub fn title_bar(&self, bounds: Rect) -> Rect {
        Rect {
            height: self.title_bar_height.clamp(0.0, bounds.height.max(0.0)),
            ..bounds
        }
    }
    /// Everything below the title bar: where the application draws.
    pub fn client(&self, bounds: Rect) -> Rect {
        let bar = self.title_bar(bounds);
        Rect {
            x: bounds.x,
            y: bounds.y + bar.height,
            width: bounds.width,
            height: (bounds.height - bar.height).max(0.0),
        }
    }
    /// The total width the buttons occupy at the trailing end of the bar.
    pub fn controls_width(&self) -> f32 {
        self.control_width.max(0.0) * self.controls.len() as f32
    }
    pub fn control_rect(&self, bounds: Rect, control: WindowControl) -> Option<Rect> {
        let index = self.controls.iter().position(|c| *c == control)?;
        let bar = self.title_bar(bounds);
        let width = self.control_width.max(0.0);
        Some(Rect {
            x: bar.x + bar.width - self.controls_width() + index as f32 * width,
            width,
            ..bar
        })
    }
    /// The part of the bar that moves the window when pressed.
    pub fn drag_area(&self, bounds: Rect) -> Rect {
        let bar = self.title_bar(bounds);
        Rect {
            width: (bar.width - self.controls_width()).max(0.0),
            ..bar
        }
    }
    /// What the pointer is over. Resize edges win over the bar, and the buttons
    /// win over dragging it, so a press is never ambiguous.
    pub fn hit(&self, bounds: Rect, point: Point) -> WindowRegion {
        if !bounds.contains(point) {
            return WindowRegion::Client;
        }
        let border = self.effective_border();
        if border > 0.0 {
            let horizontal = if point.x < bounds.x + border {
                -1
            } else if point.x >= bounds.x + bounds.width - border {
                1
            } else {
                0
            };
            let vertical = if point.y < bounds.y + border {
                -1
            } else if point.y >= bounds.y + bounds.height - border {
                1
            } else {
                0
            };
            if let Some(edge) = ResizeEdge::from_axes(horizontal, vertical) {
                return WindowRegion::Resize(edge);
            }
        }
        for control in &self.controls {
            if self
                .control_rect(bounds, *control)
                .is_some_and(|r| r.contains(point))
            {
                return WindowRegion::Control(*control);
            }
        }
        if self.drag_area(bounds).contains(point) {
            return WindowRegion::Drag;
        }
        WindowRegion::Client
    }
    /// The title bar as a shared description, so a host that already renders
    /// [`View`]s gets the same bar, the same IDs and the same actions for free.
    pub fn view<A: Clone + Send + Sync + 'static>(
        &self,
        action: impl Fn(WindowAction) -> A,
    ) -> View<A> {
        let mut children = vec![text(self.title.clone())];
        children.extend(self.controls.iter().map(|control| {
            button(control.id(), control.glyph(self.maximized))
                .width(self.control_width.max(0.0))
                .on_activate(action(WindowAction::from(*control)))
        }));
        row(children)
            .height(self.title_bar_height)
            .gap(0.0)
            .padding(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bounds() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }
    #[test]
    fn controls_sit_at_the_trailing_end_in_order() {
        let chrome = WindowChrome::new("Explorer");
        let x: Vec<f32> = WindowControl::ALL
            .iter()
            .map(|c| chrome.control_rect(bounds(), *c).unwrap().x)
            .collect();
        assert!(x[0] < x[1] && x[1] < x[2]);
        let close = chrome.control_rect(bounds(), WindowControl::Close).unwrap();
        assert_eq!(close.x + close.width, 800.0);
    }
    #[test]
    fn edges_take_priority_over_the_bar_and_buttons_over_dragging() {
        let chrome = WindowChrome::new("Explorer");
        assert_eq!(
            chrome.hit(bounds(), Point::new(1.0, 1.0)),
            WindowRegion::Resize(ResizeEdge::NorthWest)
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(400.0, 599.5)),
            WindowRegion::Resize(ResizeEdge::South)
        );
        let close = chrome.control_rect(bounds(), WindowControl::Close).unwrap();
        assert_eq!(
            chrome.hit(bounds(), Point::new(close.x + 4.0, 20.0)),
            WindowRegion::Control(WindowControl::Close)
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(200.0, 20.0)),
            WindowRegion::Drag
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(200.0, 300.0)),
            WindowRegion::Client
        );
    }
    #[test]
    fn a_maximized_window_has_no_resize_border_or_rounding() {
        let chrome = WindowChrome::new("Explorer").maximized(true);
        assert_eq!(chrome.effective_radius(), 0.0);
        assert_eq!(
            chrome.hit(bounds(), Point::new(0.0, 300.0)),
            WindowRegion::Client
        );
        let fixed = WindowChrome::new("Explorer").resizable(false);
        assert_eq!(
            fixed.hit(bounds(), Point::new(0.0, 300.0)),
            WindowRegion::Client
        );
    }
    #[test]
    fn the_shared_description_is_valid_and_carries_the_control_ids() {
        let chrome = WindowChrome::new("Explorer");
        let view = chrome.view(|action| action);
        view.validate().unwrap();
        for control in WindowControl::ALL {
            assert_eq!(WindowControl::from_id(control.id()), Some(control));
            assert!(
                view.children
                    .iter()
                    .any(|c| c.id.as_deref() == Some(control.id())),
                "{} is missing from the shared title bar",
                control.id()
            );
        }
    }
}
