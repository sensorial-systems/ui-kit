use crate::{ResizeEdge, WindowAction, WindowChrome, WindowRegion};
use std::time::{Duration, Instant};
use ui_kit_core::{Point, Rect};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    window::{CursorIcon, ResizeDirection, Window, WindowAttributes},
};

/// Route consumed events away from application widgets. The host owns closing.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ChromeEvent {
    pub consumed: bool,
    pub close_requested: bool,
}

/// Persist alongside a winit window. No event loop or renderer is owned here.
pub struct WinitWindow {
    pub chrome: WindowChrome,
    pointer: Option<Point>,
    pressed: Option<WindowRegion>,
    /// The resize cursor last set for an edge, so the cursor changes only
    /// when the pointer goes onto an edge or off it -- not at every move,
    /// which would undo the cursor the content asked for.
    edge_cursor: Option<CursorIcon>,
    /// Where the application has controls of its own on a bar it draws, in
    /// window coordinates: presses there are its, not the window's.
    bar_controls: Vec<Rect>,
    /// When the bar was last pressed, to tell a double press.
    bar_pressed_at: Option<Instant>,
    /// Whether the window manager's edge was last set for the whole screen
    /// (see `sync`).
    edge_fullscreen: Option<bool>,
}

/// Two presses on the bar this close together maximize or restore the window.
const DOUBLE_PRESS: Duration = Duration::from_millis(450);

impl WinitWindow {
    pub fn new(title: impl Into<String>) -> Self {
        // Windows rounds the corners itself (see `attributes`), by its own
        // radius: the outline follows it.
        let radius = if cfg!(windows) { 8.0 } else { 10.0 };
        Self {
            chrome: WindowChrome {
                radius,
                ..WindowChrome::new(title)
            },
            pointer: None,
            pressed: None,
            edge_cursor: None,
            bar_controls: Vec::new(),
            bar_pressed_at: None,
            edge_fullscreen: None,
        }
    }

    /// Where the application has controls on a bar it draws
    /// ([`WindowChrome::custom_bar`]), in window coordinates: the bar moves
    /// the window everywhere else. Rectangles whose middle is off the bar are
    /// not on it, and are left out. The application's last frame says.
    pub fn set_bar_controls(&mut self, bounds: Rect, controls: impl IntoIterator<Item = Rect>) {
        let bar = self.chrome.drag_area(bounds);
        self.bar_controls.clear();
        self.bar_controls.extend(controls.into_iter().filter(|rect| {
            bar.contains(Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5))
        }));
    }

    /// What `point` is on: the frame's own answer, except that the
    /// application's controls on the bar are the application's.
    /// The cursor the frame itself shows now, over whatever the content
    /// asks for: a resize cursor while the pointer is on an edge or
    /// dragging one. A host applying the content's cursor after each frame
    /// uses this one instead where there is one; otherwise the two take
    /// turns at every frame and the cursor flickers on the edges.
    pub fn cursor(&self) -> Option<CursorIcon> {
        match self.pressed {
            Some(region @ WindowRegion::Resize(_)) => resize_cursor(region),
            _ => self.edge_cursor,
        }
    }

    pub fn region(&self, bounds: Rect, point: Point) -> WindowRegion {
        match self.chrome.hit(bounds, point) {
            WindowRegion::Drag if self.bar_controls.iter().any(|r| r.contains(point)) => {
                WindowRegion::Client
            }
            region => region,
        }
    }

    /// Compose with the caller's size, platform options, and other attributes.
    ///
    /// On Windows 11 the window manager rounds the window's corners, as it
    /// does a decorated window's, and gives it its shadow and edge: an opaque
    /// window is rounded too, and it is square again maximized or fullscreen.
    pub fn attributes(&self, attributes: WindowAttributes) -> WindowAttributes {
        let attributes = attributes
            .with_title(&self.chrome.title)
            .with_decorations(false)
            .with_resizable(self.chrome.resizable)
            .with_transparent(self.chrome.transparent);
        #[cfg(windows)]
        let attributes = {
            use winit::platform::windows::{CornerPreference, WindowAttributesExtWindows};
            attributes.with_corner_preference(if self.chrome.radius > 0.0 {
                CornerPreference::Round
            } else {
                CornerPreference::DoNotRound
            })
        };
        attributes
    }

    /// Whether the window manager rounds the corners itself. Where it does
    /// not, a host with a transparent surface rounds them as it presents.
    pub fn rounded_by_system(&self) -> bool {
        cfg!(windows)
    }


    pub fn bounds(&self, window: &Window) -> Rect {
        let size = window.inner_size().to_logical::<f32>(window.scale_factor());
        Rect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        }
    }

    pub fn sync(&mut self, window: &Window) {
        self.chrome.maximized = window.is_maximized();
        self.chrome.fullscreen = window.fullscreen().is_some();
        self.chrome.resizable = window.is_resizable();
        if self.edge_fullscreen != Some(self.chrome.fullscreen) {
            self.edge_fullscreen = Some(self.chrome.fullscreen);
            self.set_edge(window);
        }
    }

    /// On Windows 11 the window manager draws a thin edge round a window
    /// whose corners it rounds, and keeps it on one without a frame even
    /// when it covers the screen: there, the window has neither, and the
    /// application every pixel. In a window again, both come back.
    fn set_edge(&self, window: &Window) {
        #[cfg(windows)]
        {
            use winit::platform::windows::{Color, CornerPreference, WindowExtWindows};
            if self.chrome.fullscreen {
                window.set_corner_preference(CornerPreference::DoNotRound);
                window.set_border_color(None);
            } else {
                window.set_corner_preference(if self.chrome.radius > 0.0 {
                    CornerPreference::Round
                } else {
                    CornerPreference::DoNotRound
                });
                window.set_border_color(Some(Color::SYSTEM_DEFAULT));
            }
        }
        #[cfg(not(windows))]
        let _ = window;
    }

    /// Call before application event handling. Move/resize must start during
    /// the mouse event (rather than waiting for a render on another frame).
    pub fn event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> Result<ChromeEvent, winit::error::ExternalError> {
        self.sync(window);
        let bounds = self.bounds(window);
        let mut result = ChromeEvent::default();
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical::<f32>(window.scale_factor());
                self.pointer = Some(Point::new(p.x, p.y));
                let edge = resize_cursor(self.region(bounds, self.pointer.unwrap()));
                if edge != self.edge_cursor {
                    // Off an edge, the content's cursor shows again with
                    // its next frame; until then, the plain one.
                    window.set_cursor(edge.unwrap_or_default());
                    self.edge_cursor = edge;
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer = None;
                if self.pressed.is_none() {
                    self.edge_cursor = None;
                }
            }
            WindowEvent::Focused(false) => {
                self.pointer = None;
                self.pressed = None;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                let region = self
                    .pointer
                    .map(|p| self.region(bounds, p))
                    .unwrap_or(WindowRegion::Client);
                if *state == ElementState::Pressed {
                    self.pressed = Some(region);
                    result.consumed = region != WindowRegion::Client;
                    match region {
                        WindowRegion::Drag => {
                            self.pressed = None;
                            let now = Instant::now();
                            let double = self
                                .bar_pressed_at
                                .is_some_and(|at| now.duration_since(at) < DOUBLE_PRESS);
                            if double && self.chrome.resizable && !self.chrome.fullscreen {
                                self.bar_pressed_at = None;
                                self.action(window, WindowAction::ToggleMaximize);
                            } else {
                                self.bar_pressed_at = Some(now);
                                window.drag_window()?;
                            }
                        }
                        WindowRegion::Resize(edge) => {
                            self.pressed = None;
                            window.drag_resize_window(direction(edge))?;
                        }
                        _ => {}
                    }
                } else if let Some(pressed) = self.pressed.take() {
                    result.consumed = pressed != WindowRegion::Client;
                    if pressed == region {
                        if let WindowRegion::Control(control) = region {
                            result.close_requested = self.action(window, control.into());
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { .. } => {
                result.consumed = self
                    .pointer
                    .is_some_and(|p| self.region(bounds, p) != WindowRegion::Client);
            }
            _ => {}
        }
        if !matches!(event, WindowEvent::RedrawRequested) {
            window.request_redraw();
        }
        Ok(result)
    }

    /// Also usable by another renderer's title-bar buttons.
    pub fn action(&mut self, window: &Window, action: WindowAction) -> bool {
        match action {
            WindowAction::ToggleFullscreen => window.set_fullscreen(match window.fullscreen() {
                Some(_) => None,
                None => Some(winit::window::Fullscreen::Borderless(None)),
            }),
            WindowAction::Minimize => window.set_minimized(true),
            WindowAction::ToggleMaximize => window.set_maximized(!window.is_maximized()),
            WindowAction::Close => return true,
        }
        self.sync(window);
        false
    }

    /// Paint chrome and compose client-local commands and input. The application
    /// receives a viewport starting at (0, 0); translation and clipping live here.
    #[cfg(feature = "immediate")]
    pub fn frame(
        &mut self,
        window: &Window,
        input: ui_kit_immediate::Input,
        style: &crate::WindowStyle,
        content: impl FnOnce(ui_kit_immediate::Input, f32, f32) -> Vec<ui_kit_immediate::DrawCommand>,
    ) -> Vec<ui_kit_immediate::DrawCommand> {
        self.sync(window);
        let bounds = self.bounds(window);
        self.frame_in_bounds(bounds, input, style, content)
    }

    /// The same composition without an OS window, for offscreen rendering and tests.
    /// Custom materials must express geometry relative to their command rectangle.
    #[cfg(feature = "immediate")]
    pub fn frame_in_bounds(
        &self,
        bounds: Rect,
        input: ui_kit_immediate::Input,
        style: &crate::WindowStyle,
        content: impl FnOnce(ui_kit_immediate::Input, f32, f32) -> Vec<ui_kit_immediate::DrawCommand>,
    ) -> Vec<ui_kit_immediate::DrawCommand> {
        // On the whole screen there is no frame: the application has it all.
        if self.chrome.fullscreen {
            return content(input, bounds.width, bounds.height);
        }
        let mut ui = ui_kit_immediate::Ui::default();
        ui.begin(ui_kit_immediate::Input {
            pointer: self.pointer,
            ..Default::default()
        });
        // The outline goes on after the content, which would cover it where
        // it reaches the window's edge.
        let mut body = style.clone();
        body.frame.stroke_width = 0.0;
        crate::window_chrome(&mut ui, &self.chrome, bounds, &body);
        ui.end();
        if let Some(WindowRegion::Control(control)) = self.pressed {
            let pressed = self.chrome.control_rect(bounds, control);
            for command in &mut ui.commands {
                command.active = command.hovered && Some(command.rect) == pressed;
            }
        }
        let client = self.chrome.client(bounds);
        let mut input = input;
        input.pointer = input
            .pointer
            .filter(|p| client.contains(*p))
            .map(|p| Point::new(p.x - client.x, p.y - client.y));
        // Fingers live in the same client coordinates as the pointer.
        for touch in &mut input.touches {
            touch.position = Point::new(touch.position.x - client.x, touch.position.y - client.y);
        }
        let mut commands = content(input, client.width, client.height);
        for command in &mut commands {
            // A command drawn on a zoomed canvas is in the canvas's units:
            // the client's offset goes on its way to the screen, unscaled.
            match &mut command.transform {
                Some(transform) => {
                    transform.offset_x += client.x;
                    transform.offset_y += client.y;
                }
                None => {
                    command.rect.x += client.x;
                    command.rect.y += client.y;
                }
            }
            let clip = command.clip.unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                width: client.width,
                height: client.height,
            });
            let x = (clip.x + client.x).max(client.x);
            let y = (clip.y + client.y).max(client.y);
            command.clip = Some(Rect {
                x,
                y,
                width: ((clip.x + client.x + clip.width).min(client.x + client.width) - x).max(0.0),
                height: ((clip.y + client.y + clip.height).min(client.y + client.height) - y)
                    .max(0.0),
            });
        }
        // On a bar the application draws, the window's buttons go over what
        // it drew there: after its content, which is drawn in order.
        let buttons: Vec<_> = if self.chrome.custom_bar {
            let (buttons, body) = std::mem::take(&mut ui.commands)
                .into_iter()
                .partition(|command| command.layer == crate::immediate::BUTTONS);
            ui.commands = body;
            buttons
        } else {
            Vec::new()
        };
        ui.commands.extend(commands);
        ui.commands.extend(buttons);
        let mut outline = ui_kit_immediate::Ui::default();
        outline.begin(ui_kit_immediate::Input::default());
        crate::window_outline(&mut outline, &self.chrome, bounds, style);
        outline.end();
        ui.commands.extend(outline.commands);
        ui.commands
    }
}

/// The cursor for a resize edge; none elsewhere.
fn resize_cursor(region: WindowRegion) -> Option<CursorIcon> {
    match region {
        WindowRegion::Resize(ResizeEdge::North | ResizeEdge::South) => Some(CursorIcon::NsResize),
        WindowRegion::Resize(ResizeEdge::East | ResizeEdge::West) => Some(CursorIcon::EwResize),
        WindowRegion::Resize(ResizeEdge::NorthEast | ResizeEdge::SouthWest) => {
            Some(CursorIcon::NeswResize)
        }
        WindowRegion::Resize(ResizeEdge::NorthWest | ResizeEdge::SouthEast) => {
            Some(CursorIcon::NwseResize)
        }
        _ => None,
    }
}

fn direction(edge: ResizeEdge) -> ResizeDirection {
    match edge {
        ResizeEdge::North => ResizeDirection::North,
        ResizeEdge::South => ResizeDirection::South,
        ResizeEdge::East => ResizeDirection::East,
        ResizeEdge::West => ResizeDirection::West,
        ResizeEdge::NorthEast => ResizeDirection::NorthEast,
        ResizeEdge::NorthWest => ResizeDirection::NorthWest,
        ResizeEdge::SouthEast => ResizeDirection::SouthEast,
        ResizeEdge::SouthWest => ResizeDirection::SouthWest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On an edge, and while one is dragged, the frame's resize cursor is
    /// the window's: a host must not put the content's over it each frame.
    /// Everywhere else the content's cursor shows.
    #[test]
    fn the_frame_keeps_its_resize_cursor_on_its_edges() {
        let mut frame = WinitWindow::new("Store");
        assert_eq!(frame.cursor(), None, "the content's cursor, before any edge");
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        for (at, expected) in [
            (Point::new(797.0, 300.0), Some(CursorIcon::EwResize)),
            (Point::new(400.0, 598.0), Some(CursorIcon::NsResize)),
            (Point::new(798.0, 598.0), Some(CursorIcon::NwseResize)),
            (Point::new(2.0, 598.0), Some(CursorIcon::NeswResize)),
            (Point::new(400.0, 300.0), None),
        ] {
            frame.edge_cursor = resize_cursor(frame.region(bounds, at));
            assert_eq!(frame.cursor(), expected, "at {at:?}");
        }
        // Dragging an edge keeps its cursor wherever the pointer gets to.
        frame.pressed = Some(WindowRegion::Resize(ResizeEdge::East));
        frame.edge_cursor = None;
        assert_eq!(frame.cursor(), Some(CursorIcon::EwResize));
    }

    /// The window's buttons are over what the application draws on its bar.
    #[cfg(feature = "immediate")]
    #[test]
    fn the_buttons_are_drawn_over_an_application_drawn_bar() {
        use ui_kit_immediate::Ui;
        let mut frame = WinitWindow::new("Store");
        frame.chrome = frame.chrome.clone().custom_bar(64.0);
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let commands = frame.frame_in_bounds(bounds, Default::default(), &crate::WindowStyle::default(), |input, width, height| {
            assert_eq!((width, height), (800.0, 600.0), "the application has the whole window");
            let mut ui = Ui::default();
            ui.begin(input);
            ui.label(Rect::new(0.0, 0.0, 800.0, 64.0), "Bar");
            ui.end();
            ui.commands
        });
        let bar = commands.iter().position(|c| c.text == "Bar").unwrap();
        let close = frame.chrome.control_rect(bounds, crate::WindowControl::Close).unwrap();
        let button = commands.iter().rposition(|c| c.rect == close).unwrap();
        assert!(button > bar, "the close button is under the bar");
    }

    /// On a bar the application draws, its tabs and buttons take presses and
    /// the rest of the bar still moves the window.
    #[test]
    fn the_applications_controls_on_its_bar_are_not_dragged_by() {
        let mut frame = WinitWindow::new("Store");
        frame.chrome = frame.chrome.clone().custom_bar(64.0);
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let tab = Rect::new(300.0, 16.0, 80.0, 32.0);
        let tile = Rect::new(300.0, 200.0, 80.0, 32.0);
        frame.set_bar_controls(bounds, [tab, tile]);
        assert_eq!(frame.region(bounds, Point::new(320.0, 30.0)), WindowRegion::Client);
        assert_eq!(frame.region(bounds, Point::new(120.0, 30.0)), WindowRegion::Drag);
        assert_eq!(frame.bar_controls, vec![tab], "only what is on the bar counts");
    }

    #[test]
    fn attributes_preserve_host_configuration() {

        let frame = WinitWindow::new("Example");
        let attributes = frame.attributes(
            Window::default_attributes()
                .with_inner_size(winit::dpi::LogicalSize::new(640.0, 480.0))
                .with_visible(false),
        );
        assert!(!attributes.decorations);
        assert!(!attributes.visible);
        assert_eq!(attributes.title, "Example");
        assert!(attributes.inner_size.is_some());
        assert!(attributes.transparent);
    }

    #[cfg(feature = "immediate")]
    #[test]
    fn client_input_and_paint_share_coordinates_and_clip() {
        use ui_kit_immediate::{Input, Ui};
        let frame = WinitWindow::new("Example");
        let bounds = Rect {
            x: 10.0,
            y: 20.0,
            width: 640.0,
            height: 480.0,
        };
        let commands = frame.frame_in_bounds(
            bounds,
            Input {
                pointer: Some(Point::new(30.0, 78.0)),
                down: true,
                ..Default::default()
            },
            &crate::WindowStyle::default(),
            |input, width, height| {
                assert_eq!(input.pointer, Some(Point::new(20.0, 20.0)));
                assert!(input.down);
                assert_eq!((width, height), (640.0, 442.0));
                let mut ui = Ui::default();
                ui.begin(input);
                ui.label(
                    Rect {
                        x: 0.0,
                        y: -10.0,
                        width: 200.0,
                        height: 60.0,
                    },
                    "Client",
                );
                ui.end();
                ui.commands
            },
        );
        let content = commands.iter().rev().find(|c| c.text == "Client").unwrap();
        assert_eq!(content.rect.y, 48.0);
        // Drawn on a canvas zoomed in, a control lands where the pointer
        // finds it: the client's offset is not zoomed with the canvas.
        let zoomed = frame.frame_in_bounds(
            bounds,
            Input {
                pointer: Some(Point::new(30.0, 78.0)),
                ..Default::default()
            },
            &crate::WindowStyle::default(),
            |input, _, _| {
                let mut ui = Ui::default();
                ui.begin(input);
                let canvas = ui_kit_immediate::immediate::Transform::new(Point::new(5.0, 5.0), 2.0);
                let hovered = ui.with_transform(canvas, |ui| {
                    ui.label(Rect::new(5.0, 5.0, 10.0, 10.0), "Zoomed");
                    ui.interact("zoomed", Rect::new(5.0, 5.0, 10.0, 10.0)).hovered
                });
                assert!(hovered, "the pointer is over the control");
                ui.end();
                ui.commands
            },
        );
        let zoomed = zoomed.iter().find(|c| c.text == "Zoomed").unwrap();
        let on_screen = zoomed.transform.map_or(zoomed.rect, |t| t.apply_rect(zoomed.rect));
        assert!(on_screen.contains(Point::new(30.0, 78.0)), "drawn at {on_screen:?}, away from the pointer");
        // The outline is drawn last, over the content, round the whole window.
        let outline = commands.last().unwrap();
        assert_eq!(outline.rect, bounds);
        assert_eq!(outline.style.fill[3], 0.0);
        assert_eq!(content.clip.unwrap(), frame.chrome.client(bounds));
        frame.frame_in_bounds(
            bounds,
            Input {
                pointer: Some(Point::new(30.0, 30.0)),
                ..Default::default()
            },
            &crate::WindowStyle::default(),
            |input, _, _| {
                assert!(input.pointer.is_none());
                vec![]
            },
        );
    }
}
