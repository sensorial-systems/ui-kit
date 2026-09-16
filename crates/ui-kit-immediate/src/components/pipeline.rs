//! A kanban board: cards in columns, moved by dragging.
use super::card::panel;
use super::feedback::Badge;
use super::theme::{is_dark, with_alpha};
use super::Label;
use crate::{CursorIcon, Flex, Insets, Layer, Size, Stack, Ui};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// A column of a [`PipelineBoard`].
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineColumn {
    pub id: String,
    pub label: String,
}

impl PipelineColumn {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// A card on a [`PipelineBoard`], in the column at index `column`.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineCard {
    pub id: String,
    pub title: String,
    pub column: usize,
    /// A colour for the card's group, shown as a stripe.
    pub color: Option<Color>,
}

impl PipelineCard {
    pub fn new(id: impl Into<String>, title: impl Into<String>, column: usize) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            column,
            color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    card: usize,
    /// Where the pointer was pressed, and where on the card.
    origin: Point,
    grab: Point,
    moved: bool,
}

/// What a [`PipelineBoard`] keeps between frames: the selection and any drag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PipelineState {
    /// Indices into the cards.
    pub selected: Vec<usize>,
    drag: Option<Drag>,
}

impl PipelineState {
    /// Whether a card is being dragged.
    pub fn dragging(&self) -> bool {
        self.drag.is_some_and(|drag| drag.moved)
    }

    /// Forget a drag, as when the window loses focus.
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }

    /// The cards a drag of `card` moves: the selection if it is part of it.
    fn moving(&self, card: usize) -> Vec<usize> {
        if self.selected.contains(&card) {
            self.selected.clone()
        } else {
            vec![card]
        }
    }
}

/// What happened on a [`PipelineBoard`] this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PipelineResponse {
    /// The card pressed, if one was.
    pub pressed: Option<usize>,
    /// Cards dropped into another column, as `(card, column)`.
    pub moved: Vec<(usize, usize)>,
}

const CARD: f32 = 66.0;
const GAP: f32 = 8.0;
const HEADING: f32 = 22.0;
/// How far the pointer has to travel before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;

/// Columns of cards that can be selected (Shift adds to the selection) and
/// dragged between columns.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineBoard<'a> {
    columns: &'a [PipelineColumn],
}

impl<'a> PipelineBoard<'a> {
    pub fn new(columns: &'a [PipelineColumn]) -> Self {
        Self { columns }
    }

    /// How tall the board needs to be for `cards`, with room to drop one more
    /// into the fullest column.
    pub fn height(&self, cards: &[PipelineCard]) -> f32 {
        let tallest = (0..self.columns.len())
            .map(|c| cards.iter().filter(|card| card.column == c).count())
            .max()
            .unwrap_or(0);
        8.0 + HEADING + 14.0 + (tallest + 1) as f32 * (CARD + GAP) + 8.0
    }

    /// Draw the board and move cards that are dropped. `extend` adds a pressed
    /// card to the selection instead of replacing it, as Shift does.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        cards: &mut [PipelineCard],
        state: &mut PipelineState,
        extend: bool,
    ) -> PipelineResponse {
        let p = ui.theme.palette;
        let pointer = ui.input().pointer;
        let down = ui.input().down;
        let mut response = PipelineResponse::default();
        state.selected.retain(|i| *i < cards.len());
        if let (Some(drag), Some(pt)) = (&mut state.drag, pointer) {
            drag.moved |= (pt.x - drag.origin.x).hypot(pt.y - drag.origin.y) > DRAG_THRESHOLD;
        }
        let dragging = state.drag.filter(|drag| drag.moved);
        let moving = dragging
            .map(|drag| state.moving(drag.card))
            .unwrap_or_default();

        let areas = Flex::row(4.0).place(rect, &vec![Size::FILL; self.columns.len()]);
        let target = dragging
            .and(pointer)
            .and_then(|pt| areas.iter().position(|area| area.contains(pt)));
        let mut previews = Vec::new();
        for (c, (column, area)) in self.columns.iter().zip(&areas).enumerate() {
            let hovered = target == Some(c);
            let count = cards.iter().filter(|card| card.column == c).count();
            if hovered {
                panel(ui, *area, with_alpha(p.accent, 0.08), p.accent, 8.0);
            } else {
                panel(
                    ui,
                    *area,
                    p.raised,
                    if dragging.is_some() {
                        with_alpha(p.accent, 0.35)
                    } else {
                        p.border
                    },
                    8.0,
                );
            }
            let (heading, list) = area.inset(8.0).split_top(HEADING, 14.0);
            let badge =
                Badge::new(Default::default()).color(if hovered { p.accent } else { p.muted });
            let count_text = count.to_string();
            let (title, count_rect) =
                heading.split_right(badge.width(ui, &count_text).max(24.0), 6.0);
            Label::new().size(13.0).weight(600).show(
                ui,
                title.inset_by(Insets::symmetric(2.0, 0.0)),
                &column.label,
            );
            badge.show(ui, count_rect.center_vertically(20.0), &count_text);

            let mut rows = Stack::new(list, GAP);
            for (index, card) in cards
                .iter()
                .enumerate()
                .filter(|(_, card)| card.column == c)
            {
                let slot = rows.row(CARD);
                let lifted = moving.contains(&index);
                let pressed = ui.interact(&format!("{id}-card-{}", card.id), slot);
                if pressed.hovered && dragging.is_none() {
                    ui.set_cursor(CursorIcon::Grab);
                }
                if pressed.active && state.drag.is_none() {
                    if extend {
                        if let Some(i) = state.selected.iter().position(|s| *s == index) {
                            state.selected.remove(i);
                        } else {
                            state.selected.push(index);
                        }
                    } else if !state.selected.contains(&index) {
                        state.selected = vec![index];
                    }
                    if let Some(pt) = pointer {
                        state.drag = Some(Drag {
                            card: index,
                            origin: pt,
                            grab: Point::new(pt.x - slot.x, pt.y - slot.y),
                            moved: false,
                        });
                    }
                    response.pressed = Some(index);
                }
                if lifted {
                    // Where the card is lifted from: an outline of it.
                    panel(
                        ui,
                        slot,
                        with_alpha(p.accent, 0.04),
                        with_alpha(p.accent, 0.4),
                        8.0,
                    );
                    Label::muted().size(13.0).show(
                        ui,
                        slot.inset(8.0).split_top(40.0, 0.0).0,
                        &card.title,
                    );
                } else {
                    draw_card(ui, slot, card, state.selected.contains(&index));
                }
                if dragging.is_some_and(|drag| drag.card == index) {
                    previews.push((slot, card.clone()));
                }
            }
            // Where the dropped cards will land.
            let incoming = moving.iter().filter(|i| cards[**i].column != c).count();
            if hovered && incoming > 0 {
                let slot = rows.row(CARD);
                panel(ui, slot, with_alpha(p.accent, 0.12), p.accent, 8.0);
                let text = if incoming == 1 {
                    "Move here".to_string()
                } else {
                    format!("Move {incoming} cards here")
                };
                Label::new()
                    .size(13.0)
                    .weight(600)
                    .color(p.accent)
                    .align(TextAlign::Center)
                    .show(ui, slot, &text);
            }
        }

        // The card follows the pointer above everything else.
        if let (Some(drag), Some(pt)) = (dragging, pointer) {
            ui.set_cursor(CursorIcon::Grabbing);
            if let Some((slot, card)) = previews.first() {
                let floating = Rect::new(
                    pt.x - drag.grab.x,
                    pt.y - drag.grab.y,
                    slot.width,
                    slot.height,
                );
                ui.scoped(|ui| {
                    ui.clip = None;
                    ui.with_layer(Layer::OVERLAY, |ui| {
                        let shadow = if is_dark(&p) {
                            [0.0, 0.0, 0.0, 0.4]
                        } else {
                            [0.06, 0.09, 0.18, 0.18]
                        };
                        ui.shadow_box(
                            Rect {
                                y: floating.y + 6.0,
                                ..floating
                            },
                            8.0,
                            16.0,
                            shadow,
                        );
                        draw_card(ui, floating, card, true);
                        if moving.len() > 1 {
                            let text = format!("+{}", moving.len() - 1);
                            let badge = Badge::new(Default::default()).color(p.accent);
                            let width = badge.width(ui, &text).max(26.0);
                            let chip = Rect::new(
                                floating.right() - width + 8.0,
                                floating.y - 10.0,
                                width,
                                22.0,
                            );
                            super::card::panel(ui, chip, p.accent, [0.0; 4], 11.0);
                            Label::new()
                                .size(12.0)
                                .weight(700)
                                .color([1.0; 4])
                                .align(TextAlign::Center)
                                .show(ui, chip, &text);
                        }
                    });
                });
            }
        }

        if !down {
            if let Some(drag) = state.drag.take() {
                if let (true, Some(column)) = (drag.moved, target) {
                    for card in state.moving(drag.card) {
                        if cards[card].column != column {
                            cards[card].column = column;
                            response.moved.push((card, column));
                        }
                    }
                }
            }
        }
        response
    }
}

fn draw_card(ui: &mut Ui, rect: Rect, card: &PipelineCard, selected: bool) {
    let p = ui.theme.palette;
    panel(
        ui,
        rect,
        p.surface,
        if selected { p.accent } else { p.border },
        8.0,
    );
    let mut text = rect.inset(8.0);
    if let Some(color) = card.color {
        let (stripe, rest) = text.split_left(3.0, 8.0);
        panel(ui, stripe, color, [0.0; 4], 1.5);
        text = rest;
    }
    Label::new()
        .size(13.0)
        .weight(500)
        .show(ui, text.split_top(40.0, 0.0).0, &card.title);
}
