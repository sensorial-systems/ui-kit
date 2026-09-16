use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::feedback::{Badge, Spinner, StatusVariant};
use super::theme::with_alpha;
use super::Label;
use crate::icon::Icon;
use crate::vector::line;
use crate::{Flex, Insets, Size, Stack, Strip, Ui};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// A column of a [`Table`].
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumn {
    pub title: String,
    /// The column's share of the table's width.
    pub weight: f32,
    pub align: TextAlign,
    pub sortable: bool,
}

impl TableColumn {
    pub fn new(title: impl Into<String>, weight: f32) -> Self {
        Self {
            title: title.into(),
            weight,
            align: TextAlign::Left,
            sortable: true,
        }
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
}

/// What a table cell shows.
#[derive(Clone, Debug, PartialEq)]
pub enum TableCell {
    Text(String),
    /// Text in a heavier weight, as an id or key column reads.
    Strong(String),
    /// Secondary text in the muted colour.
    Muted(String),
    /// A badge, filling the cell's width.
    Badge {
        text: String,
        color: Option<Color>,
        variant: StatusVariant,
        large: bool,
    },
}

impl TableCell {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn badge(text: impl Into<String>, variant: StatusVariant) -> Self {
        Self::Badge {
            text: text.into(),
            color: None,
            variant,
            large: false,
        }
    }

    /// The text the cell sorts by.
    pub fn sort_key(&self) -> &str {
        match self {
            TableCell::Text(text) | TableCell::Strong(text) | TableCell::Muted(text) => text,
            TableCell::Badge { text, .. } => text,
        }
    }
}

/// Which way a sorted column runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

/// The column a table is sorted by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TableSort {
    pub column: usize,
    pub direction: SortDirection,
}

/// How a [`Table`] looks and behaves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TableOptions {
    /// Tint every other row.
    pub striped: bool,
    /// Tint the row under the pointer.
    pub hoverable: bool,
    /// Shorter rows.
    pub compact: bool,
    /// No lines between rows.
    pub borderless: bool,
    /// Cover the rows with a spinner and ignore the pointer until loaded.
    pub loading: bool,
}

/// What a [`Table`] did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TableResponse {
    /// The sort was changed by clicking a header.
    pub sorted: bool,
    /// How tall the table drew, header included.
    pub height: f32,
}

/// Rows of cells under a header that sorts them.
#[derive(Clone, Debug, PartialEq)]
pub struct Table<'a> {
    columns: &'a [TableColumn],
    rows: &'a [Vec<TableCell>],
    options: TableOptions,
    empty: &'a str,
}

const HEADER: f32 = 42.0;

impl<'a> Table<'a> {
    pub fn new(columns: &'a [TableColumn], rows: &'a [Vec<TableCell>]) -> Self {
        Self {
            columns,
            rows,
            options: TableOptions::default(),
            empty: "No data available",
        }
    }

    pub fn options(mut self, options: TableOptions) -> Self {
        self.options = options;
        self
    }

    /// What shows when there are no rows.
    pub fn empty_label(mut self, label: &'a str) -> Self {
        self.empty = label;
        self
    }

    pub fn row_height(&self) -> f32 {
        if self.options.compact {
            32.0
        } else {
            44.0
        }
    }

    /// How tall the table is, header included.
    pub fn height(&self) -> f32 {
        HEADER + self.rows.len().max(1) as f32 * self.row_height()
    }

    /// The order the rows show in under `sort`.
    pub fn order(&self, sort: Option<TableSort>) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        if let Some(sort) = sort {
            let key = |i: &usize| {
                self.rows[*i]
                    .get(sort.column)
                    .map(TableCell::sort_key)
                    .unwrap_or("")
            };
            order.sort_by(|a, b| key(a).cmp(key(b)));
            if sort.direction == SortDirection::Descending {
                order.reverse();
            }
        }
        order
    }

    /// Draw the table in `rect`'s width; `time` turns the loading spinner.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        sort: &mut Option<TableSort>,
        time: f32,
    ) -> TableResponse {
        let p = ui.theme.palette;
        let loading = self.options.loading;
        let sizes: Vec<Size> = self.columns.iter().map(|c| Size::Fill(c.weight)).collect();
        let cells = |row: Rect| Flex::row(0.0).place(row, &sizes);
        let mut response = TableResponse {
            sorted: false,
            height: self.height(),
        };
        let (header, body) = rect.split_top(HEADER, 0.0);
        let body = Rect {
            height: response.height - HEADER,
            ..body
        };

        panel(ui, header, p.raised, [0.0; 4], 8.0);
        for (i, (column, cell)) in self.columns.iter().zip(cells(header)).enumerate() {
            let active = sort.filter(|s| s.column == i);
            let mut button = Button::new(&column.title)
                .variant(ButtonVariant::Text)
                .align(column.align)
                .disabled(loading || !column.sortable);
            if column.sortable {
                button = button.trailing(match active.map(|s| s.direction) {
                    Some(SortDirection::Ascending) => Icon::ArrowUp,
                    Some(SortDirection::Descending) => Icon::ArrowDown,
                    None => Icon::ArrowUpDown,
                });
            }
            if button.show(ui, &format!("{id}-sort-{i}"), cell).clicked {
                *sort = Some(TableSort {
                    column: i,
                    direction: match active.map(|s| s.direction) {
                        Some(SortDirection::Ascending) => SortDirection::Descending,
                        _ => SortDirection::Ascending,
                    },
                });
                response.sorted = true;
            }
        }

        if self.rows.is_empty() {
            Label::muted()
                .align(TextAlign::Center)
                .show(ui, body, self.empty);
            return response;
        }
        let mut lines = Stack::new(body, 0.0);
        for (position, index) in self.order(*sort).into_iter().enumerate() {
            let row = lines.row(self.row_height());
            let hovered = self.options.hoverable
                && !loading
                && ui.pointer_over(row)
                && ui.input().pointer.is_some_and(|pt| row.contains(pt));
            if (self.options.striped && position % 2 == 1) || hovered {
                panel(ui, row, p.raised, [0.0; 4], 0.0);
            }
            for ((column, cell), area) in self.columns.iter().zip(&self.rows[index]).zip(cells(row))
            {
                match cell {
                    TableCell::Badge {
                        text,
                        color,
                        variant,
                        large,
                    } => {
                        let mut badge = Badge::new(*variant);
                        if let Some(color) = color {
                            badge = badge.color(*color);
                        }
                        if *large {
                            badge = badge.large();
                        }
                        badge.show(
                            ui,
                            area.inset_by(Insets::symmetric(8.0, 0.0))
                                .center_vertically(24.0),
                            text,
                        );
                    }
                    TableCell::Text(text) | TableCell::Strong(text) | TableCell::Muted(text) => {
                        let label = match cell {
                            TableCell::Strong(_) => Label::new().weight(600),
                            TableCell::Muted(_) => Label::muted(),
                            _ => Label::new(),
                        };
                        label.align(column.align).show(
                            ui,
                            area.inset_by(Insets::new(12.0, 0.0, 8.0, 0.0)),
                            text,
                        );
                    }
                }
            }
            if !self.options.borderless {
                line(
                    ui,
                    Point::new(row.x, row.bottom()),
                    Point::new(row.right(), row.bottom()),
                    1.0,
                    p.border,
                );
            }
        }

        if loading {
            panel(ui, body, with_alpha(p.background, 0.8), [0.0; 4], 0.0);
            let label = Label::muted().size(13.0).weight(500);
            let text = "Loading";
            let width = 24.0 + 8.0 + label.width(ui, text);
            let mut status = Strip::row(body.center_horizontally(width.min(body.width)), 8.0);
            Spinner::new(20.0).show(ui, status.leading(24.0), time);
            label.show(ui, status.rest(), text);
        }
        response
    }
}
