use super::button::{Button, ButtonVariant};
use super::input::TextInput;
use super::popup;
use super::Label;
use crate::icon::Icon;
use crate::{Grid, Insets, Layer, Response, Stack, Strip, Ui};
use ui_kit_core::{Rect, TextAlign};

/// A date and time being picked: the month shown, the day chosen, and the
/// time as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: String,
    pub minute: String,
}

impl Default for Calendar {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
            hour: "00".into(),
            minute: "00".into(),
        }
    }
}

/// How many days `month` of `year` has.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

/// The weekday the month starts on, Sunday being 0.
fn first_weekday(year: i32, month: u32) -> u32 {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = year - if month < 3 { 1 } else { 0 };
    ((y + y / 4 - y / 100 + y / 400 + offsets[month as usize - 1] + 1).rem_euclid(7)) as u32
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl Calendar {
    /// Read `YYYY-MM-DD HH:MM`; anything else gives the default.
    pub fn from_value(value: &str) -> Self {
        let parts: Vec<_> = value.split(['-', ' ', ':']).collect();
        if parts.len() != 5 {
            return Self::default();
        }
        match (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>(),
            parts[4].parse::<u32>(),
        ) {
            (Ok(y), Ok(m), Ok(d), Ok(h), Ok(min))
                if (1..=9999).contains(&y)
                    && (1..=12).contains(&m)
                    && d > 0
                    && d <= days_in_month(y, m)
                    && h < 24
                    && min < 60 =>
            {
                Self {
                    year: y,
                    month: m,
                    day: d,
                    hour: format!("{h:02}"),
                    minute: format!("{min:02}"),
                }
            }
            _ => Self::default(),
        }
    }

    /// Move `delta` months, keeping the day inside the new month.
    pub fn change_month(&mut self, delta: i32) {
        let index = (self.year * 12 + self.month as i32 - 1 + delta).clamp(12, 9999 * 12 + 11);
        self.year = index / 12;
        self.month = (index % 12 + 1) as u32;
        self.day = self.day.min(days_in_month(self.year, self.month));
    }

    /// `YYYY-MM-DD HH:MM`, or `None` while the time is not a valid one.
    pub fn value(&self) -> Option<String> {
        let hour = self.hour.parse::<u32>().ok()?;
        let minute = self.minute.parse::<u32>().ok()?;
        if hour >= 24 || minute >= 60 {
            return None;
        }
        Some(format!(
            "{:04}-{:02}-{:02} {hour:02}:{minute:02}",
            self.year, self.month, self.day
        ))
    }

    /// The popup's size.
    pub const SIZE: (f32, f32) = (380.0, 418.0);

    /// Draw the month grid and time fields in `rect`. Returns `Some(true)`
    /// when Apply was pressed with a valid value, `Some(false)` on Cancel.
    pub fn show(&mut self, ui: &mut Ui, id: &str, rect: Rect) -> Option<bool> {
        let mut rows = Stack::new(rect.inset_by(Insets::new(20.0, 18.0, 20.0, 18.0)), 6.0);
        let mut header = Strip::row(rows.row(32.0), 6.0);
        let nav = |icon| Button::icon_only(icon).variant(ButtonVariant::Text);
        if nav(Icon::ChevronLeft)
            .show(ui, &format!("{id}-prev"), header.leading(32.0))
            .clicked
        {
            self.change_month(-1);
        }
        if nav(Icon::ChevronRight)
            .show(ui, &format!("{id}-next"), header.trailing(32.0))
            .clicked
        {
            self.change_month(1);
        }
        Label::heading(17.0).align(TextAlign::Center).show(
            ui,
            header.rest(),
            &format!("{} {}", MONTHS[self.month as usize - 1], self.year),
        );
        rows.space(6.0);
        for (label, cell) in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
            .iter()
            .zip(rows.row(24.0).columns(7, 0.0))
        {
            Label::muted()
                .size(12.0)
                .weight(600)
                .align(TextAlign::Center)
                .show(ui, cell.inset_by(Insets::new(0.0, 0.0, 4.0, 0.0)), label);
        }
        // Six weeks of days; the neighbouring months' days are shown disabled.
        let grid = Grid::columns(rows.row(34.0 * 6.0), 7, 0.0).with_cell_height(34.0);
        let offset = first_weekday(self.year, self.month) as i32;
        let length = days_in_month(self.year, self.month) as i32;
        let previous = if self.month == 1 {
            days_in_month(self.year - 1, 12)
        } else {
            days_in_month(self.year, self.month - 1)
        } as i32;
        for index in 0..42 {
            let day = index - offset + 1;
            let valid = day > 0 && day <= length;
            let shown = if day <= 0 {
                previous + day
            } else if !valid {
                day - length
            } else {
                day
            };
            let variant = if valid && day as u32 == self.day {
                ButtonVariant::Selected
            } else {
                ButtonVariant::Text
            };
            if Button::new(&shown.to_string())
                .variant(variant)
                .disabled(!valid)
                .show(
                    ui,
                    &format!("{id}-day-{index}"),
                    grid.cell(index as usize)
                        .inset_by(Insets::new(0.0, 0.0, 4.0, 4.0)),
                )
                .clicked
            {
                self.day = day as u32;
            }
        }
        let mut time = Strip::row(rows.row(32.0), 4.0);
        Label::new()
            .size(13.0)
            .weight(500)
            .show(ui, time.leading(58.0), "Time");
        TextInput::new("HH").show(
            ui,
            &format!("{id}-hour"),
            time.leading(64.0),
            &mut self.hour,
        );
        Label::new()
            .size(16.0)
            .weight(500)
            .align(TextAlign::Center)
            .show(ui, time.leading(12.0), ":");
        TextInput::new("MM").show(
            ui,
            &format!("{id}-minute"),
            time.leading(64.0),
            &mut self.minute,
        );
        let value = self.value();
        let error = rows.row(20.0);
        if value.is_none() {
            Label::new().size(12.0).color(super::theme::DANGER).show(
                ui,
                error,
                "Enter hours 0-23 and minutes 0-59.",
            );
        }
        let mut buttons = Strip::row(rows.row(34.0), 0.0);
        if Button::new("Cancel")
            .variant(ButtonVariant::Outline)
            .show(ui, &format!("{id}-cancel"), buttons.leading(100.0))
            .clicked
        {
            return Some(false);
        }
        if Button::new("Apply")
            .disabled(value.is_none())
            .show(ui, &format!("{id}-apply"), buttons.trailing(110.0))
            .clicked
        {
            return Some(true);
        }
        None
    }
}

/// What a [`DateTimePicker`] keeps between frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateTimePickerState {
    pub open: bool,
    pub calendar: Calendar,
}

/// A field showing a date and time that opens a calendar to change it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DateTimePicker {
    disabled: bool,
}

impl DateTimePicker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draw the field and, while open, its calendar. `changed` means
    /// `value` was set by applying the calendar.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        value: &mut String,
        state: &mut DateTimePickerState,
    ) -> Response {
        let mut response = Button::new(value)
            .variant(ButtonVariant::Outline)
            .align(TextAlign::Left)
            .trailing(Icon::Calendar)
            .disabled(self.disabled)
            .show(ui, id, rect);
        if response.clicked {
            state.open = !state.open;
            state.calendar = Calendar::from_value(value);
        }
        if !state.open || self.disabled {
            state.open = false;
            return response;
        }
        let (width, height) = Calendar::SIZE;
        let area = popup::below(ui, rect, 4.0, width, height);
        let outcome = popup::show(ui, area, Layer::OVERLAY, |ui| {
            state.calendar.show(ui, &format!("{id}-calendar"), area)
        });
        match outcome {
            Some(true) => {
                if let Some(new) = state.calendar.value() {
                    response.changed = *value != new;
                    *value = new;
                }
                state.open = false;
            }
            Some(false) => state.open = false,
            None => {
                if popup::dismissed(ui, &[area, rect]) {
                    state.open = false;
                }
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years_and_month_changes_keep_the_day_valid() {
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        let mut c = Calendar::from_value("2024-01-31 23:59");
        c.change_month(1);
        assert_eq!(c.value().as_deref(), Some("2024-02-29 23:59"));
        c.change_month(-2);
        assert_eq!((c.year, c.month), (2023, 12));
        c.hour = "24".into();
        assert!(c.value().is_none());
    }

    #[test]
    fn july_2026_starts_on_a_wednesday() {
        assert_eq!(first_weekday(2026, 7), 3);
    }
}
