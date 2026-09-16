use super::card::panel;
use super::theme::{INFO, SUCCESS, WARNING};
use super::Label;
use crate::vector::{center, fill_triangles, polyline};
use crate::{Flex, Size, Strip, Ui};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// A thin trend line through values, scaled to fill its rect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sparkline {
    color: Option<Color>,
    width: f32,
}

impl Default for Sparkline {
    fn default() -> Self {
        Self {
            color: None,
            width: 1.0,
        }
    }
}

impl Sparkline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn line_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect, values: &[f32]) {
        if values.len() < 2 {
            return;
        }
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = (max - min).max(0.01);
        let last = (values.len() - 1) as f32;
        let points: Vec<Point> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                Point::new(
                    rect.x + i as f32 / last * rect.width,
                    rect.bottom() - (v - min) / range * rect.height,
                )
            })
            .collect();
        let color = self.color.unwrap_or(ui.theme.palette.accent);
        polyline(ui, &points, self.width, color);
    }
}

/// One slice of a [`PieChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
    pub color: Color,
}

impl PieSlice {
    pub fn new(label: impl Into<String>, value: f32, color: Color) -> Self {
        Self {
            label: label.into(),
            value,
            color,
        }
    }
}

/// A pie or donut chart with an optional label in the middle of the donut.
#[derive(Clone, Debug, PartialEq)]
pub struct PieChart<'a> {
    slices: &'a [PieSlice],
    donut: bool,
    center: Option<&'a str>,
}

impl<'a> PieChart<'a> {
    pub fn new(slices: &'a [PieSlice]) -> Self {
        Self {
            slices,
            donut: false,
            center: None,
        }
    }

    /// Cut a hole in the middle, leaving a ring a third of the radius wide.
    pub fn donut(mut self) -> Self {
        self.donut = true;
        self
    }

    /// Text in the donut's hole.
    pub fn center_label(mut self, text: &'a str) -> Self {
        self.center = Some(text);
        self
    }

    /// Draw the chart as large as fits in `rect`, centred.
    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let middle = center(rect);
        let radius = rect.width.min(rect.height) * 0.5;
        let total: f32 = self.slices.iter().map(|s| s.value.max(0.0)).sum();
        if total <= 0.0 || radius <= 0.0 {
            return;
        }
        let at =
            |angle: f32, r: f32| Point::new(middle.x + angle.cos() * r, middle.y + angle.sin() * r);
        let mut start = 0.0;
        for slice in self.slices {
            let end = start + slice.value.max(0.0) / total;
            let steps = ((end - start) * 180.0).ceil().max(1.0) as usize;
            let mut points = Vec::with_capacity(steps * 6);
            for i in 0..steps {
                let a = (start + (end - start) * i as f32 / steps as f32) * std::f32::consts::TAU;
                let b =
                    (start + (end - start) * (i + 1) as f32 / steps as f32) * std::f32::consts::TAU;
                let (outer_a, outer_b) = (at(a, radius), at(b, radius));
                if self.donut {
                    let (inner_a, inner_b) = (at(a, radius * 0.65), at(b, radius * 0.65));
                    points
                        .extend_from_slice(&[inner_a, outer_a, outer_b, inner_a, outer_b, inner_b]);
                } else {
                    points.extend_from_slice(&[middle, outer_a, outer_b]);
                }
            }
            fill_triangles(ui, &points, slice.color);
            start = end;
        }
        if let (true, Some(text)) = (self.donut, self.center) {
            // Centred both ways on the middle of the ring.
            let hole = Rect::new(middle.x, middle.y, 0.0, 0.0);
            let size = (radius * 0.45).clamp(12.0, 28.0);
            Label::new()
                .size(size)
                .weight(600)
                .align(TextAlign::Center)
                .show(
                    ui,
                    Rect::new(
                        hole.x - radius * 0.6,
                        hole.y - size * 0.75,
                        radius * 1.2,
                        size * 1.5,
                    ),
                    text,
                );
        }
    }

    /// Draw a legend of the slices, a dot and label for each, left to right.
    pub fn legend(&self, ui: &mut Ui, rect: Rect) {
        let mut row = Strip::row(rect, 16.0);
        for slice in self.slices {
            let label = Label::muted().size(13.0);
            let width = 14.0 + label.width(ui, &slice.label);
            let Some(item) = row.leading_if_room(width, 0.0) else {
                break;
            };
            let (dot, text) = item.split_left(8.0, 6.0);
            panel(ui, dot.center_vertically(8.0), slice.color, [0.0; 4], 4.0);
            label.show(ui, text, &slice.label);
        }
    }
}

/// The default series colours: blue, green, amber.
pub const SERIES: [Color; 3] = [INFO, SUCCESS, WARNING];

/// One bar of a [`StackedBarChart`]: a label and its segments.
#[derive(Clone, Debug, PartialEq)]
pub struct BarGroup {
    pub label: String,
    pub values: Vec<f32>,
}

impl BarGroup {
    pub fn new(label: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
        }
    }
}

/// Which way a [`StackedBarChart`]'s bars run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartOrientation {
    /// Bars across, each normalised to the full width.
    #[default]
    Horizontal,
    /// Columns up from a shared baseline, in absolute units.
    Vertical,
}

/// Bars made of stacked segments, one colour per series.
#[derive(Clone, Debug, PartialEq)]
pub struct StackedBarChart<'a> {
    groups: &'a [BarGroup],
    colors: &'a [Color],
    orientation: ChartOrientation,
    thickness: f32,
    gap: f32,
    label_size: f32,
    unit: f32,
}

impl<'a> StackedBarChart<'a> {
    pub fn new(groups: &'a [BarGroup]) -> Self {
        Self {
            groups,
            colors: &SERIES,
            orientation: ChartOrientation::Horizontal,
            thickness: 28.0,
            gap: 16.0,
            label_size: 100.0,
            unit: 1.0,
        }
    }

    pub fn colors(mut self, colors: &'a [Color]) -> Self {
        self.colors = colors;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.orientation = ChartOrientation::Vertical;
        self
    }

    /// How thick each bar is, and how far apart bars are.
    pub fn bars(mut self, thickness: f32, gap: f32) -> Self {
        self.thickness = thickness;
        self.gap = gap;
        self
    }

    /// For a vertical chart: how many points tall one unit of value is.
    pub fn unit(mut self, points: f32) -> Self {
        self.unit = points;
        self
    }

    /// How tall (horizontal) or wide (vertical) the chart is.
    pub fn extent(&self) -> f32 {
        let n = self.groups.len() as f32;
        match self.orientation {
            ChartOrientation::Horizontal => n * self.thickness + (n - 1.0).max(0.0) * self.gap,
            ChartOrientation::Vertical => {
                let tallest = self
                    .groups
                    .iter()
                    .map(|g| g.values.iter().sum::<f32>())
                    .fold(0.0, f32::max);
                tallest * self.unit + 30.0
            }
        }
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let color = |j: usize| self.colors[j % self.colors.len().max(1)];
        match self.orientation {
            ChartOrientation::Horizontal => {
                let mut rows = Strip::column(rect, self.gap);
                for group in self.groups {
                    let (label, bar) = rows
                        .leading(self.thickness)
                        .split_left(self.label_size, 8.0);
                    Label::new()
                        .size(13.0)
                        .weight(500)
                        .show(ui, label, &group.label);
                    let sizes: Vec<Size> = group.values.iter().map(|v| Size::Fill(*v)).collect();
                    for (j, segment) in Flex::row(0.0).place(bar, &sizes).into_iter().enumerate() {
                        panel(ui, segment, color(j), [0.0; 4], 0.0);
                    }
                }
            }
            ChartOrientation::Vertical => {
                let mut columns = Strip::row(rect, self.gap);
                for group in self.groups {
                    let (stack, year) = columns.leading(self.thickness).split_bottom(24.0, 6.0);
                    let mut stack = Strip::column(stack, 0.0);
                    for (j, value) in group.values.iter().enumerate() {
                        panel(
                            ui,
                            stack.trailing(value * self.unit),
                            color(j),
                            [0.0; 4],
                            0.0,
                        );
                    }
                    Label::muted()
                        .size(12.0)
                        .align(TextAlign::Center)
                        .show(ui, year, &group.label);
                }
            }
        }
    }
}
