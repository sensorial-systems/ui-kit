//! A quantity split into named parts, drawn end to end across one track.
use super::card::panel;
use super::theme::with_alpha;
use super::Label;
use crate::{Strip, Ui};
use ui_kit_core::{Color, Rect};

/// One named part of a [`SegmentedBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct BarSegment {
    pub label: String,
    pub value: f32,
    pub color: Color,
    /// What the segment stands for, shown when the pointer rests on it.
    pub hint: String,
}

impl BarSegment {
    pub fn new(label: impl Into<String>, value: f32, color: Color) -> Self {
        Self {
            label: label.into(),
            value,
            color,
            hint: String::new(),
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
}

/// Parts of a whole laid end to end, the remainder left as empty track.
///
/// A single-fill progress bar shows one number. This shows how a total is
/// made up: what is ours, what belongs to someone else, and what is still
/// free. A segment too thin to see is still given a pixel, because a slice
/// that rounds away is how a slow leak stays invisible. Segment boundaries
/// are placed in one pass so they tile without seams.
#[derive(Clone, Debug, PartialEq)]
pub struct SegmentedBar<'a> {
    segments: &'a [BarSegment],
    total: f32,
    radius: f32,
    /// What the empty part of the track is called in the legend.
    free_label: &'a str,
}

impl<'a> SegmentedBar<'a> {
    /// A bar across `total` units, filled by `segments` in order.
    pub fn new(segments: &'a [BarSegment], total: f32) -> Self {
        Self {
            segments,
            total,
            radius: 4.0,
            free_label: "Free",
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// What to call the remainder in the legend; empty hides it.
    pub fn free_label(mut self, label: &'a str) -> Self {
        self.free_label = label;
        self
    }

    /// What the segments leave unclaimed.
    pub fn free(&self) -> f32 {
        (self.total - self.segments.iter().map(|s| s.value.max(0.0)).sum::<f32>()).max(0.0)
    }

    /// Where each segment lands across `rect`, in order, skipping empty ones.
    pub fn layout(&self, rect: Rect) -> Vec<(Rect, &'a BarSegment)> {
        let mut placed = Vec::with_capacity(self.segments.len());
        if self.total <= 0.0 || rect.width <= 0.0 {
            return placed;
        }
        let mut cursor = rect.x;
        for segment in self.segments {
            if segment.value <= 0.0 {
                continue;
            }
            let span = (rect.width * (segment.value / self.total)).max(1.0);
            let end = (cursor + span).min(rect.right());
            if end <= cursor {
                continue;
            }
            placed.push((
                Rect::new(cursor, rect.y, end - cursor, rect.height),
                segment,
            ));
            cursor = end;
        }
        placed
    }

    /// Draw the bar in `rect`. Each segment shows its hint as a tooltip.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect) {
        let p = ui.theme.palette;
        panel(ui, rect, p.raised, [0.0; 4], self.radius);
        let placed = self.layout(rect);
        let count = placed.len();
        for (index, (area, segment)) in placed.into_iter().enumerate() {
            let segment_id = format!("{id}.{index}");
            let response = ui.interact(&segment_id, area);
            let color = if response.hovered {
                with_alpha(segment.color, 1.0)
            } else {
                segment.color
            };
            // Only the ends of the track are rounded, so a segment follows the
            // bar's outline without notching its neighbours.
            let first = index == 0;
            let last = index + 1 == count && area.right() >= rect.right() - 0.5;
            let radius = if first || last { self.radius } else { 0.0 };
            // A rounded segment in the middle of the track would show a sliver
            // of track through its corners; overdraw it square first.
            if (first || last) && !(first && last) {
                let square = if first {
                    Rect {
                        x: area.x + self.radius,
                        width: (area.width - self.radius).max(0.0),
                        ..area
                    }
                } else {
                    Rect {
                        width: (area.width - self.radius).max(0.0),
                        ..area
                    }
                };
                panel(ui, square, color, [0.0; 4], 0.0);
            }
            panel(ui, area, color, [0.0; 4], radius);
            if !segment.hint.is_empty() {
                ui.tooltip(&segment_id, &segment.hint);
            }
        }
    }

    /// A swatch and a figure for each segment, and for what is left, across
    /// `rect`. `format` turns a value into the text shown beside its label.
    pub fn legend(&self, ui: &mut Ui, rect: Rect, format: impl Fn(f32) -> String) {
        let p = ui.theme.palette;
        let label = Label::muted().size(12.0);
        let mut row = Strip::row(rect, 14.0);
        let mut entries: Vec<(Color, String)> = self
            .segments
            .iter()
            .map(|s| (s.color, format!("{} {}", s.label, format(s.value))))
            .collect();
        if !self.free_label.is_empty() {
            entries.push((
                p.raised,
                format!("{} {}", self.free_label, format(self.free())),
            ));
        }
        for (color, text) in entries {
            let width = 14.0 + label.width(ui, &text);
            let Some(item) = row.leading_if_room(width, 0.0) else {
                break;
            };
            let (dot, caption) = item.split_left(8.0, 6.0);
            panel(ui, dot.center_vertically(8.0), color, p.border, 2.0);
            label.show(ui, caption, &text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segments() -> Vec<BarSegment> {
        vec![
            BarSegment::new("Ours", 20.0, [0.0, 0.0, 1.0, 1.0]),
            BarSegment::new("Empty", 0.0, [0.0, 1.0, 0.0, 1.0]),
            BarSegment::new("Theirs", 70.0, [0.5, 0.5, 0.5, 1.0]),
        ]
    }

    #[test]
    fn segments_tile_the_track_in_order_and_skip_empty_ones() {
        let segments = segments();
        let bar = SegmentedBar::new(&segments, 100.0);
        let placed = bar.layout(Rect::new(0.0, 0.0, 200.0, 8.0));
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].0, Rect::new(0.0, 0.0, 40.0, 8.0));
        assert_eq!(placed[1].0, Rect::new(40.0, 0.0, 140.0, 8.0));
        assert_eq!(bar.free(), 10.0);
    }

    #[test]
    fn a_sliver_still_gets_a_pixel() {
        let tiny = [BarSegment::new("Leak", 0.001, [1.0; 4])];
        let bar = SegmentedBar::new(&tiny, 100.0);
        let placed = bar.layout(Rect::new(0.0, 0.0, 200.0, 8.0));
        assert_eq!(placed[0].0.width, 1.0);
    }
}
