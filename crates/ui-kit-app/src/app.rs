use crate::context::FrameContext;
use crate::fonts::Fonts;
use crate::gpu::Gpu;
use crate::images::ImageStore;
use ui_kit_immediate::{DrawCommand, Output, TextMeasure};

/// Something an application asks the runner to do to its window.
///
/// An application that lives in the system tray spends most of its time with
/// no window on screen, and has to be able to bring one back from a tray
/// click. The runner owns the window, so the application says what it wants
/// and the runner does it after the next update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowCommand {
    /// Put the window on screen, in front.
    Show,
    /// Take the window off screen. The application keeps running and keeps
    /// being updated, so a tray can still reach it.
    Hide,
    /// Bring an already visible window to the front.
    Focus,
    /// Leave the event loop.
    Exit,
}

/// Interface for applications driven by the `ui-kit-app` runner.
pub trait App {
    /// Render UI commands for the current frame.
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand>;

    /// What the last frame's [`Ui`](ui_kit_immediate::Ui) asked of the host:
    /// its cursor, its input method area, text to copy. The runner applies
    /// it to the window after each frame.
    fn output(&self) -> Option<Output> {
        None
    }

    /// Optional hook called on mouse scroll wheel input. The wheel also
    /// reaches the frame's input, where scroll containers take it.
    fn on_scroll(&mut self, _delta: f32) {}

    /// Optional hook called when the window loses focus.
    fn on_focus_lost(&mut self) {}

    /// The application is on screen: once at start, once the window and its
    /// surface exist, and again each time it comes back from
    /// [`on_suspend`](App::on_suspend). Start audio and other
    /// foreground-only work here.
    fn on_resume(&mut self) {}

    /// The application left the screen (on Android, the activity went to the
    /// background and its window is gone). Nothing is drawn until
    /// [`on_resume`](App::on_resume); stop audio and other foreground work.
    fn on_suspend(&mut self) {}

    /// Called once per frame interval, whether or not the window is visible,
    /// before any drawing. Polling, background results and tray events
    /// belong here: a hidden window still has to notice them.
    fn update(&mut self) {}

    /// Called before each drawn frame, with the pictures the renderer can
    /// draw. Upload what this frame will show with
    /// [`Ui::image`](ui_kit_immediate::Ui::image) and remove what it no
    /// longer will. Not called while the window is hidden.
    fn prepare(&mut self, _images: &mut dyn ImageStore) {}

    /// What the device has to support: features and limits beyond wgpu's
    /// defaults, for an application that runs its own GPU work on the
    /// renderer's device. Called once, with the adapter chosen for the window.
    fn device_descriptor(&self, _adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
        wgpu::DeviceDescriptor::default()
    }

    /// Called once the device exists, before the renderer is made, with
    /// everything it was made from. An application that renders content of
    /// its own keeps it, to render on the device the UI draws with.
    fn gpu_ready(&mut self, _gpu: &Gpu) {}

    /// Called once the renderer exists, before anything is measured, with
    /// its fonts: load the application's own and choose the faces the
    /// generic families stand for.
    fn fonts_ready(&mut self, _fonts: &dyn Fonts) {}

    /// Called once the renderer exists, with what measures text the way it
    /// will be drawn. Install it on the application's `Ui` so content-sized
    /// controls fit their labels rather than an estimate of them.
    fn text_measure_ready(&mut self, _measure: TextMeasure) {}

    /// The window was asked to close, by its close button or the system.
    /// Return `true` to exit, or `false` to keep running, having done
    /// something else about it -- hidden the window, or asked about unsaved
    /// work.
    fn on_close_requested(&mut self) -> bool {
        true
    }

    /// What the application wants done to its window, drained after each
    /// update.
    fn window_commands(&mut self) -> Vec<WindowCommand> {
        Vec::new()
    }

    /// Returns true if the application wishes to terminate.
    fn should_exit(&self) -> bool {
        false
    }
}

/// Adapter to allow closures `FnMut(&FrameContext) -> Vec<DrawCommand>` to implement `App`.
pub struct ClosureApp<F> {
    func: F,
}

impl<F> ClosureApp<F>
where
    F: FnMut(&FrameContext) -> Vec<DrawCommand>,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> App for ClosureApp<F>
where
    F: FnMut(&FrameContext) -> Vec<DrawCommand>,
{
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand> {
        (self.func)(ctx)
    }
}
