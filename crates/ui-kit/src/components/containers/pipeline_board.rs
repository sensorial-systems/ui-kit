use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct PipelineColumn {
    pub id: String,
    pub label: String,
    pub class: String,
}

#[derive(Clone, PartialEq)]
pub struct PipelineCard {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub group: String,
    pub color: String,
    pub meta: String,
}

#[component]
pub fn PipelineBoard(
    columns: Vec<PipelineColumn>,
    cards: Vec<PipelineCard>,
    #[props(default)] selected: Vec<String>,
    #[props(default)] dragging: Option<String>,
    #[props(default)] hovered_column: Option<String>,
    #[props(default = "No items".to_string())] empty_label: String,
    #[props(default = "No items".to_string())] empty_column_label: String,
    onselect: EventHandler<(String, bool)>,
    onpointerdown: EventHandler<(String, f64, f64)>,
    onhovercolumn: EventHandler<Option<String>>,
    ondrop: EventHandler<String>,
) -> Element {
    let column_count = columns.len();
    rsx! {
        section {
            class: "kanban-board pipeline-board",
            style: "--pipeline-columns: {column_count};",
            if cards.is_empty() {
                div { class: "kanban-empty pipeline-empty", "{empty_label}" }
            } else {
                for column in columns {
                    {
                        let column_cards = cards.iter().filter(|card| card.column_id == column.id).cloned().collect::<Vec<_>>();
                        let count = column_cards.len();
                        let is_dragging = dragging.is_some();
                        let is_hovered = is_dragging && hovered_column.as_ref() == Some(&column.id);
                        let drop_class = if is_dragging { "is-drop-target" } else { "" };
                        let hover_class = if is_hovered { "is-hovered" } else { "" };
                        let column_id = column.id.clone();
                        rsx! {
                            article {
                                key: "{column.id}",
                                class: "kanban-column pipeline-column {column.class} {drop_class} {hover_class}",
                                onmouseenter: {
                                    let column_id = column_id.clone();
                                    move |_| if is_dragging { onhovercolumn.call(Some(column_id.clone())); }
                                },
                                onmouseleave: {
                                    let column_id = column_id.clone();
                                    let hovered_column = hovered_column.clone();
                                    move |_| if hovered_column.as_ref() == Some(&column_id) { onhovercolumn.call(None); }
                                },
                                onmouseup: {
                                    let column_id = column_id.clone();
                                    move |event| if is_dragging { event.stop_propagation(); ondrop.call(column_id.clone()); }
                                },
                                div { class: "kanban-column-header pipeline-column-header",
                                    div { p { class: "eyebrow", "{column.label}" } h2 { "{count}" } }
                                }
                                div { class: "kanban-card-list pipeline-card-list",
                                    if column_cards.is_empty() {
                                        div { class: "kanban-column-empty pipeline-column-empty", "{empty_column_label}" }
                                    }
                                    for card in column_cards {
                                        {
                                            let card_id = card.id.clone();
                                            let is_selected = selected.contains(&card.id);
                                            let is_card_dragging = dragging.as_ref() == Some(&card.id);
                                            let selected_class = if is_selected { "selected" } else { "" };
                                            let dragging_class = if is_card_dragging { "dragging" } else { "" };
                                            rsx! {
                                                article {
                                                    key: "{card.id}",
                                                    class: "kanban-card pipeline-card {selected_class} {dragging_class}",
                                                    style: "--project-color: {card.color}; --pipeline-color: {card.color};",
                                                    role: "button",
                                                    tabindex: "0",
                                                    onclick: {
                                                        let card_id = card_id.clone();
                                                        move |event| onselect.call((card_id.clone(), event.data().modifiers().shift()))
                                                    },
                                                    onmousedown: {
                                                        let card_id = card_id.clone();
                                                        move |event| {
                                                            event.prevent_default();
                                                            let coordinates = event.data().client_coordinates();
                                                            onpointerdown.call((card_id.clone(), coordinates.x, coordinates.y));
                                                        }
                                                    },
                                                    div { class: "kanban-card-main pipeline-card-main",
                                                        div { class: "kanban-card-project pipeline-card-group",
                                                            span { class: "project-color-dot pipeline-color-dot" }
                                                            span { "{card.group}" }
                                                        }
                                                        h3 { "{card.title}" }
                                                        p { class: "kanban-card-meta pipeline-card-meta", "{card.meta}" }
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

#[component]
pub fn PipelineDragPreview(card: PipelineCard, count: usize, x: f64, y: f64) -> Element {
    rsx! {
        article {
            class: "kanban-drag-preview pipeline-drag-preview",
            style: "left: {x + 16.0}px; top: {y + 16.0}px; --project-color: {card.color}; --pipeline-color: {card.color};",
            div { class: "kanban-card-project pipeline-card-group",
                span { class: "project-color-dot pipeline-color-dot" }
                span { "{card.group}" }
            }
            h3 { "{card.title}" }
            p { class: "kanban-card-meta pipeline-card-meta", "{card.meta}" }
            if count > 1 { span { class: "kanban-drag-count pipeline-drag-count", "+{count - 1}" } }
        }
    }
}
