//! Larger building blocks: bills, metric cards, section headings, modals and
//! a browser frame with tabs.
use super::button::{Button, ButtonVariant};
use super::card::{panel, Card};
use super::charts::Sparkline;
use super::feedback::{Badge, ProgressBar, StatusVariant};
use super::input::TextInput;
use super::Label;
use crate::icon::Icon;
use crate::vector::line;
use crate::{Flex, Insets, Key, Layer, Size, Stack, Strip, Ui};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// A section title with an accent bar before it.
pub fn section_heading(ui: &mut Ui, rect: Rect, title: &str) {
    let (bar, text) = rect.split_left(4.0, 12.0);
    panel(ui, bar, ui.theme.palette.accent, [0.0; 4], 0.0);
    Label::heading(20.0).show(ui, text, title);
}

/// A line of a [`Bill`].
#[derive(Clone, Debug, PartialEq)]
pub struct BillItem {
    pub label: String,
    /// Quantity and unit price, as it should read.
    pub quantity: String,
    pub amount: String,
}

impl BillItem {
    pub fn new(
        label: impl Into<String>,
        quantity: impl Into<String>,
        amount: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            quantity: quantity.into(),
            amount: amount.into(),
        }
    }
}

/// An invoice: a heading, a status, its lines and a total.
#[derive(Clone, Debug, PartialEq)]
pub struct Bill<'a> {
    pub kicker: &'a str,
    pub number: &'a str,
    pub status: (&'a str, StatusVariant),
    pub items: &'a [BillItem],
    pub total: &'a str,
    pub note: &'a str,
}

impl Bill<'_> {
    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let p = ui.theme.palette;
        let mut column = Strip::column(rect, 14.0);
        Label::muted()
            .size(11.0)
            .weight(700)
            .show(ui, column.leading(22.0), self.kicker);
        Label::heading(20.0).show(ui, column.leading(30.0), self.number);
        let (text, variant) = self.status;
        let badge = Badge::new(variant);
        let width = badge.width(ui, text).max(92.0);
        badge.show(ui, column.leading(24.0).split_left(width, 0.0).0, text);
        column.skip_leading(8.0);
        let note = column.trailing(44.0);
        let total = column.trailing(30.0);
        let mut lines = Stack::new(column.rest(), 8.0);
        for item in self.items {
            let row = lines.row(56.0);
            let (name, details) = row.split_top(24.0, 2.0);
            Label::new()
                .size(13.0)
                .weight(500)
                .show(ui, name, &item.label);
            let [quantity, amount] = Flex::row(0.0).split(
                details.split_top(20.0, 0.0).0,
                [Size::Fill(0.55), Size::Fill(0.45)],
            );
            Label::muted().size(11.0).show(ui, quantity, &item.quantity);
            Label::new()
                .size(13.0)
                .weight(600)
                .align(TextAlign::Right)
                .show(ui, amount, &item.amount);
            line(
                ui,
                Point::new(row.x, row.bottom()),
                Point::new(row.right(), row.bottom()),
                1.0,
                p.border,
            );
        }
        let [caption, sum] = Flex::row(0.0).split(total, [Size::Fill(0.45), Size::Fill(0.55)]);
        Label::heading(18.0).show(ui, caption, "Total");
        Label::heading(20.0)
            .align(TextAlign::Right)
            .show(ui, sum, self.total);
        Label::muted().size(12.0).show(ui, note, self.note);
    }
}

/// What a [`MetricCard`] draws under its value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MetricChart<'a> {
    #[default]
    None,
    Progress(f32),
    Sparkline(&'a [f32]),
}

/// A small card with a label, a large value and an optional chart.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MetricCard<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub color: Option<Color>,
    pub chart: MetricChart<'a>,
}

impl MetricCard<'_> {
    pub const HEIGHT: f32 = 124.0;

    pub fn show(&self, ui: &mut Ui, rect: Rect, glass: f32) {
        Card::new().glass(glass).show(ui, rect);
        let inside = rect.inset_by(Insets::symmetric(16.0, 14.0));
        let (heading, chart) = inside.split_bottom(26.0, 0.0);
        let mut lines = Stack::new(heading, 8.0);
        Label::muted()
            .size(13.0)
            .weight(500)
            .show(ui, lines.row(20.0), self.label);
        let mut value = Label::heading(28.0);
        if let Some(color) = self.color {
            value = value.color(color);
        }
        value.show(ui, lines.row(36.0), self.value);
        match self.chart {
            MetricChart::None => {}
            MetricChart::Progress(fraction) => {
                ProgressBar::new().show(ui, chart.center_vertically(8.0), fraction)
            }
            MetricChart::Sparkline(values) => Sparkline::new().show(ui, chart, values),
        }
    }
}

/// A dialog over a dimmed page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modal {
    width: f32,
    height: f32,
    glass: f32,
}

impl Modal {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            glass: 1.0,
        }
    }

    pub fn glass(mut self, glass: f32) -> Self {
        self.glass = glass;
        self
    }

    /// Draw the dialog in the middle of `viewport` and its contents with
    /// `body`, given the dialog's rect. Returns whether the close button or
    /// Escape asked to close it, and what `body` returned.
    pub fn show<R>(
        &self,
        ui: &mut Ui,
        id: &str,
        viewport: Rect,
        body: impl FnOnce(&mut Ui, Rect) -> R,
    ) -> (bool, R) {
        ui.scoped(|ui| {
            ui.clip = None;
            ui.with_layer(Layer::MODAL, |ui| {
                // The scrim takes the pointer from the whole page.
                ui.reserve(Layer::MODAL, viewport);
                let scrim = ui.theme.palette.scrim;
                panel(ui, viewport, scrim, [0.0; 4], 0.0);
                let dialog = Rect::new(
                    viewport.x + (viewport.width - self.width) * 0.5,
                    viewport.y + (viewport.height - self.height) * 0.5,
                    self.width,
                    self.height,
                );
                Card::new().glass(self.glass).show(ui, dialog);
                let close = dialog.inset_by(Insets::new(0.0, 12.0, 18.0, 0.0)).align(
                    32.0,
                    32.0,
                    crate::HorizontalAlignment::Right,
                    crate::VerticalAlignment::Top,
                );
                let closed = Button::icon_only(Icon::Close)
                    .variant(ButtonVariant::Text)
                    .show(ui, &format!("{id}-close"), close)
                    .clicked
                    || ui.consume_key(Key::Escape);
                (closed, body(ui, dialog))
            })
        })
    }
}

/// What a [`Browser`] keeps: its tabs, the one showing and the address typed.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserState {
    pub tabs: Vec<String>,
    pub active: usize,
    pub address: String,
}

impl BrowserState {
    pub fn new(tabs: Vec<String>) -> Self {
        let address = tabs.first().cloned().unwrap_or_default();
        Self {
            tabs,
            active: 0,
            address,
        }
    }

    /// What the active tab shows.
    pub fn current(&self) -> &str {
        self.tabs.get(self.active).map_or("", String::as_str)
    }
}

/// Something done in a [`Browser`]'s chrome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserEvent {
    Back,
    Forward,
    Navigate(String),
    SelectTab(usize),
    CloseTab(usize),
    NewTab,
}

/// Browser chrome: back, forward, an address bar and tabs over a content area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Browser;

impl Browser {
    /// Draw the chrome in `rect`. Tabs and the address update `state`; the
    /// events say what happened, and the returned rect is where the page goes.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        state: &mut BrowserState,
    ) -> (Vec<BrowserEvent>, Rect) {
        let p = ui.theme.palette;
        let mut events = Vec::new();
        panel(ui, rect, p.surface, p.border, 12.0);
        let mut chrome = Strip::column(rect.inset(8.0), 10.0);
        let mut toolbar = Strip::row(chrome.leading(34.0), 2.0);
        let nav = |icon| Button::icon_only(icon).variant(ButtonVariant::Text);
        if nav(Icon::ChevronLeft)
            .show(ui, &format!("{id}-back"), toolbar.leading(34.0))
            .clicked
        {
            events.push(BrowserEvent::Back);
        }
        if nav(Icon::ChevronRight)
            .show(ui, &format!("{id}-forward"), toolbar.leading(34.0))
            .clicked
        {
            events.push(BrowserEvent::Forward);
        }
        let go = toolbar.trailing(34.0);
        toolbar.skip_leading(10.0);
        toolbar.skip_trailing(6.0);
        TextInput::new("Address").show(
            ui,
            &format!("{id}-address"),
            toolbar.rest(),
            &mut state.address,
        );
        let submitted = ui.focused(&format!("{id}-address")) && ui.input().activate;
        if nav(Icon::CornerDownLeft)
            .show(ui, &format!("{id}-go"), go)
            .clicked
            || submitted
        {
            if let Some(tab) = state.tabs.get_mut(state.active) {
                *tab = state.address.clone();
            }
            events.push(BrowserEvent::Navigate(state.address.clone()));
        }

        let mut tabs = Strip::row(chrome.leading(34.0), 8.0);
        let add = tabs.trailing(32.0);
        let mut close = None;
        for i in 0..state.tabs.len() {
            if tabs.remaining() < 172.0 {
                break;
            }
            let (title, cross) = tabs.leading(172.0).split_left(144.0, 0.0);
            let label = if state.tabs[i].is_empty() {
                "New tab".to_string()
            } else {
                state.tabs[i].clone()
            };
            let variant = if state.active == i {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Text
            };
            if Button::new(&label)
                .variant(variant)
                .show(ui, &format!("{id}-tab-{i}"), title)
                .clicked
            {
                state.active = i;
                state.address = state.tabs[i].clone();
                events.push(BrowserEvent::SelectTab(i));
            }
            if nav(Icon::Close)
                .show(ui, &format!("{id}-close-{i}"), cross)
                .clicked
            {
                close = Some(i);
            }
        }
        if let Some(i) = close {
            state.tabs.remove(i);
            if state.tabs.is_empty() {
                state.tabs.push(String::new());
            }
            state.active = state.active.min(state.tabs.len() - 1);
            state.address = state.current().to_string();
            events.push(BrowserEvent::CloseTab(i));
        }
        if nav(Icon::Plus)
            .show(ui, &format!("{id}-new-tab"), add)
            .clicked
        {
            state.tabs.push(String::new());
            state.active = state.tabs.len() - 1;
            state.address.clear();
            events.push(BrowserEvent::NewTab);
        }
        (events, chrome.rest())
    }
}
