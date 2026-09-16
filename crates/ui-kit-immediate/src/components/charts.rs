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

/// A ring gauge: an arc filled to a fraction, with a value in the middle.
///
/// The arc leaves a gap at the bottom, as a dial does, so the empty part of
/// the ring reads as the room left rather than as a broken circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gauge {
    fraction: f32,
    color: Option<Color>,
    thickness: f32,
}

impl Gauge {
    /// A gauge filled to `fraction`, from 0 to 1.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            },
            color: None,
            thickness: 10.0,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// How wide the ring is.
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness.max(1.0);
        self
    }

    /// The colour a fill reads in at `fraction`: the accent while there is
    /// room, amber past three quarters, red near full.
    pub fn level_color(fraction: f32) -> Color {
        if fraction >= 0.9 {
            super::theme::DANGER
        } else if fraction >= 0.75 {
            WARNING
        } else {
            INFO
        }
    }

    /// Draw the ring as large as fits in `rect`, with `value` in its middle
    /// and `caption` under the value.
    pub fn show(&self, ui: &mut Ui, rect: Rect, value: &str, caption: &str) {
        let p = ui.theme.palette;
        let middle = center(rect);
        let radius = rect.width.min(rect.height) * 0.5 - 1.0;
        if radius <= self.thickness {
            return;
        }
        // Three quarters of a turn, opening downwards.
        let start = 0.75 * std::f32::consts::PI;
        let sweep = 1.5 * std::f32::consts::PI;
        let color = self
            .color
            .unwrap_or_else(|| Self::level_color(self.fraction));
        arc(
            ui,
            middle,
            radius,
            self.thickness,
            start,
            start + sweep,
            p.raised,
        );
        if self.fraction > 0.0 {
            arc(
                ui,
                middle,
                radius,
                self.thickness,
                start,
                start + sweep * self.fraction,
                color,
            );
        }
        let inner = radius - self.thickness;
        let size = (inner * 0.5).clamp(12.0, 26.0);
        Label::heading(size).align(TextAlign::Center).show(
            ui,
            Rect::new(
                middle.x - inner,
                middle.y - size * 0.9,
                inner * 2.0,
                size * 1.4,
            ),
            value,
        );
        Label::muted()
            .size((size * 0.5).clamp(10.0, 13.0))
            .align(TextAlign::Center)
            .show(
                ui,
                Rect::new(middle.x - inner, middle.y + size * 0.5, inner * 2.0, 18.0),
                caption,
            );
    }
}

/// Fill the ring segment between `from` and `to` radians, `thickness` wide
/// inside `radius`.
pub fn arc(
    ui: &mut Ui,
    middle: Point,
    radius: f32,
    thickness: f32,
    from: f32,
    to: f32,
    color: Color,
) {
    if to <= from || radius <= 0.0 {
        return;
    }
    let inner = (radius - thickness).max(0.0);
    let steps = (((to - from) * radius * 0.5).ceil() as usize).clamp(4, 256);
    let at =
        |angle: f32, r: f32| Point::new(middle.x + angle.cos() * r, middle.y + angle.sin() * r);
    let mut points = Vec::with_capacity(steps * 6);
    for i in 0..steps {
        let a = from + (to - from) * i as f32 / steps as f32;
        let b = from + (to - from) * (i + 1) as f32 / steps as f32;
        let (outer_a, outer_b) = (at(a, radius), at(b, radius));
        let (inner_a, inner_b) = (at(a, inner), at(b, inner));
        points.extend_from_slice(&[inner_a, outer_a, outer_b, inner_a, outer_b, inner_b]);
    }
    fill_triangles(ui, &points, color);
}

/// A time series drawn as a line over a filled area, with faint guide lines.
///
/// Where a [`Sparkline`] shows a shape, this shows a quantity: the range can
/// be pinned, so a series that is nowhere near its ceiling looks that way,
/// and the fill gives the line weight against the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaChart {
    color: Option<Color>,
    range: Option<(f32, f32)>,
    guides: usize,
    line_width: f32,
}

impl Default for AreaChart {
    fn default() -> Self {
        Self {
            color: None,
            range: None,
            guides: 3,
            line_width: 1.5,
        }
    }
}

impl AreaChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Pin the vertical axis to `min..=max` instead of fitting the values.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.range = Some((min, max));
        self
    }

    /// How many horizontal guide lines to draw; 0 for none.
    pub fn guides(mut self, guides: usize) -> Self {
        self.guides = guides;
        self
    }

    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.5);
        self
    }

    /// The vertical extent the chart shows for `values`.
    pub fn bounds(&self, values: &[f32]) -> (f32, f32) {
        match self.range {
            Some(range) => range,
            None => {
                let min = values.iter().copied().fold(f32::INFINITY, f32::min);
                let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                if min.is_finite() && max.is_finite() && max > min {
                    (min.min(0.0), max)
                } else if max.is_finite() {
                    (0.0, max.max(1.0))
                } else {
                    (0.0, 1.0)
                }
            }
        }
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect, values: &[f32]) {
        let p = ui.theme.palette;
        let guide = super::theme::with_alpha(p.border, 0.7);
        if self.guides > 0 {
            for i in 0..=self.guides {
                let y = rect.y + rect.height * i as f32 / self.guides as f32;
                polyline(
                    ui,
                    &[Point::new(rect.x, y), Point::new(rect.right(), y)],
                    1.0,
                    guide,
                );
            }
        }
        if values.len() < 2 || rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let (min, max) = self.bounds(values);
        let span = (max - min).max(f32::EPSILON);
        let last = (values.len() - 1) as f32;
        let y_of = |v: f32| rect.bottom() - ((v - min) / span).clamp(0.0, 1.0) * rect.height;
        let points: Vec<Point> = values
            .iter()
            .enumerate()
            .map(|(i, v)| Point::new(rect.x + i as f32 / last * rect.width, y_of(*v)))
            .collect();
        let color = self.color.unwrap_or(p.accent);
        let mut area = Vec::with_capacity(points.len() * 6);
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (fa, fb) = (
                Point::new(a.x, rect.bottom()),
                Point::new(b.x, rect.bottom()),
            );
            area.extend_from_slice(&[fa, a, b, fa, b, fb]);
        }
        fill_triangles(ui, &area, super::theme::with_alpha(color, 0.18));
        polyline(ui, &points, self.line_width, color);
        if let Some(end) = points.last() {
            crate::vector::circle(ui, *end, self.line_width + 1.5, color);
        }
    }
}

/// One bar of a [`BarChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub label: String,
    pub value: f32,
    /// What the bar's length reads as, beside it.
    pub caption: String,
    pub color: Option<Color>,
}

impl Bar {
    pub fn new(label: impl Into<String>, value: f32, caption: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            caption: caption.into(),
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Horizontal bars on a shared scale, each with a label and a value beside it.
///
/// A [`StackedBarChart`] normalises every bar to the full width, which
/// compares parts within a bar. This compares the bars with each other: the
/// longest fills the width and the rest are in proportion to it.
#[derive(Clone, Debug, PartialEq)]
pub struct BarChart<'a> {
    bars: &'a [Bar],
    thickness: f32,
    gap: f32,
    label_width: f32,
    caption_width: f32,
    max: Option<f32>,
}

impl<'a> BarChart<'a> {
    pub fn new(bars: &'a [Bar]) -> Self {
        Self {
            bars,
            thickness: 10.0,
            gap: 12.0,
            label_width: 110.0,
            caption_width: 88.0,
            max: None,
        }
    }

    /// How thick each bar is, and how far apart bars are.
    pub fn bars(mut self, thickness: f32, gap: f32) -> Self {
        self.thickness = thickness;
        self.gap = gap;
        self
    }

    /// Room for the labels on the left and the captions on the right.
    pub fn columns(mut self, label_width: f32, caption_width: f32) -> Self {
        self.label_width = label_width;
        self.caption_width = caption_width;
        self
    }

    /// The value that fills the full width, instead of the longest bar.
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// How tall each row is: the bar and the text beside it.
    pub fn row_height(&self) -> f32 {
        self.thickness.max(18.0)
    }

    /// How tall the chart is.
    pub fn height(&self) -> f32 {
        let n = self.bars.len() as f32;
        n * self.row_height() + (n - 1.0).max(0.0) * self.gap
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let p = ui.theme.palette;
        let max = self
            .max
            .unwrap_or_else(|| self.bars.iter().map(|b| b.value).fold(0.0, f32::max))
            .max(f32::EPSILON);
        let mut rows = Strip::column(rect, self.gap);
        for (i, bar) in self.bars.iter().enumerate() {
            let row = rows.leading(self.row_height());
            let (label, rest) = row.split_left(self.label_width, 10.0);
            let (track, caption) = rest.split_right(self.caption_width, 10.0);
            Label::new()
                .size(13.0)
                .weight(500)
                .show(ui, label, &bar.label);
            let track = track.center_vertically(self.thickness);
            let radius = self.thickness * 0.5;
            panel(ui, track, p.raised, [0.0; 4], radius);
            let floor = if bar.value > 0.0 { self.thickness } else { 0.0 };
            let width = (track.width * (bar.value / max).clamp(0.0, 1.0)).max(floor);
            let color = bar.color.unwrap_or_else(|| SERIES[i % SERIES.len()]);
            if width > 0.0 {
                panel(ui, Rect { width, ..track }, color, [0.0; 4], radius);
            }
            Label::muted()
                .size(12.0)
                .weight(500)
                .align(TextAlign::Right)
                .show(ui, caption, &bar.caption);
        }
    }
}

/// Capacity as a row of slots: how many are in use, and how many are waiting
/// for one.
///
/// A device that runs one thing at a time with four behind it looks very
/// different from one running four with none waiting, and a pair of numbers
/// does not say so at a glance. Filled slots pulse while something is in them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotLane {
    slots: usize,
    running: usize,
    queued: usize,
    color: Option<Color>,
}

impl SlotLane {
    pub const HEIGHT: f32 = 26.0;

    pub fn new(slots: usize, running: usize, queued: usize) -> Self {
        Self {
            slots: slots.max(1),
            running,
            queued,
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Draw the lane across `rect`; `time` drives the pulse.
    pub fn show(&self, ui: &mut Ui, rect: Rect, time: f32) {
        let p = ui.theme.palette;
        let color = self.color.unwrap_or(p.accent);
        let slot_width = 34.0;
        let mut row = Strip::row(rect, 6.0);
        for i in 0..self.slots.min(16) {
            let Some(slot) = row.leading_if_room(slot_width, 0.0) else {
                break;
            };
            let slot = slot.center_vertically(Self::HEIGHT);
            if i < self.running {
                let pulse = 0.85 + 0.15 * (time * 3.0 + i as f32).sin();
                panel(
                    ui,
                    slot,
                    super::theme::with_alpha(color, pulse),
                    [0.0; 4],
                    6.0,
                );
                crate::icon::Icon::Zap.paint(ui, slot.centered(14.0, 14.0), [1.0, 1.0, 1.0, 0.95]);
            } else {
                panel(ui, slot, p.raised, p.border, 6.0);
            }
        }
        if self.queued > 0 {
            row.skip_leading(6.0);
            let text = format!("{} waiting", self.queued);
            let badge = super::feedback::Badge::new(super::feedback::StatusVariant::Warning);
            let width = badge.width(ui, &text);
            if let Some(cell) = row.leading_if_room(width, 0.0) {
                badge.show(ui, cell.center_vertically(22.0), &text);
            }
        }
    }
}
