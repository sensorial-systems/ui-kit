//! Independent application logic and rendering contracts.
use crate::immediate::{DrawCommand, Ui};

/// Build a frame without knowing how or where it will be rendered.
pub trait UiLogic {
    fn update(&mut self, ui: &mut Ui);
}
impl<F: FnMut(&mut Ui)> UiLogic for F {
    fn update(&mut self, ui: &mut Ui) {
        self(ui);
    }
}
/// Compose logic in declaration order. Children must use distinct widget IDs.
pub struct Compose<A, B>(pub A, pub B);
impl<A: UiLogic, B: UiLogic> UiLogic for Compose<A, B> {
    fn update(&mut self, ui: &mut Ui) {
        self.0.update(ui);
        self.1.update(ui);
    }
}

/// A target can carry a GPU encoder, texture, canvas, or a recording sink.
/// Rendering never advances input state or mutates application values.
pub trait UiRenderer<Target> {
    type Error;
    fn render(&mut self, target: &mut Target, commands: &[DrawCommand]) -> Result<(), Self::Error>;
}
