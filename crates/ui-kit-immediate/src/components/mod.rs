//! Ready-made components: the immediate counterparts of the Dioxus ones in
//! `ui-kit`, drawn in the colours of the [`Ui`](crate::Ui)'s theme palette.
//!
//! Stateless components are small builders with a `show` method. Stateful
//! ones keep their state in a value the application owns and passes back
//! every frame. Popups, menus and dialogs draw over the page on their own
//! layers, so they can be shown from anywhere in a frame.
mod button;
mod card;
mod charts;
mod color_picker;
mod containers;
mod date_time_picker;
mod editable_text;
mod feedback;
mod form_flow;
mod graph;
mod input;
mod label;
mod menu;
mod navigation;
mod pipeline;
pub mod popup;
mod segmented_bar;
mod table;
pub mod theme;
mod timeline;

pub use button::{Button, ButtonVariant};
pub use card::{panel, popup_surface, Card};
pub use charts::{
    arc, AreaChart, Bar, BarChart, BarGroup, ChartOrientation, Gauge, PieChart, PieSlice, SlotLane,
    Sparkline, StackedBarChart, SERIES,
};
pub use color_picker::{
    hsv, parse_hex, ColorField, ColorFieldState, ColorPicker, ColorPickerMode, PRESETS,
};
pub use containers::{
    card_header, empty_state, section_heading, stat_row, Bill, BillItem, Browser, BrowserEvent,
    BrowserState, MetricCard, MetricChart, Modal,
};
pub use date_time_picker::{days_in_month, Calendar, DateTimePicker, DateTimePickerState};
pub use editable_text::{EditableText, RichTextMetrics};
pub use feedback::{Badge, Notification, ProgressBar, Spinner, StatusDot, StatusVariant};
pub use form_flow::{
    Answer, Condition, FormFlow, FormFlowEvent, FormFlowState, Question, QuestionKind,
};
pub use graph::{
    animated_line, arrow, elbow, FlowGraph, FlowNode, GraphCamera, GraphEdge, GraphNode,
    HierarchyGraph, HierarchyGraphModel, HierarchyNode, NetworkGraph, NetworkGraphNode, NodeShape,
};
pub use input::{swatch, Checkbox, OtpInput, Select, Slider, Switch, TextInput};
pub use label::Label;
pub use menu::{popup_height, Dropdown, Menu, MenuItem, MenuLayout, MenuState};
pub use navigation::{Sidebar, SidebarItem};
pub use pipeline::{PipelineBoard, PipelineCard, PipelineColumn, PipelineResponse, PipelineState};
pub use segmented_bar::{BarSegment, SegmentedBar};
pub use table::{
    SortDirection, Table, TableCell, TableColumn, TableOptions, TableResponse, TableSort,
};
pub use timeline::{
    Timeline, TimelineMilestone, TimelineProject, TimelineRange, TimelineState, TimelineTask,
};
