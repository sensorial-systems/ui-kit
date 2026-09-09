use crate::{ResizeEdge, WindowAction, WindowChrome, WindowRegion};
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
}

impl WinitWindow {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            chrome: WindowChrome::new(title),
            pointer: None,
            pressed: None,
        }
    }

    /// Compose with the caller's size, platform options, and other attributes.
    pub fn attributes(&self, attributes: WindowAttributes) -> WindowAttributes {
        attributes
            .with_title(&self.chrome.title)
            .with_decorations(false)
            .with_resizable(self.chrome.resizable)
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
        self.chrome.resizable = window.is_resizable();
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
                let region = self.chrome.hit(bounds, self.pointer.unwrap());
                window.set_cursor(match region {
                    WindowRegion::Resize(ResizeEdge::North | ResizeEdge::South) => {
                        CursorIcon::NsResize
                    }
                    WindowRegion::Resize(ResizeEdge::East | ResizeEdge::West) => {
                        CursorIcon::EwResize
                    }
                    WindowRegion::Resize(ResizeEdge::NorthEast | ResizeEdge::SouthWest) => {
                        CursorIcon::NeswResize
                    }
                    WindowRegion::Resize(ResizeEdge::NorthWest | ResizeEdge::SouthEast) => {
                        CursorIcon::NwseResize
                    }
                    _ => CursorIcon::Default,
                });
            }
            WindowEvent::CursorLeft { .. } => self.pointer = None,
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
                    .map(|p| self.chrome.hit(bounds, p))
                    .unwrap_or(WindowRegion::Client);
                if *state == ElementState::Pressed {
                    self.pressed = Some(region);
                    result.consumed = region != WindowRegion::Client;
                    match region {
                        WindowRegion::Drag => {
                            self.pressed = None;
                            window.drag_window()?;
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
                    .is_some_and(|p| self.chrome.hit(bounds, p) != WindowRegion::Client);
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
        let mut ui = ui_kit_immediate::Ui::default();
        ui.begin(ui_kit_immediate::Input {
            pointer: self.pointer,
            ..Default::default()
        });
        crate::window_chrome(&mut ui, &self.chrome, bounds, style);
        ui.end();
        for command in &mut ui.commands {
            if let Some(WindowRegion::Control(control)) = self.pressed {
                command.active =
                    command.hovered && command.text == control.glyph(self.chrome.maximized);
            }
        }
        let client = self.chrome.client(bounds);
        let mut input = input;
        input.pointer = input
            .pointer
            .filter(|p| client.contains(*p))
            .map(|p| Point::new(p.x - client.x, p.y - client.y));
        let mut commands = content(input, client.width, client.height);
        for command in &mut commands {
            command.rect.x += client.x;
            command.rect.y += client.y;
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
        ui.commands.extend(commands);
        ui.commands
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
        let content = commands.last().unwrap();
        assert_eq!(content.rect.y, 48.0);
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
