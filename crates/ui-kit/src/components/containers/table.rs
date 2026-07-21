use dioxus::prelude::*;
use crate::components::info::{Spinner, SpinnerSize, SpinnerVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TableAlign {
    pub fn class_name(&self) -> &'static str {
        match self {
            TableAlign::Left => "uikit-table-align-left",
            TableAlign::Center => "uikit-table-align-center",
            TableAlign::Right => "uikit-table-align-right",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct TableColumn {
    pub id: String,
    pub title: String,
    pub align: TableAlign,
    pub width: Option<String>,
    pub sortable: bool,
}

#[derive(Clone, PartialEq)]
pub struct TableRow {
    pub id: String,
    pub cells: Vec<Element>,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
    None,
}

#[component]
pub fn Table(
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    #[props(default)] striped: bool,
    #[props(default)] hoverable: bool,
    #[props(default)] compact: bool,
    #[props(default)] borderless: bool,
    #[props(default)] loading: bool,
    /// Column sorting interaction
    onsort: Option<EventHandler<(String, SortDirection)>>,
    /// Active sorted column ID
    active_sort_col: Option<String>,
    /// Active sorted direction
    active_sort_dir: Option<SortDirection>,
    /// Custom empty state view
    empty_state: Option<Element>,
) -> Element {
    let striped_class = if striped { "uikit-table-striped" } else { "" };
    let hoverable_class = if hoverable { "uikit-table-hoverable" } else { "" };
    let compact_class = if compact { "uikit-table-compact" } else { "" };
    let borderless_class = if borderless { "uikit-table-borderless" } else { "" };
    let blur_class = if loading { "uikit-table-loading-blur" } else { "" };

    rsx! {
        div {
            class: "uikit-table-container {borderless_class}",
            if loading {
                div {
                    class: "uikit-table-loading-overlay",
                    Spinner {
                        size: SpinnerSize::Medium,
                        variant: SpinnerVariant::Primary,
                    }
                }
            }
            table {
                class: "uikit-table {striped_class} {hoverable_class} {compact_class} {blur_class}",
                thead {
                    tr {
                        for col in columns.iter() {
                            {
                                let col_id = col.id.clone();
                                let is_active = active_sort_col.as_ref() == Some(&col_id);
                                let sort_dir = if is_active {
                                    active_sort_dir.unwrap_or(SortDirection::None)
                                } else {
                                    SortDirection::None
                                };
                                let align_class = col.align.class_name();
                                let style = col.width.as_ref().map(|w| format!("width: {};", w)).unwrap_or_default();
                                
                                if col.sortable && onsort.is_some() {
                                    let next_dir = match sort_dir {
                                        SortDirection::None => SortDirection::Ascending,
                                        SortDirection::Ascending => SortDirection::Descending,
                                        SortDirection::Descending => SortDirection::None,
                                    };
                                    let on_click = move |_| {
                                        if let Some(ref handler) = onsort {
                                            handler.call((col_id.clone(), next_dir));
                                        }
                                    };
                                    rsx! {
                                        th {
                                            class: "uikit-table-header-sortable {align_class}",
                                            style: "{style}",
                                            onclick: on_click,
                                            span {
                                                class: "uikit-table-header-content",
                                                span { "{col.title}" }
                                                span {
                                                    class: "uikit-table-sort-icon",
                                                    {match sort_dir {
                                                        SortDirection::Ascending => " ▲",
                                                        SortDirection::Descending => " ▼",
                                                        SortDirection::None => " ⇅",
                                                    }}
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        th {
                                            class: "{align_class}",
                                            style: "{style}",
                                            "{col.title}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                tbody {
                    if rows.is_empty() {
                        tr {
                            td {
                                colspan: columns.len() as u64,
                                class: "uikit-table-align-center",
                                if let Some(ref custom_empty) = empty_state {
                                    {custom_empty.clone()}
                                } else {
                                    div {
                                        class: "uikit-table-empty",
                                        "No data available"
                                    }
                                }
                            }
                        }
                    } else {
                        for row in rows.iter() {
                            {
                                let selected_class = if row.selected { "uikit-table-row-selected" } else { "" };
                                rsx! {
                                    tr {
                                        key: "{row.id}",
                                        class: "{selected_class}",
                                        for (i, cell) in row.cells.iter().enumerate() {
                                            {
                                                let align_class = columns.get(i).map(|c| c.align.class_name()).unwrap_or_default();
                                                rsx! {
                                                    td {
                                                        class: "{align_class}",
                                                        {cell.clone()}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
