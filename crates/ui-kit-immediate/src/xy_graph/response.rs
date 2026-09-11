//! Response emitted by an XY graph widget during interaction.

use ui_kit_core::XyPoint;

/// Interaction state and events produced by an XY graph widget.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XyGraphResponse {
    pub hovered: bool,
    pub active: bool,
    pub changed: bool,
    pub dragged_handle: Option<(String, XyPoint)>,
    pub hovered_data_point: Option<XyPoint>,
}

impl XyGraphResponse {
    pub fn is_dragging(&self) -> bool {
        self.dragged_handle.is_some()
    }
}
