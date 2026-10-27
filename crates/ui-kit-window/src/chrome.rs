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
    /// Take the whole screen, or give it back.
    Fullscreen,
    Minimize,
    Maximize,
    Close,
}
impl WindowControl {
    pub const ALL: [Self; 4] = [Self::Fullscreen, Self::Minimize, Self::Maximize, Self::Close];
    pub fn id(self) -> &'static str {
        match self {
            Self::Fullscreen => "window.fullscreen",
            Self::Minimize => "window.minimize",
            Self::Maximize => "window.maximize",
            Self::Close => "window.close",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    /// The accessible name, which changes with the window state. A
    /// [`WindowChrome`] knows all of it: [`WindowChrome::label`].
    pub fn label(self, maximized: bool) -> &'static str {
        match self {
            Self::Fullscreen => "Full screen",
            Self::Minimize => "Minimize",
            Self::Maximize if maximized => "Restore",
            Self::Maximize => "Maximize",
            Self::Close => "Close",
        }
    }
    /// A glyph from the system font, so no icon set has to ship with the kit.
    pub fn glyph(self, maximized: bool) -> &'static str {
        match self {
            Self::Fullscreen => "\u{26f6}",
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
    ToggleFullscreen,
    Minimize,
    ToggleMaximize,
    Close,
}
impl From<WindowControl> for WindowAction {
    fn from(control: WindowControl) -> Self {
        match control {
            WindowControl::Fullscreen => Self::ToggleFullscreen,
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
    /// How far along an edge from a corner a press on that edge resizes
    /// both ways, so a corner is easy to catch.
    pub resize_corner: f32,
    pub radius: f32,
    pub maximized: bool,
    /// Whether the window takes the whole screen. Like a maximized one, it
    /// sits flush with the screen's edges, and it has no frame at all: no
    /// title bar, buttons or outline, the application all of it.
    pub fullscreen: bool,
    pub resizable: bool,
    pub transparent: bool,
    /// The application draws the title bar itself: its content runs to the
    /// top of the window, under the bar, and only the window's buttons are
    /// painted over it. The bar still moves the window, except where the
    /// application has controls of its own on it.
    pub custom_bar: bool,
    /// How tall the buttons are, centred on the bar: the bar's height when
    /// `None`. A tall bar an application draws reads better with buttons of
    /// the usual height in its middle than with buttons as tall as it.
    pub control_height: Option<f32>,
    /// Buttons, laid out left to right at the trailing end of the bar.
    pub controls: Vec<WindowControl>,
}
impl Default for WindowChrome {
    fn default() -> Self {
        Self {
            title: String::new(),
            title_bar_height: 38.0,
            control_width: 46.0,
            resize_border: 8.0,
            resize_corner: 16.0,
            radius: 10.0,
            maximized: false,
            fullscreen: false,
            resizable: true,
            transparent: true,
            custom_bar: false,
            control_height: None,
            controls: vec![WindowControl::Minimize, WindowControl::Maximize, WindowControl::Close],
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
    pub fn transparent(mut self, value: bool) -> Self {
        self.transparent = value;
        self
    }
    pub fn fullscreen(mut self, value: bool) -> Self {
        self.fullscreen = value;
        self
    }
    /// Leave the bar to the application (see [`WindowChrome::custom_bar`]),
    /// `height` tall.
    pub fn custom_bar(mut self, height: f32) -> Self {
        self.custom_bar = true;
        self.title_bar_height = height;
        self
    }
    pub fn controls(mut self, value: impl IntoIterator<Item = WindowControl>) -> Self {
        self.controls = value.into_iter().collect();
        self
    }
    /// Whether the window sits flush with the screen's edges.
    fn flush(&self) -> bool {
        self.maximized || self.fullscreen
    }
    /// A maximized or fullscreen window has no rounded corners and no resize
    /// border: it sits flush with the screen, where both would leave a gap or
    /// swallow clicks.
    pub fn effective_radius(&self) -> f32 {
        if self.flush() {
            0.0
        } else {
            self.radius
        }
    }
    /// The accessible name of `control`, as the window is now.
    pub fn label(&self, control: WindowControl) -> &'static str {
        match control {
            WindowControl::Fullscreen if self.fullscreen => "Exit full screen",
            control => control.label(self.maximized),
        }
    }
    fn effective_border(&self) -> f32 {
        if self.resizable && !self.flush() {
            self.resize_border.max(0.0)
        } else {
            0.0
        }
    }
    /// The bar across the top: none on the whole screen.
    pub fn title_bar(&self, bounds: Rect) -> Rect {
        let height = if self.fullscreen { 0.0 } else { self.title_bar_height };
        Rect {
            height: height.clamp(0.0, bounds.height.max(0.0)),
            ..bounds
        }
    }
    /// Where the application draws: everything below the title bar, or the
    /// whole window when the application draws the bar.
    pub fn client(&self, bounds: Rect) -> Rect {
        if self.custom_bar {
            return bounds;
        }
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
        let height = self.control_height.map_or(bar.height, |h| h.clamp(0.0, bar.height));
        Some(Rect {
            x: bar.x + bar.width - self.controls_width() + index as f32 * width,
            y: bar.y + (bar.height - height) * 0.5,
            width,
            height,
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
            // -1 near the start of an axis, 1 near its end, 0 between.
            let side = |at: f32, start: f32, length: f32, reach: f32| {
                if at < start + reach {
                    -1
                } else if at >= start + length - reach {
                    1
                } else {
                    0
                }
            };
            let horizontal = side(point.x, bounds.x, bounds.width, border);
            let vertical = side(point.y, bounds.y, bounds.height, border);
            if horizontal != 0 || vertical != 0 {
                // On an edge: near enough a corner, it is the corner.
                let corner = self.resize_corner.max(border);
                let horizontal = match horizontal {
                    0 => side(point.x, bounds.x, bounds.width, corner),
                    h => h,
                };
                let vertical = match vertical {
                    0 => side(point.y, bounds.y, bounds.height, corner),
                    v => v,
                };
                if let Some(edge) = ResizeEdge::from_axes(horizontal, vertical) {
                    return WindowRegion::Resize(edge);
                }
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
        let chrome = WindowChrome::new("Explorer").controls(WindowControl::ALL);
        let x: Vec<f32> = WindowControl::ALL
            .iter()
            .map(|c| chrome.control_rect(bounds(), *c).unwrap().x)
            .collect();
        assert!(x.windows(2).all(|pair| pair[0] < pair[1]));
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
        assert_eq!(
            chrome.hit(bounds(), Point::new(400.0, 593.0)),
            WindowRegion::Resize(ResizeEdge::South),
            "the edge is wide enough to catch"
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(788.0, 598.0)),
            WindowRegion::Resize(ResizeEdge::SouthEast),
            "a corner reaches along both its edges"
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(2.0, 586.0)),
            WindowRegion::Resize(ResizeEdge::SouthWest)
        );
        assert_eq!(
            chrome.hit(bounds(), Point::new(2.0, 300.0)),
            WindowRegion::Resize(ResizeEdge::West)
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
    fn an_application_drawn_bar_leaves_the_whole_window_to_the_application() {
        let chrome = WindowChrome::new("Store")
            .custom_bar(72.0)
            .controls(WindowControl::ALL);
        assert_eq!(chrome.client(bounds()), bounds());
        // Still moved from the bar, and resized from the edges.
        assert_eq!(chrome.hit(bounds(), Point::new(300.0, 40.0)), WindowRegion::Drag);
        assert_eq!(chrome.hit(bounds(), Point::new(300.0, 100.0)), WindowRegion::Client);
        let centred = WindowChrome {
            control_height: Some(40.0),
            ..chrome.clone()
        };
        let close = centred.control_rect(bounds(), WindowControl::Close).unwrap();
        assert_eq!((close.y, close.height), (16.0, 40.0));
        assert_eq!(chrome.controls_width(), 4.0 * chrome.control_width);
    }

    #[test]
    fn a_maximized_window_has_no_resize_border_or_rounding() {
        let chrome = WindowChrome::new("Explorer").maximized(true);
        assert_eq!(chrome.effective_radius(), 0.0);
        assert_eq!(
            chrome.hit(bounds(), Point::new(0.0, 300.0)),
            WindowRegion::Client
        );
        let fullscreen = WindowChrome::new("Explorer").fullscreen(true);
        assert_eq!(fullscreen.effective_radius(), 0.0);
        assert_eq!(
            fullscreen.hit(bounds(), Point::new(0.0, 300.0)),
            WindowRegion::Client
        );
        assert_eq!(fullscreen.label(WindowControl::Fullscreen), "Exit full screen");
        assert_eq!(fullscreen.client(bounds()), bounds(), "the whole screen is the application's");
        assert_eq!(fullscreen.hit(bounds(), Point::new(200.0, 20.0)), WindowRegion::Client, "nothing on it drags the window");
        assert_eq!(
            fullscreen.hit(bounds(), Point::new(799.0, 1.0)),
            WindowRegion::Client,
            "nor are there buttons in its corner"
        );
        let fixed = WindowChrome::new("Explorer").resizable(false);
        assert_eq!(
            fixed.hit(bounds(), Point::new(0.0, 300.0)),
            WindowRegion::Client
        );
    }
    #[test]
    fn the_shared_description_is_valid_and_carries_the_control_ids() {
        let chrome = WindowChrome::new("Explorer").controls(WindowControl::ALL);
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
