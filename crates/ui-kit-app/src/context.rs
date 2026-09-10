use ui_kit_immediate::Input;

/// Environmental information provided to the application every frame.
#[derive(Clone, Debug)]
pub struct FrameContext {
    pub input: Input,
    pub width: f32,
    pub height: f32,
    pub elapsed: f32,
    pub scale: f32,
    pub dark: bool,
}

impl FrameContext {
    pub fn new(
        input: Input,
        width: f32,
        height: f32,
        elapsed: f32,
        scale: f32,
        dark: bool,
    ) -> Self {
        Self {
            input,
            width,
            height,
            elapsed,
            scale,
            dark,
        }
    }
}
