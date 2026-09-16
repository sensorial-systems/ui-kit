// Shared semantic UI. No Dioxus, GPU, executor, or application-owned state.
pub mod graph;

use prism_ui::immediate::{Flex, Size};
pub use prism_ui::{
    immediate::{Color, Material, Point, Rect, Style},
    rich_text::{self, FormatAction, RichText, RichTextEditor, TextFormat, TextRun},
    xy_graph::{
        self, XyAxis, XyHandle, XyPlot, XyPoint, XyRange, XySeries, XyShadedRegion, XyTick,
    },
    TextAlign,
};
use std::{collections::HashSet, sync::Arc};

/// Tokens shared by both adapters. Nodes may override a complete style.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub control: Style,
    pub panel: Style,
    pub text: Style,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            control: Style::default(),
            panel: Style::default(),
            text: Style::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub padding: f32,
    pub gap: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            padding: 0.0,
            gap: 12.0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberRange {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}
impl Default for NumberRange {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            step: 0.01,
        }
    }
}
impl NumberRange {
    pub fn valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && (self.max - self.min).is_finite()
            && self.max > self.min
            && self.step.is_finite()
            && self.step > 0.0
    }
    pub fn normalize(self, value: f64) -> Option<f64> {
        if !self.valid() || !value.is_finite() {
            return None;
        }
        Some(
            (self.min
                + ((value.clamp(self.min, self.max) - self.min) / self.step).round() * self.step)
                .clamp(self.min, self.max),
        )
    }
    pub fn fraction(self, value: f64) -> f32 {
        self.normalize(value)
            .map_or(0.0, |v| ((v - self.min) / (self.max - self.min)) as f32)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Column,
    Row,
    Panel,
    Text(String),
    Button {
        label: String,
        loading: bool,
    },
    Checkbox {
        label: String,
        checked: bool,
    },
    Slider {
        label: String,
        value: f64,
        range: NumberRange,
    },
    TextInput {
        label: String,
        value: String,
        placeholder: String,
    },
    /// An on/off switch.
    Toggle {
        label: String,
        on: bool,
    },
    /// One of a list of options, chosen from a dropdown.
    Select {
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// How far along something is, from 0 to 1.
    Progress {
        label: String,
        value: f64,
    },
    /// A line between groups of content.
    Separator,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Activate,
    Toggle(bool),
    Number(f64),
    Text(String),
    /// The index of the option chosen in a [`Kind::Select`].
    Select(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub id: String,
    pub value: Value,
}
#[derive(Clone)]
pub struct Handler<A>(Arc<dyn Fn(Value) -> Option<A> + Send + Sync>);
impl<A> PartialEq for Handler<A> {
    fn eq(&self, b: &Self) -> bool {
        Arc::ptr_eq(&self.0, &b.0)
    }
}
#[derive(Clone, PartialEq)]
pub struct View<A> {
    pub id: Option<String>,
    pub kind: Kind,
    pub children: Vec<View<A>>,
    pub layout: Layout,
    pub style: Option<Style>,
    pub disabled: bool,
    pub handler: Option<Handler<A>>,
}
impl<A> View<A> {
    fn new(kind: Kind) -> Self {
        Self {
            id: None,
            kind,
            children: vec![],
            layout: Layout::default(),
            style: None,
            disabled: false,
            handler: None,
        }
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn width(mut self, value: f32) -> Self {
        self.layout.width = Some(value);
        self
    }
    pub fn height(mut self, value: f32) -> Self {
        self.layout.height = Some(value);
        self
    }
    pub fn padding(mut self, value: f32) -> Self {
        self.layout.padding = value;
        self
    }
    pub fn gap(mut self, value: f32) -> Self {
        self.layout.gap = value;
        self
    }
    pub fn styled(mut self, value: Style) -> Self {
        self.style = Some(value);
        self
    }
    pub fn range(mut self, min: f64, max: f64, step: f64) -> Self {
        if let Kind::Slider { range, .. } = &mut self.kind {
            *range = NumberRange { min, max, step }
        }
        self
    }
    pub fn loading(mut self, value: bool) -> Self {
        if let Kind::Button { loading, .. } = &mut self.kind {
            *loading = value
        }
        self
    }
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        if let Kind::TextInput { placeholder, .. } = &mut self.kind {
            *placeholder = text.into()
        }
        self
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled || matches!(self.kind, Kind::Button { loading: true, .. })
    }
    pub fn is_control(&self) -> bool {
        matches!(
            self.kind,
            Kind::Button { .. }
                | Kind::Checkbox { .. }
                | Kind::Slider { .. }
                | Kind::TextInput { .. }
                | Kind::Toggle { .. }
                | Kind::Select { .. }
        )
    }
    pub fn resolved_style<'a>(&'a self, theme: &'a Theme) -> &'a Style {
        self.style.as_ref().unwrap_or(match self.kind {
            Kind::Panel => &theme.panel,
            Kind::Text(_) => &theme.text,
            _ => &theme.control,
        })
    }
    pub fn intrinsic_height(&self) -> f32 {
        self.layout.height.unwrap_or_else(|| match self.kind {
            Kind::Column | Kind::Panel => {
                self.children
                    .iter()
                    .map(|c| c.intrinsic_height())
                    .sum::<f32>()
                    + self.layout.gap * self.children.len().saturating_sub(1) as f32
                    + 2.0 * self.layout.padding
            }
            Kind::Row => {
                self.children
                    .iter()
                    .map(|c| c.intrinsic_height())
                    .fold(0.0, f32::max)
                    + 2.0 * self.layout.padding
            }
            Kind::Text(_) | Kind::Progress { .. } => 28.0,
            Kind::Separator => 9.0,
            _ => 44.0,
        })
    }
    /// Both hosts use this dispatch path; invalid/mismatched/disabled events are ignored.
    pub fn dispatch(&self, event: &Event) -> Option<A> {
        if self.is_disabled() {
            return None;
        }
        if self.id.as_deref() == Some(&event.id) {
            let value = match (&self.kind, &event.value) {
                (Kind::Button { .. }, Value::Activate) => Value::Activate,
                (Kind::Checkbox { .. }, Value::Toggle(v)) => Value::Toggle(*v),
                (Kind::Slider { range, .. }, Value::Number(v)) => {
                    Value::Number(range.normalize(*v)?)
                }
                (Kind::TextInput { .. }, Value::Text(v)) => Value::Text(v.clone()),
                (Kind::Toggle { .. }, Value::Toggle(v)) => Value::Toggle(*v),
                (Kind::Select { options, .. }, Value::Select(i)) if *i < options.len() => {
                    Value::Select(*i)
                }
                _ => return None,
            };
            return self.handler.as_ref().and_then(|h| (h.0)(value));
        }
        self.children.iter().find_map(|child| child.dispatch(event))
    }
    pub fn validate(&self) -> Result<(), String> {
        fn walk<A>(v: &View<A>, ids: &mut HashSet<String>) -> Result<(), String> {
            if v.is_control() && v.id.as_ref().is_none_or(|s| s.is_empty()) {
                return Err("controls need nonempty IDs".into());
            }
            if let Some(id) = &v.id {
                if !ids.insert(id.clone()) {
                    return Err(format!("duplicate UI ID: {id}"));
                }
            }
            if ![
                Some(v.layout.padding),
                Some(v.layout.gap),
                v.layout.width,
                v.layout.height,
            ]
            .into_iter()
            .flatten()
            .all(|x| x.is_finite() && x >= 0.0)
            {
                return Err("invalid layout".into());
            }
            if let Kind::Slider { range, value, .. } = v.kind {
                if !range.valid() || !value.is_finite() {
                    return Err("invalid slider range/value".into());
                }
            }
            if let Kind::Progress { value, .. } = v.kind {
                if !(0.0..=1.0).contains(&value) {
                    return Err("progress must be between 0 and 1".into());
                }
            }
            if let Kind::Select {
                options, selected, ..
            } = &v.kind
            {
                if *selected >= options.len().max(1) {
                    return Err("select index out of range".into());
                }
            }
            if let Some(style) = &v.style {
                if !style.radius.is_finite()
                    || style.radius < 0.0
                    || !style.font_size.is_finite()
                    || style.font_size <= 0.0
                    || !style.text_padding.is_finite()
                    || style.text_padding < 0.0
                    || !style.stroke_width.is_finite()
                    || style.stroke_width < 0.0
                    || !(1..=1000).contains(&style.font_weight)
                    || style
                        .gradient_end
                        .is_some_and(|c| c.iter().any(|v| !v.is_finite()))
                {
                    return Err("invalid style".into());
                }
            }
            for child in &v.children {
                walk(child, ids)?
            }
            Ok(())
        }
        walk(self, &mut HashSet::new())
    }
}
impl<A: Clone + Send + Sync + 'static> View<A> {
    pub fn on_activate(mut self, action: A) -> Self {
        self.handler = Some(Handler(Arc::new(move |v| {
            matches!(v, Value::Activate).then(|| action.clone())
        })));
        self
    }
    pub fn on_toggle(mut self, f: impl Fn(bool) -> A + Send + Sync + 'static) -> Self {
        self.handler = Some(Handler(Arc::new(move |v| {
            if let Value::Toggle(v) = v {
                Some(f(v))
            } else {
                None
            }
        })));
        self
    }
    pub fn on_change(mut self, f: impl Fn(f64) -> A + Send + Sync + 'static) -> Self {
        self.handler = Some(Handler(Arc::new(move |v| {
            if let Value::Number(v) = v {
                Some(f(v))
            } else {
                None
            }
        })));
        self
    }
    /// For a [`Kind::Select`]: the index of the option chosen.
    pub fn on_select(mut self, f: impl Fn(usize) -> A + Send + Sync + 'static) -> Self {
        self.handler = Some(Handler(Arc::new(move |v| {
            if let Value::Select(i) = v {
                Some(f(i))
            } else {
                None
            }
        })));
        self
    }
    pub fn on_input(mut self, f: impl Fn(String) -> A + Send + Sync + 'static) -> Self {
        self.handler = Some(Handler(Arc::new(move |v| {
            if let Value::Text(v) = v {
                Some(f(v))
            } else {
                None
            }
        })));
        self
    }
}
pub fn text<A>(value: impl Into<String>) -> View<A> {
    View::new(Kind::Text(value.into()))
}
pub fn button<A>(id: impl Into<String>, label: impl Into<String>) -> View<A> {
    let mut v = View::new(Kind::Button {
        label: label.into(),
        loading: false,
    });
    v.id = Some(id.into());
    v
}
pub fn checkbox<A>(id: impl Into<String>, label: impl Into<String>, checked: bool) -> View<A> {
    let mut v = View::new(Kind::Checkbox {
        label: label.into(),
        checked,
    });
    v.id = Some(id.into());
    v
}
pub fn slider<A>(id: impl Into<String>, label: impl Into<String>, value: f64) -> View<A> {
    let mut v = View::new(Kind::Slider {
        label: label.into(),
        value,
        range: Default::default(),
    });
    v.id = Some(id.into());
    v
}
pub fn text_input<A>(
    id: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
) -> View<A> {
    let mut v = View::new(Kind::TextInput {
        label: label.into(),
        value: value.into(),
        placeholder: String::new(),
    });
    v.id = Some(id.into());
    v
}
pub fn toggle<A>(id: impl Into<String>, label: impl Into<String>, on: bool) -> View<A> {
    let mut v = View::new(Kind::Toggle {
        label: label.into(),
        on,
    });
    v.id = Some(id.into());
    v
}
pub fn select<A>(
    id: impl Into<String>,
    label: impl Into<String>,
    options: impl IntoIterator<Item = impl Into<String>>,
    selected: usize,
) -> View<A> {
    let mut v = View::new(Kind::Select {
        label: label.into(),
        options: options.into_iter().map(Into::into).collect(),
        selected,
    });
    v.id = Some(id.into());
    v
}
pub fn progress<A>(label: impl Into<String>, value: f64) -> View<A> {
    View::new(Kind::Progress {
        label: label.into(),
        value,
    })
}
pub fn separator<A>() -> View<A> {
    View::new(Kind::Separator)
}
pub fn column<A>(children: impl IntoIterator<Item = View<A>>) -> View<A> {
    let mut v = View::new(Kind::Column);
    v.children = children.into_iter().collect();
    v
}
pub fn row<A>(children: impl IntoIterator<Item = View<A>>) -> View<A> {
    let mut v = column(children);
    v.kind = Kind::Row;
    v
}
pub fn panel<A>(children: impl IntoIterator<Item = View<A>>) -> View<A> {
    let mut v = column(children);
    v.kind = Kind::Panel;
    v.layout.padding = 20.0;
    v
}

pub struct Placed<'a, A> {
    pub view: &'a View<A>,
    pub rect: Rect,
    pub clip: Rect,
    pub disabled: bool,
}
/// Shared supported layout: fixed or fill widths, intrinsic/fixed heights, rows/columns,
/// padding and gaps, solved by the same [`Flex`] the immediate host lays bars out with.
/// Overflow clips; scrolling and CSS-style wrapping are not yet modeled.
pub fn layout<A>(view: &View<A>, bounds: Rect) -> Vec<Placed<'_, A>> {
    fn walk<'a, A>(
        v: &'a View<A>,
        r: Rect,
        clip: Rect,
        disabled: bool,
        out: &mut Vec<Placed<'a, A>>,
    ) {
        let disabled = disabled || v.is_disabled();
        let clip = clip.intersect(r);
        out.push(Placed {
            view: v,
            rect: r,
            clip,
            disabled,
        });
        let inside = r.inset(v.layout.padding);
        // One solver for both hosts' layouts: a row shares its width between
        // the children without one, a column stacks them by intrinsic height.
        let is_row = matches!(v.kind, Kind::Row);
        let (flex, sizes): (Flex, Vec<Size>) = if is_row {
            (
                Flex::row(v.layout.gap),
                v.children
                    .iter()
                    .map(|c| c.layout.width.map_or(Size::FILL, Size::Fixed))
                    .collect(),
            )
        } else {
            (
                Flex::column(v.layout.gap),
                v.children
                    .iter()
                    .map(|c| Size::Fixed(c.intrinsic_height()))
                    .collect(),
            )
        };
        for (c, band) in v.children.iter().zip(flex.place(inside, &sizes)) {
            let rect = if is_row {
                Rect {
                    height: c.intrinsic_height(),
                    ..band
                }
            } else {
                Rect {
                    width: c.layout.width.unwrap_or(inside.width),
                    ..band
                }
            };
            walk(c, rect, clip.intersect(inside), disabled, out);
        }
    }
    let r = Rect {
        width: view.layout.width.unwrap_or(bounds.width),
        height: view.intrinsic_height(),
        ..bounds
    };
    let mut out = vec![];
    walk(view, r, bounds, false, &mut out);
    out
}
