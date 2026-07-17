use dioxus::prelude::*;

#[component]
pub fn DateTimePicker(
    value: String,
    on_change: EventHandler<String>,
    #[props(into, default)] label: Option<String>,
    #[props(into, default)] placeholder: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(into, default)] class: String,
) -> Element {
    let mut is_open = use_signal(|| false);

    // Initial state setup parsed from current value or defaulting to 2026-07-16 12:00
    let (init_y, init_m, init_d, init_h, init_min) = parse_datetime(&value).unwrap_or((2026, 7, 16, 12, 0));
    let mut view_year = use_signal(|| init_y);
    let mut view_month = use_signal(|| init_m);
    let mut selected_day = use_signal(|| Some(init_d));
    let mut selected_hour = use_signal(|| init_h);
    let mut selected_minute = use_signal(|| init_min);

    // Month name helper
    let month_name = match *view_month.read() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };

    let start_wday = day_of_week(*view_year.read(), *view_month.read(), 1);
    let total_days = days_in_month(*view_year.read(), *view_month.read());

    let prev_month_total = if *view_month.read() == 1 {
        days_in_month(*view_year.read() - 1, 12)
    } else {
        days_in_month(*view_year.read(), *view_month.read() - 1)
    };

    let handle_prev_month = move |_| {
        let cur_month = *view_month.read();
        if cur_month == 1 {
            let cur_year = *view_year.read();
            view_year.set(cur_year - 1);
            view_month.set(12);
        } else {
            view_month.set(cur_month - 1);
        }
    };

    let handle_next_month = move |_| {
        let cur_month = *view_month.read();
        if cur_month == 12 {
            let cur_year = *view_year.read();
            view_year.set(cur_year + 1);
            view_month.set(1);
        } else {
            view_month.set(cur_month + 1);
        }
    };

    let apply_change = move |_| {
        if let Some(day) = *selected_day.read() {
            let formatted = format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                *view_year.read(),
                *view_month.read(),
                day,
                *selected_hour.read(),
                *selected_minute.read()
            );
            on_change.call(formatted);
        }
        is_open.set(false);
    };

    let default_placeholder = placeholder.unwrap_or_else(|| "Select date & time".to_string());

    rsx! {
        div {
            class: "uikit-datetime-container {class}",
            class: if disabled { "disabled" },
            if let Some(lbl) = label {
                span { class: "uikit-input-label", "{lbl}" }
            }
            div {
                class: "uikit-datetime-input-wrapper",
                onclick: move |_| {
                    if !disabled {
                        let cur_open = *is_open.read();
                        is_open.set(!cur_open);
                    }
                },
                input {
                    r#type: "text",
                    class: "uikit-datetime-input",
                    placeholder: "{default_placeholder}",
                    value: "{value}",
                    readonly: true,
                    disabled: disabled,
                }
                span { class: "uikit-datetime-icon",
                    svg {
                        view_box: "0 0 24 24",
                        width: "16",
                        height: "16",
                        stroke: "currentColor",
                        stroke_width: "2",
                        fill: "none",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        rect { x: "3", y: "4", width: "18", height: "18", rx: "2", ry: "2" }
                        line { x1: "16", y1: "2", x2: "16", y2: "6" }
                        line { x1: "8", y1: "2", x2: "8", y2: "6" }
                        line { x1: "3", y1: "10", x2: "21", y2: "10" }
                    }
                }
            }

            if *is_open.read() && !disabled {
                div {
                    class: "uikit-datetime-popover",
                    onclick: move |e| e.stop_propagation(),

                    // Month & Year header selector
                    div { class: "uikit-datetime-popover-header",
                        button {
                            r#type: "button",
                            class: "uikit-datetime-nav-btn",
                            onclick: handle_prev_month,
                            "‹"
                        }
                        span { class: "uikit-datetime-month-year", "{month_name} {view_year}" }
                        button {
                            r#type: "button",
                            class: "uikit-datetime-nav-btn",
                            onclick: handle_next_month,
                            "›"
                        }
                    }

                    // Weekdays headers
                    div { class: "uikit-datetime-weekdays",
                        span { "Su" }
                        span { "Mo" }
                        span { "Tu" }
                        span { "We" }
                        span { "Th" }
                        span { "Fr" }
                        span { "Sa" }
                    }

                    // Days Grid
                    div { class: "uikit-datetime-days-grid",
                        // Previous month trailing days
                        for d in (prev_month_total - start_wday + 1)..=prev_month_total {
                            span { class: "uikit-datetime-day-muted", "{d}" }
                        }

                        // Current month days
                        for d in 1..=total_days {
                            {
                                let is_selected = selected_day.read().map_or(false, |sel| sel == d);
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "uikit-datetime-day-btn",
                                        class: if is_selected { "selected" },
                                        onclick: move |_| {
                                            selected_day.set(Some(d));
                                        },
                                        "{d}"
                                    }
                                }
                            }
                        }
                    }

                    // Time Selection row
                    div { class: "uikit-datetime-time-section",
                        span { class: "uikit-datetime-time-label", "Time" }
                        div { class: "uikit-datetime-time-inputs",
                            input {
                                r#type: "number",
                                class: "uikit-datetime-time-field",
                                min: 0,
                                max: 23,
                                value: "{selected_hour}",
                                oninput: move |evt| {
                                    if let Ok(h) = evt.value().parse::<i32>() {
                                        selected_hour.set(h.clamp(0, 23));
                                    }
                                }
                            }
                            span { class: "uikit-datetime-time-separator", ":" }
                            input {
                                r#type: "number",
                                class: "uikit-datetime-time-field",
                                min: 0,
                                max: 59,
                                value: "{selected_minute}",
                                oninput: move |evt| {
                                    if let Ok(m) = evt.value().parse::<i32>() {
                                        selected_minute.set(m.clamp(0, 59));
                                    }
                                }
                            }
                        }
                    }

                    // Confirm Action row
                    div { class: "uikit-datetime-popover-footer",
                        button {
                            r#type: "button",
                            class: "uikit-btn uikit-btn-sm uikit-btn-secondary",
                            onclick: move |_| is_open.set(false),
                            "Cancel"
                        }
                        button {
                            r#type: "button",
                            class: "uikit-btn uikit-btn-sm uikit-btn-primary",
                            onclick: apply_change,
                            "Confirm"
                        }
                    }
                }
            }
        }
    }
}

// Simple algorithm to parse datetime format: YYYY-MM-DD HH:MM
fn parse_datetime(s: &str) -> Option<(i32, i32, i32, i32, i32)> {
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 2 {
        return None;
    }
    let y = date_parts[0].parse().ok()?;
    let m = date_parts[1].parse().ok()?;
    let d = date_parts[2].parse().ok()?;
    let h = time_parts[0].parse().ok()?;
    let min = time_parts[1].parse().ok()?;
    Some((y, m, d, h, min))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn day_of_week(y: i32, m: i32, d: i32) -> i32 {
    // Sakamoto's algorithm: returns 0 for Sunday, 1 for Monday, ..., 6 for Saturday
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = y;
    if m < 3 {
        y -= 1;
    }
    (y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d) % 7
}
