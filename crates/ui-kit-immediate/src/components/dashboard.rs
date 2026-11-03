//! The pieces of a page of figures: a flat [`Surface`] to set things on,
//! a change told as a [`Delta`] pill, [`Stat`]s laid in a [`StatGrid`],
//! the pages above this one as [`Breadcrumbs`], a [`TabBar`] that scrolls
//! when its tabs don't fit, a [`Segmented`] choice of a few options, a
//! [`LinkList`] whose rows lead somewhere, and a [`PropertyList`].

use super::card::panel;
use super::theme::{is_dark, rgb, with_alpha};
use super::Label;
use crate::icon::Icon;
use crate::{vector, CursorIcon, Response, Ui};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// A flat card: white on a light page (the page's surface on a dark one),
/// with a hairline border and a faint shadow. Quieter than the frosted
/// [`Card`](super::Card), for pages of figures, charts and tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    radius: f32,
    hovered: bool,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            radius: 12.0,
            hovered: false,
        }
    }
}

impl Surface {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Drawn with a darker border, as a surface that leads somewhere is
    /// under the pointer.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let p = ui.theme.palette;
        let dark = is_dark(&p);
        if !dark {
            let shadow = Rect {
                y: rect.y + 1.0,
                ..rect
            };
            ui.shadow_box(shadow, self.radius, 3.0, [0.06, 0.09, 0.18, 0.06]);
        }
        let fill = if dark { p.surface } else { rgb(0xffffff) };
        let stroke = if self.hovered {
            with_alpha(p.text, 0.28)
        } else {
            p.border
        };
        panel(ui, rect, fill, stroke, self.radius);
    }
}

/// `rect` filled in the border color: a hairline between things.
pub fn divider(ui: &mut Ui, rect: Rect) {
    let color = ui.theme.palette.border;
    panel(ui, rect, color, [0.0; 4], 0.0);
}

/// How a figure moved, as a small tinted pill: an arrow up or down, and
/// the movement ("12.4%"), green when it's good news and red when bad;
/// grey without an arrow for news of neither kind ("New").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Delta<'a> {
    text: &'a str,
    rising: Option<bool>,
    good: Option<bool>,
}

impl<'a> Delta<'a> {
    pub const HEIGHT: f32 = 22.0;
    const PAD: f32 = 8.0;
    const ARROW: f32 = 8.0;

    /// A pill of `text` alone, grey.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            rising: None,
            good: None,
        }
    }

    /// With an arrow up (rising) or down.
    pub fn rising(mut self, rising: bool) -> Self {
        self.rising = Some(rising);
        self
    }

    /// Tinted as good news or bad.
    pub fn good(mut self, good: bool) -> Self {
        self.good = Some(good);
        self
    }

    fn label() -> Label {
        Label::new().size(12.0).weight(600)
    }

    /// The pill's fill and ink.
    fn colors(&self, ui: &Ui) -> (Color, Color) {
        let p = ui.theme.palette;
        match (self.good, is_dark(&p)) {
            (Some(true), false) => (rgb(0xe6f4ea), rgb(0x137333)),
            (Some(false), false) => (rgb(0xfce8e6), rgb(0xc5221f)),
            (Some(true), true) => (with_alpha(rgb(0x34a853), 0.18), rgb(0x81c995)),
            (Some(false), true) => (with_alpha(rgb(0xea4335), 0.18), rgb(0xf28b82)),
            (None, _) => (p.raised, p.muted),
        }
    }

    /// How wide the pill is.
    pub fn width(&self, ui: &Ui) -> f32 {
        let arrow = if self.rising.is_some() {
            Self::ARROW + 4.0
        } else {
            0.0
        };
        Self::label().width(ui, self.text) + arrow + 2.0 * Self::PAD
    }

    /// Draws the pill at the left of `rect`, centred down it.
    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let width = self.width(ui).min(rect.width);
        let pill = Rect::new(rect.x, rect.y, width, rect.height).center_vertically(Self::HEIGHT);
        let (fill, ink) = self.colors(ui);
        panel(ui, pill, fill, [0.0; 4], Self::HEIGHT / 2.0);
        let mut text = pill.inset_by(crate::Insets::symmetric(Self::PAD, 0.0));
        if let Some(rising) = self.rising {
            let (mark, rest) = text.split_left(Self::ARROW, 4.0);
            let (cx, cy) = (mark.x + mark.width / 2.0, mark.y + mark.height / 2.0);
            let (w, h) = (Self::ARROW / 2.0, Self::ARROW * 0.35);
            let triangle = if rising {
                [
                    Point::new(cx - w, cy + h),
                    Point::new(cx + w, cy + h),
                    Point::new(cx, cy - h),
                ]
            } else {
                [
                    Point::new(cx - w, cy - h),
                    Point::new(cx + w, cy - h),
                    Point::new(cx, cy + h),
                ]
            };
            vector::fill_triangles(ui, &triangle, ink);
            text = rest;
        }
        Self::label()
            .color(ink)
            .ellipsis()
            .show(ui, text, self.text);
    }
}

/// One part of a [`Stat`]'s figure: whose it is, in its color, its share
/// (`amount`, which sizes it on the bar) and its figure as text.
#[derive(Clone, Debug, PartialEq)]
pub struct StatPart {
    pub label: String,
    pub color: Color,
    pub amount: f32,
    pub value: String,
}

impl StatPart {
    pub fn new(
        label: impl Into<String>,
        color: Color,
        amount: f32,
        value: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            amount,
            value: value.into(),
        }
    }
}

/// A figure: what it is, the figure large, how it moved and against what,
/// a note, and the parts it's made of as a bar with a line for each.
/// It draws no surface of its own: set it in a [`StatGrid`]'s cell, or on
/// a [`Surface`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stat<'a> {
    label: &'a str,
    value: &'a str,
    delta: Option<Delta<'a>>,
    against: Option<&'a str>,
    note: Option<&'a str>,
    parts: &'a [StatPart],
}

impl<'a> Stat<'a> {
    const LABEL_LINE: f32 = 18.0;
    const VALUE: f32 = 36.0;
    const NOTE_LINE: f32 = 16.0;
    const PART: f32 = 20.0;
    const BAR: f32 = 6.0;

    pub fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            delta: None,
            against: None,
            note: None,
            parts: &[],
        }
    }

    pub fn delta(mut self, delta: Delta<'a>) -> Self {
        self.delta = Some(delta);
        self
    }

    /// What the change is against, after its pill where it fits: "vs.
    /// the 30 days before".
    pub fn against(mut self, against: &'a str) -> Self {
        self.against = Some(against);
        self
    }

    /// A line or two under the figure: what it's over, what it leaves out.
    pub fn note(mut self, note: &'a str) -> Self {
        self.note = Some(note);
        self
    }

    /// The parts the figure is made of; with fewer than two, none are shown.
    pub fn parts(mut self, parts: &'a [StatPart]) -> Self {
        self.parts = parts;
        self
    }

    fn caption() -> Label {
        Label::muted().size(13.0).weight(500)
    }

    fn figure() -> Label {
        Label::heading(26.0)
    }

    fn small() -> Label {
        Label::muted().size(12.0)
    }

    fn shown_parts(&self) -> &'a [StatPart] {
        if self.parts.len() > 1 {
            self.parts
        } else {
            &[]
        }
    }

    /// How tall it is `width` wide.
    pub fn height(&self, ui: &Ui, width: f32) -> f32 {
        let label = clamp_lines(wrap(ui, &Self::caption(), self.label, width), 2).len() as f32;
        let mut height = label * Self::LABEL_LINE + 2.0 + Self::VALUE;
        if self.delta.is_some() {
            height += 4.0 + Delta::HEIGHT;
        }
        if let Some(note) = self.note {
            let lines = clamp_lines(wrap(ui, &Self::small(), note, width), 4).len() as f32;
            height += 6.0 + lines * Self::NOTE_LINE;
        }
        let parts = self.shown_parts();
        if !parts.is_empty() {
            height += 14.0 + Self::BAR + 6.0 + parts.len() as f32 * Self::PART;
        }
        height
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let mut y = rect.y;
        let label = clamp_lines(wrap(ui, &Self::caption(), self.label, rect.width), 2);
        for line in &label {
            Self::caption().ellipsis().show(
                ui,
                Rect::new(rect.x, y, rect.width, Self::LABEL_LINE),
                line,
            );
            y += Self::LABEL_LINE;
        }
        y += 2.0;
        Self::figure().ellipsis().show(
            ui,
            Rect::new(rect.x, y, rect.width, Self::VALUE),
            self.value,
        );
        y += Self::VALUE;
        if let Some(delta) = self.delta {
            y += 4.0;
            let row = Rect::new(rect.x, y, rect.width, Delta::HEIGHT);
            delta.show(ui, row);
            let used = delta.width(ui) + 6.0;
            if let Some(against) = self.against {
                let rest = row.width - used;
                if Self::small().width(ui, against) <= rest {
                    Self::small().show(ui, row.split_left(used, 0.0).1, against);
                }
            }
            y += Delta::HEIGHT;
        }
        if let Some(note) = self.note {
            y += 6.0;
            for line in clamp_lines(wrap(ui, &Self::small(), note, rect.width), 4) {
                Self::small().ellipsis().show(
                    ui,
                    Rect::new(rect.x, y, rect.width, Self::NOTE_LINE),
                    &line,
                );
                y += Self::NOTE_LINE;
            }
        }
        let parts = self.shown_parts();
        if parts.is_empty() {
            return;
        }
        y += 14.0;
        share_bar(ui, Rect::new(rect.x, y, rect.width, Self::BAR), parts);
        y += Self::BAR + 6.0;
        let value = Label::new().size(12.0).weight(500).align(TextAlign::Right);
        for part in parts {
            let row = Rect::new(rect.x, y, rect.width, Self::PART);
            let width = (value.width(ui, &part.value) + 2.0).min(row.width * 0.6);
            let (name, figure) = row.split_right(width, 8.0);
            let (mark, words) = name.split_left(8.0, 6.0);
            let center = Point::new(mark.x + 4.0, mark.y + mark.height / 2.0);
            vector::circle(ui, center, 3.5, part.color);
            Self::small().ellipsis().show(ui, words, &part.label);
            value.show(ui, figure, &part.value);
            y += Self::PART;
        }
    }
}

/// Parts laid end to end across `rect`, each as wide as its share, a
/// sliver apart; a part with none takes no room.
fn share_bar(ui: &mut Ui, rect: Rect, parts: &[StatPart]) {
    let track = ui.theme.palette.raised;
    panel(ui, rect, track, [0.0; 4], rect.height / 2.0);
    let total: f32 = parts.iter().map(|p| p.amount.max(0.0)).sum();
    if total <= 0.0 {
        return;
    }
    let shown: Vec<&StatPart> = parts.iter().filter(|p| p.amount > 0.0).collect();
    let gaps = 2.0 * shown.len().saturating_sub(1) as f32;
    let room = (rect.width - gaps).max(0.0);
    let mut x = rect.x;
    for part in shown {
        let width = (part.amount / total * room).max(2.0);
        let piece = Rect::new(x, rect.y, width.min(rect.right() - x), rect.height);
        panel(ui, piece, part.color, [0.0; 4], rect.height / 2.0);
        x += width + 2.0;
    }
}

/// `text` broken into lines `width` wide in `label`'s style.
fn wrap(ui: &Ui, label: &Label, text: &str, width: f32) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    ui.wrap_lines(text, &label.style(ui), width.max(20.0))
}

/// At most `most` of `lines`, the last of them holding the rest (to end
/// in an ellipsis where it's cut).
fn clamp_lines(mut lines: Vec<String>, most: usize) -> Vec<String> {
    if lines.len() > most {
        let rest = lines.split_off(most - 1).join(" ");
        lines.push(rest);
    }
    lines
}

/// Cells in rows on one [`Surface`], hairlines between them: figures read
/// together, as many abreast as fit and balanced across the rows (six
/// that fit five abreast go three and three).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatGrid {
    min_width: f32,
    max_columns: usize,
}

impl StatGrid {
    /// The padding in each cell, across and down.
    pub const PAD: (f32, f32) = (20.0, 18.0);

    /// Cells at least `min_width` wide.
    pub fn new(min_width: f32) -> Self {
        Self {
            min_width,
            max_columns: 6,
        }
    }

    pub fn max_columns(mut self, columns: usize) -> Self {
        self.max_columns = columns.max(1);
        self
    }

    /// How many cells abreast, for `count` of them `width` wide.
    pub fn columns(&self, width: f32, count: usize) -> usize {
        let fit = ((width / self.min_width).floor() as usize).clamp(1, self.max_columns);
        let count = count.max(1);
        let rows = count.div_ceil(fit);
        count.div_ceil(rows)
    }

    /// How wide a cell's content is.
    pub fn content_width(&self, width: f32, count: usize) -> f32 {
        width / self.columns(width, count) as f32 - 2.0 * Self::PAD.0
    }

    /// Each row's height, for cells whose contents are `heights` tall.
    fn rows(&self, width: f32, heights: &[f32]) -> Vec<f32> {
        heights
            .chunks(self.columns(width, heights.len()))
            .map(|row| row.iter().copied().fold(0.0, f32::max) + 2.0 * Self::PAD.1)
            .collect()
    }

    /// How tall the grid is `width` wide.
    pub fn height(&self, width: f32, heights: &[f32]) -> f32 {
        self.rows(width, heights).iter().sum()
    }

    /// Draws the surface and the hairlines between cells whose contents
    /// are `heights` tall; returns where each cell's content goes, in
    /// their order.
    pub fn show(&self, ui: &mut Ui, rect: Rect, heights: &[f32]) -> Vec<Rect> {
        Surface::new().show(ui, rect);
        let columns = self.columns(rect.width, heights.len());
        let cell_width = rect.width / columns as f32;
        let mut cells = Vec::with_capacity(heights.len());
        let mut y = rect.y;
        for (r, (row, height)) in heights
            .chunks(columns)
            .zip(self.rows(rect.width, heights))
            .enumerate()
        {
            if r > 0 {
                divider(ui, Rect::new(rect.x, y, rect.width, 1.0));
            }
            for c in 0..row.len() {
                let x = rect.x + c as f32 * cell_width;
                if c > 0 {
                    divider(ui, Rect::new(x, y + 14.0, 1.0, height - 28.0));
                }
                cells.push(Rect::new(
                    x + Self::PAD.0,
                    y + Self::PAD.1,
                    cell_width - 2.0 * Self::PAD.0,
                    height - 2.0 * Self::PAD.1,
                ));
            }
            y += height;
        }
        cells
    }
}

/// The pages above this one, each leading back to its own, a chevron
/// between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breadcrumbs<'a> {
    items: &'a [&'a str],
}

impl<'a> Breadcrumbs<'a> {
    pub const HEIGHT: f32 = 20.0;

    pub fn new(items: &'a [&'a str]) -> Self {
        Self { items }
    }

    /// Draws them along `rect`; returns the one the reader picks.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect) -> Option<usize> {
        let label = Label::new().size(13.0).weight(500);
        let palette = ui.theme.palette;
        let mut picked = None;
        let mut x = rect.x;
        for (i, item) in self.items.iter().enumerate() {
            if x >= rect.right() - 12.0 {
                break;
            }
            let width = label.width(ui, item).min(rect.right() - x);
            let place = Rect::new(x, rect.y, width, rect.height);
            let response = ui.interact(&format!("{id}.{i}"), place);
            if response.hovered {
                ui.set_cursor(CursorIcon::Pointer);
            }
            let color = if response.hovered {
                palette.text
            } else {
                palette.muted
            };
            label.color(color).ellipsis().show(ui, place, item);
            if response.clicked {
                picked = Some(i);
            }
            x += width + 4.0;
            let chevron = Rect::new(x, rect.y, 14.0, rect.height).center_vertically(14.0);
            Icon::ChevronRight.paint(ui, chevron, with_alpha(palette.muted, 0.7));
            x += 18.0;
        }
        picked
    }
}

/// What a [`TabBar`] keeps between frames: how far it's scrolled, and
/// whether the pointer that holds it has dragged it (so letting go
/// chooses nothing).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TabBarState {
    pub scroll: f32,
    dragged: bool,
}

/// Tabs in a row over a hairline, the chosen one in the page's ink and
/// underlined in the accent. When they don't fit they scroll sideways:
/// dragged, and brought into sight when chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabBar<'a> {
    labels: &'a [&'a str],
}

impl<'a> TabBar<'a> {
    pub const HEIGHT: f32 = 42.0;
    const PAD: f32 = 12.0;

    pub fn new(labels: &'a [&'a str]) -> Self {
        Self { labels }
    }

    fn label() -> Label {
        Label::new().size(14.0).weight(500)
    }

    /// How wide the tabs are together.
    pub fn width(&self, ui: &Ui) -> f32 {
        self.widths(ui).iter().sum()
    }

    fn widths(&self, ui: &Ui) -> Vec<f32> {
        self.labels
            .iter()
            .map(|l| Self::label().width(ui, l) + 2.0 * Self::PAD)
            .collect()
    }

    /// Draws the tabs along `rect`; `selected` changes (and the response
    /// says so) when the reader picks another.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        selected: &mut usize,
        state: &mut TabBarState,
    ) -> Response {
        let palette = ui.theme.palette;
        let widths = self.widths(ui);
        let overflow = (widths.iter().sum::<f32>() - rect.width).max(0.0);
        state.scroll = state.scroll.clamp(0.0, overflow);
        divider(ui, Rect::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0));
        let mut response = Response::default();
        let mut held = false;
        let mut chosen = None;
        let scroll = state.scroll;
        ui.with_clip(rect, |ui| {
            let mut x = rect.x - scroll;
            for (i, (label, width)) in self.labels.iter().zip(&widths).enumerate() {
                let tab = Rect::new(x, rect.y, *width, rect.height);
                x += width;
                let left = tab.x.max(rect.x);
                let right = tab.right().min(rect.right());
                if right <= left {
                    continue;
                }
                let r = ui.interact(
                    &format!("{id}.{i}"),
                    Rect::new(left, tab.y, right - left, tab.height),
                );
                response.hovered |= r.hovered;
                if r.hovered {
                    ui.set_cursor(CursorIcon::Pointer);
                }
                if r.drag_delta.x != 0.0 && overflow > 0.0 {
                    state.scroll = (state.scroll - r.drag_delta.x).clamp(0.0, overflow);
                    state.dragged = true;
                }
                held |= r.active;
                if r.clicked && !state.dragged {
                    chosen = Some(i);
                }
                let on = *selected == i;
                let color = if on || r.hovered {
                    palette.text
                } else {
                    palette.muted
                };
                let look = if on {
                    Self::label().weight(600)
                } else {
                    Self::label()
                };
                look.color(color).align(TextAlign::Center).show(
                    ui,
                    tab.split_bottom(2.0, 0.0).0,
                    label,
                );
                if on {
                    let mark = Rect::new(tab.x + 8.0, tab.bottom() - 2.0, tab.width - 16.0, 2.0);
                    panel(ui, mark, palette.accent, [0.0; 4], 1.0);
                }
            }
        });
        if !held {
            state.dragged = false;
        }
        if let Some(i) = chosen.filter(|i| i != selected) {
            *selected = i;
            response.changed = true;
            // Bring it into sight, with a little of its neighbours.
            let left: f32 = widths[..i].iter().sum();
            let right = left + widths[i];
            if left - 24.0 < state.scroll {
                state.scroll = (left - 24.0).max(0.0);
            } else if right + 24.0 > state.scroll + rect.width {
                state.scroll = (right + 24.0 - rect.width).min(overflow);
            }
        }
        response
    }
}

/// A compact choice of a few options: a pill with the chosen one raised
/// on it. Each option is as wide as its label, or wider to fill a wider
/// rect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segmented<'a> {
    options: &'a [&'a str],
}

impl<'a> Segmented<'a> {
    pub const HEIGHT: f32 = 32.0;
    const INSET: f32 = 3.0;
    const PAD: f32 = 12.0;

    pub fn new(options: &'a [&'a str]) -> Self {
        Self { options }
    }

    fn label() -> Label {
        Label::new().size(13.0).weight(500)
    }

    fn widths(&self, ui: &Ui) -> Vec<f32> {
        self.options
            .iter()
            .map(|o| Self::label().weight(600).width(ui, o) + 2.0 * Self::PAD)
            .collect()
    }

    /// How wide it is at its narrowest.
    pub fn width(&self, ui: &Ui) -> f32 {
        self.widths(ui).iter().sum::<f32>() + 2.0 * Self::INSET
    }

    /// Draws it in `rect` (centred down it); `selected` changes, and the
    /// response says so, when the reader picks another option.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, selected: &mut usize) -> Response {
        let palette = ui.theme.palette;
        let dark = is_dark(&palette);
        let rect = rect.center_vertically(Self::HEIGHT);
        let track = if dark { palette.raised } else { rgb(0xe9ebef) };
        panel(ui, rect, track, [0.0; 4], 8.0);
        let widths = self.widths(ui);
        let natural: f32 = widths.iter().sum();
        let room = rect.width - 2.0 * Self::INSET;
        let stretch = if natural > 0.0 { room / natural } else { 1.0 };
        let mut response = Response::default();
        let mut x = rect.x + Self::INSET;
        for (i, (option, width)) in self.options.iter().zip(widths).enumerate() {
            let width = width * stretch;
            let place = Rect::new(
                x,
                rect.y + Self::INSET,
                width,
                Self::HEIGHT - 2.0 * Self::INSET,
            );
            x += width;
            let r = ui.interact(&format!("{id}.{i}"), place);
            response.hovered |= r.hovered;
            if r.hovered {
                ui.set_cursor(CursorIcon::Pointer);
            }
            let on = *selected == i;
            if on {
                if !dark {
                    ui.shadow_box(
                        Rect {
                            y: place.y + 1.0,
                            ..place
                        },
                        6.0,
                        2.0,
                        [0.06, 0.09, 0.18, 0.12],
                    );
                }
                let fill = if dark { rgb(0x3f3f46) } else { rgb(0xffffff) };
                panel(ui, place, fill, [0.0; 4], 6.0);
            }
            let color = if on || r.hovered {
                palette.text
            } else {
                palette.muted
            };
            let look = if on {
                Self::label().weight(600)
            } else {
                Self::label()
            };
            look.color(color)
                .align(TextAlign::Center)
                .ellipsis()
                .show(ui, place, option);
            if r.clicked && !on {
                *selected = i;
                response.changed = true;
            }
        }
        response
    }
}

/// One row of a [`LinkList`]: a title in an optional color, a line under
/// it, and its figures by caption.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListRow {
    pub title: String,
    pub color: Option<Color>,
    pub subtitle: Option<String>,
    /// (caption, figure): a wide list shows each caption once, over its
    /// column.
    pub figures: Vec<(String, String)>,
    /// Whether it leads somewhere: a chevron, and the pointer's hand.
    pub link: bool,
}

impl ListRow {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn figure(mut self, caption: impl Into<String>, figure: impl Into<String>) -> Self {
        self.figures.push((caption.into(), figure.into()));
        self
    }

    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }
}

/// Rows on one [`Surface`], hairlines between them, each leading
/// somewhere. Wide, the figures stand in columns under their captions, as
/// a table; narrow, they go two abreast under each row's title.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkList<'a> {
    rows: &'a [ListRow],
}

impl<'a> LinkList<'a> {
    const PAD: f32 = 16.0;
    const HEADER: f32 = 36.0;
    const WIDE_ROW: f32 = 60.0;
    const CHEVRON: f32 = 16.0;
    const FIGURE: f32 = 36.0;

    pub fn new(rows: &'a [ListRow]) -> Self {
        Self { rows }
    }

    fn title() -> Label {
        Label::heading(15.0)
    }

    fn caption() -> Label {
        Label::muted().size(12.0).weight(500)
    }

    fn figure() -> Label {
        Label::new().size(14.0).weight(500)
    }

    /// Every caption the rows have, in the order they first come.
    fn captions(&self) -> Vec<&'a str> {
        let mut captions: Vec<&str> = Vec::new();
        for row in self.rows {
            for (caption, _) in &row.figures {
                if !captions.contains(&caption.as_str()) {
                    captions.push(caption);
                }
            }
        }
        captions
    }

    /// Each column's width when wide, or `None` when they don't fit
    /// `width` beside the titles.
    fn columns(&self, ui: &Ui, width: f32) -> Option<Vec<f32>> {
        let captions = self.captions();
        let widths: Vec<f32> = captions
            .iter()
            .map(|caption| {
                let figures = self
                    .rows
                    .iter()
                    .flat_map(|r| r.figures.iter().filter(|(c, _)| c == caption))
                    .map(|(_, f)| Self::figure().width(ui, f))
                    .fold(Self::caption().width(ui, caption), f32::max);
                figures.max(72.0) + 28.0
            })
            .collect();
        let titles = self
            .rows
            .iter()
            .map(|r| Self::title().width(ui, &r.title))
            .fold(140.0_f32, f32::max)
            .min(240.0);
        let room = width - 2.0 * Self::PAD - Self::CHEVRON - 20.0 - titles;
        (widths.iter().sum::<f32>() <= room).then_some(widths)
    }

    /// Where each of a narrow row's figures goes, `width` wide: its line
    /// and whether it's in the right half; a figure too long for half the
    /// width has a line of its own. Also how many lines they take.
    fn narrow_cells(&self, ui: &Ui, row: &ListRow, width: f32) -> (Vec<(usize, bool)>, usize) {
        let half = width / 2.0 - 12.0;
        let mut cells = Vec::with_capacity(row.figures.len());
        let (mut line, mut right) = (0, false);
        for (caption, figure) in &row.figures {
            let wide = Self::figure().width(ui, figure) > half
                || Self::caption().width(ui, caption) > half;
            if wide && right {
                (line, right) = (line + 1, false);
            }
            cells.push((line, right));
            if wide || right {
                (line, right) = (line + 1, false);
            } else {
                right = true;
            }
        }
        let lines = if right { line + 1 } else { line };
        (cells, lines)
    }

    fn narrow_row(&self, ui: &Ui, row: &ListRow, width: f32) -> f32 {
        let subtitle = if row.subtitle.is_some() { 18.0 } else { 0.0 };
        let (_, lines) = self.narrow_cells(ui, row, Self::narrow_width(row, width));
        let figures = lines as f32 * Self::FIGURE;
        let figures = if figures > 0.0 { 10.0 + figures } else { 0.0 };
        2.0 * 14.0 + 22.0 + subtitle + figures
    }

    /// How wide a narrow row's figures are, in a list `width` wide.
    fn narrow_width(row: &ListRow, width: f32) -> f32 {
        let indent = if row.color.is_some() { 18.0 } else { 0.0 };
        width - 2.0 * Self::PAD - Self::CHEVRON - 12.0 - indent
    }

    /// How tall the list is `width` wide.
    pub fn height(&self, ui: &Ui, width: f32) -> f32 {
        match self.columns(ui, width) {
            Some(_) => Self::HEADER + self.rows.len() as f32 * Self::WIDE_ROW,
            None => self
                .rows
                .iter()
                .map(|r| self.narrow_row(ui, r, width))
                .sum(),
        }
    }

    /// Draws the list in `rect`; returns the row the reader picks.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect) -> Option<usize> {
        Surface::new().show(ui, rect);
        let palette = ui.theme.palette;
        let columns = self.columns(ui, rect.width);
        let captions = self.captions();
        let mut y = rect.y;
        let mut picked = None;
        if let Some(widths) = &columns {
            let mut right = rect.right() - Self::PAD - Self::CHEVRON - 12.0;
            for (caption, width) in captions.iter().zip(widths).rev() {
                let place = Rect::new(right - width, y, *width, Self::HEADER);
                Self::caption()
                    .align(TextAlign::Right)
                    .show(ui, place, caption);
                right -= width;
            }
            y += Self::HEADER;
            divider(ui, Rect::new(rect.x, y - 1.0, rect.width, 1.0));
        }
        for (i, row) in self.rows.iter().enumerate() {
            let height = match columns {
                Some(_) => Self::WIDE_ROW,
                None => self.narrow_row(ui, row, rect.width),
            };
            let place = Rect::new(rect.x, y, rect.width, height);
            if i > 0 {
                divider(
                    ui,
                    Rect::new(rect.x + Self::PAD, y, rect.width - 2.0 * Self::PAD, 1.0),
                );
            }
            let mut hovered = false;
            if row.link {
                let r = ui.interact(&format!("{id}.{i}"), place);
                hovered = r.hovered;
                if hovered {
                    ui.set_cursor(CursorIcon::Pointer);
                    // The first and last rows keep the surface's corners.
                    let wash = place.inset_by(crate::Insets::symmetric(1.0, 1.0));
                    panel(ui, wash, with_alpha(palette.text, 0.03), [0.0; 4], 10.0);
                }
                if r.clicked {
                    picked = Some(i);
                }
            }
            let inner = place.inset_by(crate::Insets::symmetric(Self::PAD, 0.0));
            let (body, chevron) = inner.split_right(Self::CHEVRON, 12.0);
            if row.link {
                let color = if hovered {
                    palette.text
                } else {
                    with_alpha(palette.muted, 0.8)
                };
                let top = match columns {
                    Some(_) => chevron.center_vertically(Self::CHEVRON),
                    None => Rect::new(
                        chevron.x,
                        place.y + 14.0 + 3.0,
                        Self::CHEVRON,
                        Self::CHEVRON,
                    ),
                };
                Icon::ChevronRight.paint(ui, top, color);
            }
            match &columns {
                Some(widths) => {
                    let total: f32 = widths.iter().sum();
                    let (left, mut figures) = body.split_right(total, 12.0);
                    let lines = if row.subtitle.is_some() { 40.0 } else { 22.0 };
                    let mut text = left.center_vertically(lines);
                    text = self.lead(ui, text, row);
                    if let Some(subtitle) = &row.subtitle {
                        Label::muted()
                            .size(12.0)
                            .ellipsis()
                            .show(ui, text, subtitle);
                    }
                    for (caption, width) in captions.iter().zip(widths) {
                        let (cell, rest) = figures.split_left(*width, 0.0);
                        figures = rest;
                        let figure = row
                            .figures
                            .iter()
                            .find(|(c, _)| c == caption)
                            .map_or("—", |(_, f)| f);
                        let look = if figure == "—" {
                            Self::figure().color(palette.muted)
                        } else {
                            Self::figure()
                        };
                        look.align(TextAlign::Right)
                            .ellipsis()
                            .show(ui, cell, figure);
                    }
                }
                None => {
                    let text = Rect::new(body.x, place.y + 14.0, body.width, height - 28.0);
                    let rest = self.lead(ui, text, row);
                    let mut top = rest.y;
                    if let Some(subtitle) = &row.subtitle {
                        Label::muted().size(12.0).ellipsis().show(
                            ui,
                            Rect::new(rest.x, top, rest.width, 18.0),
                            subtitle,
                        );
                        top += 18.0;
                    }
                    if !row.figures.is_empty() {
                        top += 10.0;
                        let width = Self::narrow_width(row, rect.width);
                        let (cells, _) = self.narrow_cells(ui, row, width);
                        let half = width / 2.0;
                        for ((caption, figure), (line, right)) in row.figures.iter().zip(cells) {
                            let x = rest.x + if right { half } else { 0.0 };
                            let y = top + line as f32 * Self::FIGURE;
                            let cell = Rect::new(x, y, rest.right() - x, Self::FIGURE);
                            let (over, under) = cell.split_top(16.0, 0.0);
                            Self::caption().ellipsis().show(ui, over, caption);
                            Self::figure().ellipsis().show(
                                ui,
                                under.split_top(18.0, 0.0).0,
                                figure,
                            );
                        }
                    }
                }
            }
            y += height;
        }
        picked
    }

    /// The row's color dot and title along the top of `rect`; returns the
    /// rest under them, from where the title starts.
    fn lead(&self, ui: &mut Ui, rect: Rect, row: &ListRow) -> Rect {
        let (top, rest) = rect.split_top(22.0, 0.0);
        let mut title = top;
        let mut indent = 0.0;
        if let Some(color) = row.color {
            let (mark, words) = title.split_left(8.0, 10.0);
            vector::circle(
                ui,
                Point::new(mark.x + 4.0, mark.y + mark.height / 2.0),
                4.0,
                color,
            );
            title = words;
            indent = 18.0;
        }
        Self::title().ellipsis().show(ui, title, &row.title);
        Rect::new(rect.x + indent, rest.y, rect.width - indent, rest.height)
    }
}

/// Properties on a [`Surface`]: a caption on the left and its value on
/// the right, wrapping as it needs, hairlines between them; under a title
/// in an optional color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyList<'a> {
    title: Option<&'a str>,
    color: Option<Color>,
    rows: &'a [(&'a str, &'a str)],
}

impl<'a> PropertyList<'a> {
    const PAD: f32 = 16.0;
    const TITLE: f32 = 48.0;
    const LINE: f32 = 18.0;
    const ROW_PAD: f32 = 10.0;

    pub fn new(rows: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            title: None,
            color: None,
            rows,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    fn caption() -> Label {
        Label::muted().size(13.0)
    }

    fn value() -> Label {
        Label::new().size(13.0).weight(500).align(TextAlign::Right)
    }

    /// Each row's caption width and value lines, `width` wide.
    fn laid(&self, ui: &Ui, width: f32) -> Vec<(f32, Vec<String>)> {
        let inner = width - 2.0 * Self::PAD;
        self.rows
            .iter()
            .map(|(caption, value)| {
                let caption = Self::caption().width(ui, caption).min(inner * 0.45) + 16.0;
                let lines = wrap(ui, &Self::value(), value, inner - caption);
                (
                    caption,
                    if lines.is_empty() {
                        vec![String::new()]
                    } else {
                        lines
                    },
                )
            })
            .collect()
    }

    pub fn height(&self, ui: &Ui, width: f32) -> f32 {
        let title = if self.title.is_some() {
            Self::TITLE
        } else {
            6.0
        };
        let rows: f32 = self
            .laid(ui, width)
            .iter()
            .map(|(_, lines)| lines.len() as f32 * Self::LINE + 2.0 * Self::ROW_PAD)
            .sum();
        title + rows + 6.0
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        Surface::new().show(ui, rect);
        let inner = rect.inset_by(crate::Insets::symmetric(Self::PAD, 0.0));
        let mut y = rect.y + 6.0;
        if let Some(title) = self.title {
            let mut place = Rect::new(inner.x, rect.y, inner.width, Self::TITLE);
            if let Some(color) = self.color {
                let (mark, words) = place.split_left(8.0, 10.0);
                vector::circle(
                    ui,
                    Point::new(mark.x + 4.0, mark.y + mark.height / 2.0),
                    4.0,
                    color,
                );
                place = words;
            }
            Label::heading(15.0).ellipsis().show(ui, place, title);
            y = rect.y + Self::TITLE;
            divider(ui, Rect::new(rect.x, y - 1.0, rect.width, 1.0));
        }
        for (i, ((caption, _), (width, lines))) in
            self.rows.iter().zip(self.laid(ui, rect.width)).enumerate()
        {
            if i > 0 {
                divider(ui, Rect::new(inner.x, y, inner.width, 1.0));
            }
            let height = lines.len() as f32 * Self::LINE + 2.0 * Self::ROW_PAD;
            let row = Rect::new(
                inner.x,
                y + Self::ROW_PAD,
                inner.width,
                height - 2.0 * Self::ROW_PAD,
            );
            let (left, right) = row.split_left(width, 0.0);
            Self::caption()
                .ellipsis()
                .show(ui, left.split_top(Self::LINE, 0.0).0, caption);
            for (k, line) in lines.iter().enumerate() {
                let place = Rect::new(
                    right.x,
                    right.y + k as f32 * Self::LINE,
                    right.width,
                    Self::LINE,
                );
                Self::value().show(ui, place, line);
            }
            y += height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_balances_its_rows() {
        let grid = StatGrid::new(200.0).max_columns(5);
        assert_eq!(grid.columns(1200.0, 6), 3);
        assert_eq!(grid.columns(1200.0, 5), 5);
        assert_eq!(grid.columns(1200.0, 4), 4);
        assert_eq!(grid.columns(1200.0, 7), 4);
        assert_eq!(grid.columns(340.0, 6), 1);
        assert_eq!(StatGrid::new(150.0).columns(348.0, 6), 2);
        assert_eq!(StatGrid::new(150.0).columns(348.0, 1), 1);
    }

    #[test]
    fn a_long_text_keeps_its_rest_on_its_last_line() {
        let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(clamp_lines(lines.clone(), 2), vec!["a", "b c"]);
        assert_eq!(clamp_lines(lines, 3).len(), 3);
    }
}
