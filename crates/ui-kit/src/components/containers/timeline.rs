use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct TimelineColumn {
    pub label: String,
    pub accent: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct TimelineRange {
    pub left: f64,
    pub width: f64,
}

#[derive(Clone, PartialEq)]
pub struct TimelineTask {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub span_label: String,
    pub status_class: String,
    pub range: TimelineRange,
    pub relative_range: Option<TimelineRange>,
    pub selected: bool,
}

#[derive(Clone, PartialEq)]
pub struct TimelineMilestone {
    pub id: String,
    pub title: String,
    pub description: String,
    pub progress: usize,
    pub span_label: String,
    pub range: TimelineRange,
    pub expanded: bool,
    pub tasks: Vec<TimelineTask>,
}

#[derive(Clone, PartialEq)]
pub struct TimelineProject {
    pub id: String,
    pub title: String,
    pub color: String,
    pub selected: bool,
    pub expanded: bool,
    pub milestones: Vec<TimelineMilestone>,
    pub unassociated_tasks: Vec<TimelineTask>,
}

#[component]
pub fn Timeline(
    columns: Vec<TimelineColumn>,
    lane_width: f64,
    unit_width: f64,
    #[props(default)] projects: Vec<TimelineProject>,
    #[props(default)] class: String,
    #[props(default)] empty_label: Option<String>,
    #[props(default)] onmounted: Option<EventHandler<Event<MountedData>>>,
    #[props(default)] onwheel: Option<EventHandler<WheelEvent>>,
    #[props(default)] onmousedown: Option<EventHandler<MouseEvent>>,
    #[props(default)] onselectproject: Option<EventHandler<String>>,
    #[props(default)] ontoggleproject: Option<EventHandler<String>>,
    #[props(default)] onselectmilestone: Option<EventHandler<String>>,
    #[props(default)] ontogglemilestone: Option<EventHandler<String>>,
    #[props(default)] onselecttask: Option<EventHandler<String>>,
    #[props(default)] onstartmilestonemove: Option<EventHandler<(String, f64)>>,
    #[props(default)] onstarttaskmove: Option<EventHandler<(String, f64)>>,
    #[props(default)] onstarttaskresize: Option<EventHandler<(String, f64)>>,
    #[props(default)] children: Element,
) -> Element {
    let min_width = lane_width + columns.len() as f64 * unit_width;
    let column_count = columns.len();
    rsx! {
        section {
            class: "roadmap timeline {class}",
            style: "--lane-label-width: {lane_width}px; --timeline-unit-width: {unit_width}px; --timeline-columns: {column_count}; --timeline-min-width: {min_width}px;",
            onmounted: move |event| if let Some(handler) = onmounted { handler.call(event) },
            onwheel: move |event| if let Some(handler) = onwheel { handler.call(event) },
            onmousedown: move |event| if let Some(handler) = onmousedown { handler.call(event) },
            div { class: "timeline-header",
                span { class: "timeline-corner" }
                for column in columns {
                    span {
                        if let Some(accent) = column.accent { span { class: "timeline-header-date", "{accent}" } }
                        span { "{column.label}" }
                    }
                }
            }
            div { class: "timeline-body",
                if let Some(label) = empty_label {
                    div { class: "timeline-empty", "{label}" }
                } else if !projects.is_empty() {
                    for project in projects {
                        TimelineProjectRows {
                            project,
                            onselectproject,
                            ontoggleproject,
                            onselectmilestone,
                            ontogglemilestone,
                            onselecttask,
                            onstartmilestonemove,
                            onstarttaskmove,
                            onstarttaskresize,
                        }
                    }
                } else {
                    {children}
                }
            }
        }
    }
}

#[component]
fn TimelineProjectRows(
    project: TimelineProject,
    onselectproject: Option<EventHandler<String>>,
    ontoggleproject: Option<EventHandler<String>>,
    onselectmilestone: Option<EventHandler<String>>,
    ontogglemilestone: Option<EventHandler<String>>,
    onselecttask: Option<EventHandler<String>>,
    onstartmilestonemove: Option<EventHandler<(String, f64)>>,
    onstarttaskmove: Option<EventHandler<(String, f64)>>,
    onstarttaskresize: Option<EventHandler<(String, f64)>>,
) -> Element {
    let selected_class = if project.selected { " selected" } else { "" };
    let expanded_class = if project.expanded { " expanded" } else { "" };
    let project_id = project.id.clone();
    rsx! {
        div {
            class: "roadmap-row project-timeline-row{selected_class}{expanded_class}",
            style: "--project-color: {project.color};",
            div { class: "lane-label",
                button {
                    class: "expand-project-button{expanded_class}",
                    r#type: "button",
                    onclick: {
                        let id = project_id.clone();
                        move |event| { event.stop_propagation(); if let Some(handler) = ontoggleproject { handler.call(id.clone()); } }
                    },
                    aria_label: if project.expanded { "Collapse project tasks" } else { "Expand project tasks" },
                    aria_expanded: "{project.expanded}",
                    svg { view_box: "0 0 24 24", path { d: "M9 6l6 6-6 6", fill: "none", stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2.5" } }
                }
                button {
                    class: "lane-label-main",
                    r#type: "button",
                    onclick: {
                        let id = project_id.clone();
                        move |_| if let Some(handler) = onselectproject { handler.call(id.clone()); }
                    },
                    strong { "{project.title}" }
                }
            }
            div { class: "lane-track",
                if !project.expanded {
                    for milestone in project.milestones.iter() {
                        div {
                            class: "collapsed-milestone-group",
                            style: "--project-color: {project.color}; margin-left: {milestone.range.left:.4}%; width: {milestone.range.width:.4}%;",
                            div { class: "collapsed-milestone-header",
                                button {
                                    class: "milestone-bar collapsed-milestone-bar",
                                    r#type: "button",
                                    title: "{milestone.description}",
                                    onclick: {
                                        let id = milestone.id.clone();
                                        move |event| { event.stop_propagation(); if let Some(handler) = onselectmilestone { handler.call(id.clone()); } }
                                    },
                                    onmousedown: {
                                        let id = milestone.id.clone();
                                        move |event| { event.stop_propagation(); event.prevent_default(); if let Some(handler) = onstartmilestonemove { handler.call((id.clone(), event.data().client_coordinates().x)); } }
                                    },
                                    span { "{milestone.title}" }
                                    small { "{milestone.span_label}" }
                                }
                            }
                            div { class: "task-miniatures timeline-task-miniatures collapsed-milestone-body",
                                for task in milestone.tasks.iter() { TimelineMiniTask { task: task.clone(), onselecttask, onstarttaskmove, onstarttaskresize } }
                            }
                        }
                    }
                    if !project.unassociated_tasks.is_empty() {
                        div { class: "task-miniatures",
                            for task in project.unassociated_tasks.iter() { TimelineMiniTask { task: task.clone(), onselecttask, onstarttaskmove, onstarttaskresize } }
                        }
                    }
                }
            }
        }
        if project.expanded {
            for milestone in project.milestones.iter() {
                div { class: "timeline-group",
                    TimelineMilestoneRow { milestone: milestone.clone(), color: project.color.clone(), onselectmilestone, ontogglemilestone, onstartmilestonemove }
                    if milestone.expanded {
                        for task in milestone.tasks.iter() { TimelineTaskRow { task: task.clone(), color: project.color.clone(), onselecttask, onstarttaskmove, onstarttaskresize } }
                    }
                }
            }
            if !project.unassociated_tasks.is_empty() {
                div { class: "timeline-unassociated-group",
                    div { class: "roadmap-row timeline-unassociated-label-row", style: "--project-color: {project.color};",
                        div { class: "lane-label timeline-unassociated-label", strong { "Unassociated" } }
                        div { class: "lane-track" }
                    }
                    for task in project.unassociated_tasks.iter() { TimelineTaskRow { task: task.clone(), color: project.color.clone(), onselecttask, onstarttaskmove, onstarttaskresize } }
                }
            }
        }
    }
}

#[component]
fn TimelineMilestoneRow(
    milestone: TimelineMilestone,
    color: String,
    onselectmilestone: Option<EventHandler<String>>,
    ontogglemilestone: Option<EventHandler<String>>,
    onstartmilestonemove: Option<EventHandler<(String, f64)>>,
) -> Element {
    let expanded_class = if milestone.expanded { " expanded" } else { "" };
    rsx! {
        div { class: "roadmap-row timeline-group-row", style: "--project-color: {color};",
            div { class: "lane-label timeline-group-label",
                button {
                    class: "expand-project-button{expanded_class}", r#type: "button",
                    onclick: { let id = milestone.id.clone(); move |event| { event.stop_propagation(); if let Some(handler) = ontogglemilestone { handler.call(id.clone()); } } },
                    aria_expanded: "{milestone.expanded}",
                    svg { view_box: "0 0 24 24", path { d: "M9 6l6 6-6 6", fill: "none", stroke: "currentColor", stroke_width: "2.5" } }
                }
                button {
                    class: "timeline-group-title task-select-button", r#type: "button",
                    onclick: { let id = milestone.id.clone(); move |_| if let Some(handler) = onselectmilestone { handler.call(id.clone()); } },
                    strong { "{milestone.title}" } span { "{milestone.progress}%" }
                }
            }
            button {
                class: "lane-track task-lane-track task-select-button", r#type: "button",
                onclick: { let id = milestone.id.clone(); move |_| if let Some(handler) = onselectmilestone { handler.call(id.clone()); } },
                div {
                    class: "milestone-bar timeline-group-bar",
                    style: "--project-color: {color}; left: {milestone.range.left:.4}%; width: {milestone.range.width:.4}%;",
                    onmousedown: { let id = milestone.id.clone(); move |event| { event.stop_propagation(); event.prevent_default(); if let Some(handler) = onstartmilestonemove { handler.call((id.clone(), event.data().client_coordinates().x)); } } },
                    span { "{milestone.title}" } small { "{milestone.span_label}" }
                }
            }
        }
    }
}

#[component]
fn TimelineTaskRow(task: TimelineTask, color: String, onselecttask: Option<EventHandler<String>>, onstarttaskmove: Option<EventHandler<(String, f64)>>, onstarttaskresize: Option<EventHandler<(String, f64)>>) -> Element {
    let selected_class = if task.selected { " selected" } else { "" };
    rsx! {
        div { class: "roadmap-row task-timeline-row{selected_class}", style: "--project-color: {color};",
            button { class: "lane-label task-lane-label task-select-button", r#type: "button", onclick: { let id = task.id.clone(); move |_| if let Some(handler) = onselecttask { handler.call(id.clone()); } }, strong { "{task.title}" } span { "{task.meta}" } }
            button { class: "lane-track task-lane-track task-select-button", r#type: "button", onclick: { let id = task.id.clone(); move |_| if let Some(handler) = onselecttask { handler.call(id.clone()); } },
                div {
                    class: "task-full-bar {task.status_class}",
                    style: "left: {task.range.left:.4}%; width: {task.range.width:.4}%;",
                    onmousedown: { let id = task.id.clone(); move |event| { event.stop_propagation(); event.prevent_default(); if let Some(handler) = onstarttaskmove { handler.call((id.clone(), event.data().client_coordinates().x)); } } },
                    span { "{task.title}" } small { "{task.span_label}" }
                    span {
                        class: "resize-handle",
                        title: "Resize task",
                        onmousedown: {
                            let id = task.id.clone();
                            move |event| {
                                event.stop_propagation();
                                event.prevent_default();
                                if let Some(handler) = onstarttaskresize {
                                    handler.call((id.clone(), event.data().client_coordinates().x));
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn TimelineMiniTask(task: TimelineTask, onselecttask: Option<EventHandler<String>>, onstarttaskmove: Option<EventHandler<(String, f64)>>, onstarttaskresize: Option<EventHandler<(String, f64)>>) -> Element {
    let range = task.relative_range.unwrap_or(task.range);
    let selected_class = if task.selected { " selected" } else { "" };
    rsx! {
        div { class: "task-miniature-row",
            button {
                class: "task-bar task-bar-button {task.status_class}{selected_class}", r#type: "button", title: "{task.title}",
                style: "left: {range.left:.4}%; width: {range.width:.4}%;",
                onclick: { let id = task.id.clone(); move |event| { event.stop_propagation(); if let Some(handler) = onselecttask { handler.call(id.clone()); } } },
                onmousedown: { let id = task.id.clone(); move |event| { event.stop_propagation(); event.prevent_default(); if let Some(handler) = onstarttaskmove { handler.call((id.clone(), event.data().client_coordinates().x)); } } },
                span {
                    class: "task-mini-resize-handle",
                    title: "Resize task",
                    onmousedown: {
                        let id = task.id.clone();
                        move |event| {
                            event.stop_propagation();
                            event.prevent_default();
                            if let Some(handler) = onstarttaskresize {
                                handler.call((id.clone(), event.data().client_coordinates().x));
                            }
                        }
                    },
                }
            }
        }
    }
}
