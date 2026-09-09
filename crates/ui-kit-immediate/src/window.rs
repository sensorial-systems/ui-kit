//! Painting the shared window frame in immediate mode.
//!
//! The measurements, the hit test and the button IDs all come from
//! [`ui_kit_core::WindowChrome`], so an immediate host and a Dioxus host draw
//! the same window and emit the same actions. What is added here is only the
//! painting and the frame-to-frame press bookkeeping an immediate host needs.
use crate::immediate::Ui;
use ui_kit_core::{
    Material, Point, Rect, Style, TextAlign, Theme, WindowAction, WindowChrome, WindowControl,
    WindowRegion,
};

/// The colours a frame is painted with. Derived from a [`Theme`] by default;
/// override individual fields for an application with its own title bar look.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowStyle {
    /// The window body, which also draws the rounded outline and the border.
    pub frame: Style,
    pub bar: Style,
    pub title: Style,
    pub control: Style,
    /// The close button, which conventionally warns on hover.
    pub close: Style,
}
impl Default for WindowStyle {
    fn default() -> Self {
        Self::from_theme(&Theme::default())
    }
}
impl WindowStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        let bar = Style {
            radius: 0.0,
            material: Material::Solid,
            ..theme.panel.clone()
        };
        let control = Style {
            fill: [0.0; 4],
            stroke: [0.0; 4],
            stroke_width: 0.0,
            radius: 0.0,
            text_padding: 0.0,
            text_align: TextAlign::Center,
            font_size: 14.0,
            ..theme.control.clone()
        };
        Self {
            frame: theme.panel.clone(),
            bar,
            title: Style {
                text_padding: 14.0,
                font_weight: 600,
                font_size: 13.0,
                ..theme.text.clone()
            },
            close: Style {
                accent: [0.90, 0.22, 0.22, 1.0],
                fill: [0.90, 0.22, 0.22, 0.0],
                ..control.clone()
            },
            control,
        }
    }
}

/// What the frame did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowResponse {
    /// A title bar button was activated. Hosts map this onto their window.
    pub action: Option<WindowAction>,
    /// What the pointer is over, for cursor feedback.
    pub region: Option<WindowRegion>,
    /// The pointer went down on the bar: the host should start a system move.
    pub drag: bool,
    /// The pointer went down on an edge: the host should start a system resize.
    pub resize: Option<ui_kit_core::ResizeEdge>,
    /// Where the application should draw. Below the title bar.
    pub client: Rect,
}

/// Paint the frame and report what the pointer did with it.
///
/// Call this first in a frame, before the application content: it paints the
/// body the content sits on, and it claims the button IDs before anything under
/// the bar can take the press.
pub fn window_chrome(
    ui: &mut Ui,
    chrome: &WindowChrome,
    bounds: Rect,
    style: &WindowStyle,
) -> WindowResponse {
    let radius = chrome.effective_radius();
    let previous_style = std::mem::replace(
        &mut ui.style,
        Style {
            radius,
            ..style.frame.clone()
        },
    );
    let previous_clip = ui.clip.replace(bounds);
    ui.panel(bounds);
    let bar = chrome.title_bar(bounds);
    ui.style = Style {
        radius,
        ..style.bar.clone()
    };
    ui.panel(bar);
    // Square off the bottom two corners of the bar: it is the top of the body,
    // not a rounded rectangle floating on it.
    if let Some(command) = ui.commands.last_mut() {
        command.rect.height += radius;
        command.clip = Some(bar);
    }
    ui.style = Style {
        radius: 0.0,
        ..style.title.clone()
    };
    ui.label(chrome.drag_area(bounds), &chrome.title);

    let region = ui
        .input()
        .pointer
        .map(|p| chrome.hit(bounds, Point::new(p.x, p.y)));
    // The top few pixels of a button are also the window's top edge, and the
    // hit test hands those to the resize. The buttons have to agree, or one
    // press would both arm a button and start a resize: while the pointer is on
    // an edge they keep their IDs -- so focus and keyboard activation survive --
    // but shrink out of its way.
    let on_edge = matches!(region, Some(WindowRegion::Resize(_)));
    let mut action = None;
    for control in &chrome.controls {
        let Some(rect) = chrome.control_rect(bounds, *control) else {
            continue;
        };
        ui.style = if *control == WindowControl::Close {
            style.close.clone()
        } else {
            style.control.clone()
        };
        // The hover tint the renderer applies is relative to the fill, so a
        // control that is transparent until hovered needs the fill to carry the
        // colour and the alpha to arrive with the pointer.
        if *control == WindowControl::Close {
            ui.style.fill = ui.style.accent;
            ui.style.fill[3] = 0.0;
        }
        let response = ui.interact(
            control.id(),
            if on_edge {
                Rect {
                    width: 0.0,
                    height: 0.0,
                    ..rect
                }
            } else {
                rect
            },
        );
        ui.paint(
            "button",
            control.glyph(chrome.maximized),
            rect,
            0.0,
            response,
            ui.focused(control.id()),
        );
        if response.clicked {
            action = Some(WindowAction::from(*control));
        }
    }
    ui.style = previous_style;
    ui.clip = previous_clip;

    // A press that lands on a button belongs to the button, and a press that is
    // already dragging something else is not the frame's to take.
    let starting = ui.pressed();
    WindowResponse {
        action,
        region,
        drag: starting && region == Some(WindowRegion::Drag),
        resize: match region {
            Some(WindowRegion::Resize(edge)) if starting => Some(edge),
            _ => None,
        },
        client: chrome.client(bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immediate::Input;
    fn bounds() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }
    fn press(ui: &mut Ui, chrome: &WindowChrome, point: Point, down: bool) -> WindowResponse {
        ui.begin(Input {
            pointer: Some(point),
            down,
            ..Default::default()
        });
        let response = window_chrome(ui, chrome, bounds(), &WindowStyle::default());
        ui.end();
        response
    }
    #[test]
    fn a_button_press_and_release_produces_one_action() {
        let chrome = WindowChrome::new("Explorer");
        let rect = chrome
            .control_rect(bounds(), WindowControl::Maximize)
            .unwrap();
        let point = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let mut ui = Ui::default();
        assert_eq!(press(&mut ui, &chrome, point, true).action, None);
        assert_eq!(
            press(&mut ui, &chrome, point, false).action,
            Some(WindowAction::ToggleMaximize)
        );
    }
    #[test]
    fn pressing_the_bar_starts_a_move_and_pressing_an_edge_starts_a_resize() {
        let chrome = WindowChrome::new("Explorer");
        let mut ui = Ui::default();
        let drag = press(&mut ui, &chrome, Point::new(200.0, 18.0), true);
        assert!(drag.drag && drag.resize.is_none());
        // Held, not newly pressed: the host has the drag already.
        assert!(!press(&mut ui, &chrome, Point::new(200.0, 18.0), true).drag);
        let mut ui = Ui::default();
        let resize = press(&mut ui, &chrome, Point::new(799.0, 599.0), true);
        assert_eq!(resize.resize, Some(ui_kit_core::ResizeEdge::SouthEast));
        assert!(!resize.drag);
    }
    #[test]
    fn a_button_press_is_not_also_a_window_drag() {
        let chrome = WindowChrome::new("Explorer");
        let rect = chrome.control_rect(bounds(), WindowControl::Close).unwrap();
        let mut ui = Ui::default();
        let response = press(
            &mut ui,
            &chrome,
            Point::new(rect.x + 4.0, rect.y + rect.height * 0.5),
            true,
        );
        assert!(!response.drag && response.resize.is_none());
        assert_eq!(
            response.region,
            Some(WindowRegion::Control(WindowControl::Close))
        );
    }
    /// The strip a button shares with the window's top edge belongs to the
    /// resize, and the button must not arm there as well.
    #[test]
    fn a_press_on_the_top_edge_of_a_button_resizes_instead_of_closing() {
        let chrome = WindowChrome::new("Explorer");
        let rect = chrome.control_rect(bounds(), WindowControl::Close).unwrap();
        let point = Point::new(rect.x + 4.0, 1.0);
        let mut ui = Ui::default();
        let down = press(&mut ui, &chrome, point, true);
        assert_eq!(down.resize, Some(ui_kit_core::ResizeEdge::North));
        assert_eq!(press(&mut ui, &chrome, point, false).action, None);
    }
    #[test]
    fn the_client_area_starts_below_the_bar() {
        let chrome = WindowChrome::new("Explorer");
        let mut ui = Ui::default();
        let response = press(&mut ui, &chrome, Point::new(400.0, 300.0), false);
        assert_eq!(response.client.y, chrome.title_bar_height);
        assert_eq!(
            response.client.height,
            bounds().height - chrome.title_bar_height
        );
    }
}
