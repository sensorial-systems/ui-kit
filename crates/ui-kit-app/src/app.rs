use crate::context::FrameContext;
use ui_kit_immediate::DrawCommand;

/// Interface for applications driven by the `ui-kit-app` runner.
pub trait App {
    /// Render UI commands for the current frame.
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand>;

    /// Optional hook called on mouse scroll wheel input.
    fn on_scroll(&mut self, _delta: f32) {}

    /// Optional hook called when the window loses focus.
    fn on_focus_lost(&mut self) {}

    /// Optional hook called when the window requests a redraw or updates state.
    fn update(&mut self) {}

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
