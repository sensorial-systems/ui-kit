//! A roadmap: projects, their milestones and tasks as bars across columns of time.
use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::feedback::ProgressBar;
use super::Label;
use crate::icon::Icon;
use crate::vector::line;
use crate::{CursorIcon, Insets, Stack, Ui};
use ui_kit_core::{Color, Point, Rect};

/// A span of the timeline, as fractions of its full width.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimelineRange {
    pub start: f32,
    pub length: f32,
}

impl TimelineRange {
    pub fn new(start: f32, length: f32) -> Self {
        Self { start, length }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTask {
    pub id: String,
    pub title: String,
    pub range: TimelineRange,
    pub color: Color,
}

impl TimelineTask {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        range: TimelineRange,
        color: Color,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            range,
            color,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineMilestone {
    pub id: String,
    pub title: String,
    pub range: TimelineRange,
    /// How far along the milestone is, from 0 to 1.
    pub progress: f32,
    pub expanded: bool,
    pub tasks: Vec<TimelineTask>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineProject {
    pub id: String,
    pub title: String,
    pub expanded: bool,
    pub milestones: Vec<TimelineMilestone>,
    /// Tasks not part of any milestone.
    pub tasks: Vec<TimelineTask>,
}

/// Where a task being dragged is: `(project, milestone or None, task)`.
type TaskPath = (usize, Option<usize>, usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct TaskDrag {
    task: TaskPath,
    resize: bool,
    pointer: f32,
    range: TimelineRange,
}

/// What a [`Timeline`] keeps between frames: a task being moved or resized.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineState {
    drag: Option<TaskDrag>,
}

impl TimelineState {
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }
}

const HEADER: f32 = 24.0;
const HEADER_GAP: f32 = 10.0;
const ROW: f32 = 32.0;
const ROW_GAP: f32 = 12.0;
const INDENT: f32 = 12.0;
/// How close to a bar's end a press resizes it instead of moving it.
const RESIZE_EDGE: f32 = 10.0;

enum Row {
    Project(usize),
    Milestone(usize, usize),
    Task(TaskPath),
}

/// Projects, milestones and tasks as rows of bars under columns of time.
/// Collapsed rows take no space.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline<'a> {
    columns: &'a [&'a str],
    names: f32,
    zoom: f32,
}

impl<'a> Timeline<'a> {
    pub fn new(columns: &'a [&'a str]) -> Self {
        Self {
            columns,
            names: 170.0,
            zoom: 1.0,
        }
    }

    /// How wide the column of names is.
    pub fn names_width(mut self, width: f32) -> Self {
        self.names = width;
        self
    }

    /// Stretch the time columns: 1 fits them in the width.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom.max(0.1);
        self
    }

    fn rows(projects: &[TimelineProject]) -> Vec<(Row, usize)> {
        let mut rows = Vec::new();
        for (p, project) in projects.iter().enumerate() {
            rows.push((Row::Project(p), 0));
            if !project.expanded {
                continue;
            }
            for (m, milestone) in project.milestones.iter().enumerate() {
                rows.push((Row::Milestone(p, m), 1));
                if milestone.expanded {
                    for t in 0..milestone.tasks.len() {
                        rows.push((Row::Task((p, Some(m), t)), 2));
                    }
                }
            }
            for t in 0..project.tasks.len() {
                rows.push((Row::Task((p, None, t)), 1));
            }
        }
        rows
    }

    /// How tall the timeline is with the rows `projects` has expanded.
    pub fn height(&self, projects: &[TimelineProject]) -> f32 {
        let count = Self::rows(projects).len() as f32;
        HEADER + HEADER_GAP + count * ROW + (count - 1.0).max(0.0) * ROW_GAP
    }

    /// Draw the timeline; returns whether a task was moved or resized.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        projects: &mut [TimelineProject],
        state: &mut TimelineState,
    ) -> bool {
        let p = ui.theme.palette;
        let rect = Rect {
            height: self.height(projects),
            ..rect
        };
        let (days, body) = rect.split_top(HEADER, HEADER_GAP);
        let (_, tracks) = body.split_left(self.names, 0.0);
        // Columns fill the tracks with a little room at the end, times the zoom.
        let span = (tracks.width - 10.0).max(0.0) * self.zoom;
        let columns =
            Rect::new(tracks.x, days.y, span, days.height).columns(self.columns.len().max(1), 0.0);
        for (label, day) in self.columns.iter().zip(columns) {
            if day.x < rect.right() {
                Label::muted().size(12.0).weight(500).show(ui, day, label);
                line(
                    ui,
                    Point::new(day.x, day.bottom()),
                    Point::new(day.x, rect.bottom()),
                    1.0,
                    p.border,
                );
            }
        }

        let bar = |track: Rect, range: TimelineRange, height: f32| {
            Rect::new(
                track.x + span * range.start,
                track.y,
                span * range.length,
                track.height,
            )
            .center_vertically(height)
        };
        let pointer = ui.input().pointer;
        let down = ui.input().down;
        let mut changed = false;
        let mut stack = Stack::new(body, ROW_GAP);
        for (row, depth) in Self::rows(projects) {
            let (name, track) = stack.row(ROW).split_left(self.names, 0.0);
            let indent = INDENT * depth as f32;
            let name = name
                .inset_by(Insets::new(indent, 0.0, 0.0, 0.0))
                .split_left(160.0 - indent, 0.0)
                .0;
            let expander = |expanded: bool| {
                if expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                }
            };
            match row {
                Row::Project(i) => {
                    let project = &mut projects[i];
                    if Button::new(&project.title)
                        .variant(ButtonVariant::Text)
                        .icon(expander(project.expanded))
                        .align(ui_kit_core::TextAlign::Left)
                        .padding(4.0)
                        .show(ui, &format!("{id}-{}", project.id), name)
                        .clicked
                    {
                        project.expanded = !project.expanded;
                    }
                }
                Row::Milestone(i, m) => {
                    let milestone = &mut projects[i].milestones[m];
                    if Button::new(&milestone.title)
                        .variant(ButtonVariant::Text)
                        .icon(expander(milestone.expanded))
                        .align(ui_kit_core::TextAlign::Left)
                        .padding(4.0)
                        .show(ui, &format!("{id}-{}", milestone.id), name)
                        .clicked
                    {
                        milestone.expanded = !milestone.expanded;
                    }
                    ui.with_clip(tracks, |ui| {
                        ProgressBar::new().show(
                            ui,
                            bar(track, milestone.range, 24.0),
                            milestone.progress,
                        );
                    });
                }
                Row::Task(path) => {
                    let (i, m, t) = path;
                    let task = match m {
                        Some(m) => &mut projects[i].milestones[m].tasks[t],
                        None => &mut projects[i].tasks[t],
                    };
                    Label::new().size(12.0).show(
                        ui,
                        name.inset_by(Insets::new(4.0, 0.0, 0.0, 0.0)),
                        &task.title,
                    );
                    let rect = bar(track, task.range, 22.0);
                    let response = ui.interact(&format!("{id}-{}", task.id), rect);
                    let near_end = pointer.is_some_and(|pt| pt.x > rect.right() - RESIZE_EDGE);
                    if response.hovered || state.drag.is_some_and(|d| d.task == path) {
                        let resizing = state.drag.map_or(near_end, |d| d.resize);
                        ui.set_cursor(if resizing {
                            CursorIcon::ResizeHorizontal
                        } else {
                            CursorIcon::Grab
                        });
                    }
                    if response.active && state.drag.is_none() {
                        if let Some(pt) = pointer {
                            state.drag = Some(TaskDrag {
                                task: path,
                                resize: near_end,
                                pointer: pt.x,
                                range: task.range,
                            });
                        }
                    }
                    if let (Some(drag), Some(pt), true) = (state.drag, pointer, down) {
                        if drag.task == path && span > 0.0 {
                            let delta = (pt.x - drag.pointer) / span;
                            let before = task.range;
                            if drag.resize {
                                task.range.length =
                                    (drag.range.length + delta).clamp(0.05, 1.0 - task.range.start);
                            } else {
                                task.range.start =
                                    (drag.range.start + delta).clamp(0.0, 1.0 - task.range.length);
                            }
                            changed |= task.range != before;
                        }
                    }
                    let color = task.color;
                    let rect = bar(track, task.range, 22.0);
                    ui.with_clip(tracks, |ui| panel(ui, rect, color, [0.0; 4], 5.0));
                }
            }
        }
        if !down {
            state.drag = None;
        }
        changed
    }
}
