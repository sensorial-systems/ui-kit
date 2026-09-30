//! A list of jobs -- work something is running or has queued -- grouped, with
//! pause, resume and stop on each job and on each group.
//!
//! The list knows nothing of what a job is. The application maps its own
//! work onto [`Job`] and [`JobGroup`], draws the list, and acts on the
//! [`JobAction`] it returns.
use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::feedback::{Badge, ProgressBar, StatusVariant};
use super::theme::{with_alpha, DANGER, INFO, SUCCESS, WARNING};
use super::Label;
use crate::icon::Icon;
use crate::{Flex, Insets, Size, Stack, Strip, Ui};
use ui_kit_core::{Color, Rect, TextAlign};

/// Where a job is, as the list draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JobStatus {
    /// Doing its work.
    Running,
    /// Running, but on groundwork before its real work can go at full speed:
    /// compiling, tuning, warming a cache.
    Preparing,
    /// Running, and asked to pause; it has not stopped yet.
    Pausing,
    /// Waiting its turn.
    Queued,
    /// Held: it keeps its place but does not start.
    Paused,
    /// Announced, not yet waiting for anything.
    Pending,
    Completed,
    Failed,
    Stopped,
}

impl JobStatus {
    /// Whether the job still has work ahead of it, and so takes controls.
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Completed | Self::Failed | Self::Stopped)
    }

    /// Whether the job is held or being held, so its control resumes.
    pub fn is_paused(self) -> bool {
        matches!(self, Self::Pausing | Self::Paused)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Preparing => "Preparing",
            Self::Pausing => "Pausing",
            Self::Queued => "Queued",
            Self::Paused => "Paused",
            Self::Pending => "Pending",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Stopped => "Stopped",
        }
    }

    pub fn color(self, ui: &Ui) -> Color {
        match self {
            Self::Running | Self::Completed => SUCCESS,
            Self::Pausing | Self::Preparing => INFO,
            Self::Queued => WARNING,
            Self::Failed => DANGER,
            Self::Paused | Self::Pending | Self::Stopped => ui.theme.palette.muted,
        }
    }

    fn variant(self) -> StatusVariant {
        match self {
            Self::Running | Self::Completed => StatusVariant::Success,
            Self::Pausing | Self::Preparing => StatusVariant::Info,
            Self::Queued => StatusVariant::Warning,
            Self::Failed => StatusVariant::Error,
            Self::Paused | Self::Pending | Self::Stopped => StatusVariant::Default,
        }
    }

    /// What the status means, for its tooltip.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Running => "Doing its work.",
            Self::Preparing => "Getting ready before its real work can run at full speed.",
            Self::Pausing => "Asked to pause. It stops at its next checkpoint.",
            Self::Queued => "Waiting its turn, in priority order.",
            Self::Paused => "Held until resumed. It keeps its place.",
            Self::Pending => "Announced; not waiting for anything yet.",
            Self::Completed => "Finished its work.",
            Self::Failed => "Ended with an error.",
            Self::Stopped => "Stopped before it finished.",
        }
    }
}

/// A short coloured fact about a job: where it runs, its priority.
#[derive(Clone, Debug, PartialEq)]
pub struct JobBadge {
    pub text: String,
    pub color: Color,
}

impl JobBadge {
    pub fn new(text: impl Into<String>, color: Color) -> Self {
        Self {
            text: text.into(),
            color,
        }
    }
}

/// One job, as the list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    /// Stable across frames: widget ids and actions are built from it.
    pub id: String,
    pub title: String,
    /// The muted line under the title: what it runs, whose it is, what it
    /// last said.
    pub detail: String,
    pub status: JobStatus,
    /// Says the status in the job's own words -- "Tuning" rather than
    /// "Preparing" -- with its tooltip. `None` uses the status's own.
    pub status_text: Option<(String, String)>,
    /// From 0 to 1, when known.
    pub progress: Option<f32>,
    /// Under way with no way to say how far: drawn as a bar that moves rather
    /// than fills. Ignored when [`Self::progress`] is known.
    pub working: bool,
    /// Drawn in columns of their own, in order.
    pub badges: Vec<JobBadge>,
    /// How long, as a person reads it: "running 1 min 12 s".
    pub time: String,
    /// Under the time: "waited 2.1 s".
    pub time_detail: String,
    /// The [`JobGroup::id`] the job belongs to.
    pub group: Option<String>,
}

impl Job {
    pub fn new(id: impl Into<String>, title: impl Into<String>, status: JobStatus) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: String::new(),
            status,
            status_text: None,
            progress: None,
            working: false,
            badges: Vec::new(),
            time: String::new(),
            time_detail: String::new(),
            group: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Draws as under way without saying how far.
    pub fn working(mut self) -> Self {
        self.working = true;
        self
    }

    /// Names the status in the job's own words, with what it means.
    pub fn status_text(mut self, label: impl Into<String>, hint: impl Into<String>) -> Self {
        self.status_text = Some((label.into(), hint.into()));
        self
    }

    pub fn badge(mut self, badge: JobBadge) -> Self {
        self.badges.push(badge);
        self
    }

    pub fn time(mut self, time: impl Into<String>, detail: impl Into<String>) -> Self {
        self.time = time.into();
        self.time_detail = detail.into();
        self
    }

    pub fn in_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// A set of jobs steered as one.
#[derive(Clone, Debug, PartialEq)]
pub struct JobGroup {
    pub id: String,
    pub title: String,
    pub paused: bool,
}

impl JobGroup {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            paused: false,
        }
    }

    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }
}

/// What the person asked of a job or a group this frame.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum JobAction {
    Pause(String),
    Resume(String),
    Stop(String),
    PauseGroup(String),
    ResumeGroup(String),
    StopGroup(String),
}

/// Jobs under column headings, grouped: loose jobs first, then each group
/// with a heading of its own and its own controls.
///
/// Active jobs are tall rows with a progress bar and pause and stop buttons;
/// finished jobs are compact rows without controls, so the same list can show
/// history.
#[derive(Clone, Debug, PartialEq)]
pub struct JobList<'a> {
    jobs: &'a [Job],
    groups: &'a [JobGroup],
    controls: bool,
    badge_width: f32,
    empty: (&'a str, &'a str),
}

const HEADER: f32 = 22.0;
const ACTIVE_ROW: f32 = 58.0;
const FINISHED_ROW: f32 = 40.0;
const GROUP_ROW: f32 = 44.0;
const STATUS_WIDTH: f32 = 104.0;
const TIME_WIDTH: f32 = 130.0;
const ACTIONS_WIDTH: f32 = 92.0;

impl<'a> JobList<'a> {
    pub fn new(jobs: &'a [Job], groups: &'a [JobGroup]) -> Self {
        Self {
            jobs,
            groups,
            controls: true,
            badge_width: 96.0,
            empty: ("Nothing here", ""),
        }
    }

    /// Show pause and stop buttons. On by default.
    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    /// How wide each badge column is.
    pub fn badge_width(mut self, width: f32) -> Self {
        self.badge_width = width;
        self
    }

    /// What shows when there are no jobs.
    pub fn empty(mut self, title: &'a str, body: &'a str) -> Self {
        self.empty = (title, body);
        self
    }

    /// The jobs in the order they are drawn, each with the group heading to
    /// draw before it, if any.
    fn layout(&self) -> Vec<(Option<&'a JobGroup>, &'a Job)> {
        let mut out = Vec::with_capacity(self.jobs.len());
        let known = |id: &String| self.groups.iter().any(|g| &g.id == id);
        for job in self.jobs {
            if job.group.as_ref().is_none_or(|g| !known(g)) {
                out.push((None, job));
            }
        }
        for group in self.groups {
            let mut first = true;
            for job in self
                .jobs
                .iter()
                .filter(|j| j.group.as_ref() == Some(&group.id))
            {
                out.push((first.then_some(group), job));
                first = false;
            }
        }
        out
    }

    fn row_height(job: &Job) -> f32 {
        if job.status.is_active() {
            ACTIVE_ROW
        } else {
            FINISHED_ROW
        }
    }

    /// How tall the list draws.
    pub fn height(&self) -> f32 {
        if self.jobs.is_empty() {
            return 120.0;
        }
        HEADER
            + 4.0
            + self
                .layout()
                .iter()
                .map(|(group, job)| {
                    Self::row_height(job) + if group.is_some() { GROUP_ROW } else { 0.0 }
                })
                .sum::<f32>()
    }

    fn columns(&self, rect: Rect) -> Vec<Rect> {
        let badges = self.jobs.iter().map(|j| j.badges.len()).max().unwrap_or(0);
        let mut sizes = vec![Size::Fixed(STATUS_WIDTH), Size::FILL];
        sizes.extend(std::iter::repeat_n(Size::Fixed(self.badge_width), badges));
        sizes.push(Size::Fixed(TIME_WIDTH));
        if self.controls {
            sizes.push(Size::Fixed(ACTIONS_WIDTH));
        }
        Flex::row(0.0).place(rect, &sizes)
    }

    /// Draw the list down from the top of `rect`; `time` drives the pulse of
    /// running jobs. Returns what was asked of a job or a group.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, time: f32) -> Option<JobAction> {
        if self.jobs.is_empty() {
            super::containers::empty_state(
                ui,
                Rect {
                    height: 120.0,
                    ..rect
                },
                Icon::Activity,
                self.empty.0,
                self.empty.1,
            );
            return None;
        }
        let (header, list) = rect.split_top(HEADER, 4.0);
        let heading = self.columns(header);
        let inset = |r: Rect| r.inset_by(Insets::symmetric(8.0, 0.0));
        Label::muted()
            .size(11.0)
            .weight(600)
            .show(ui, inset(heading[0]), "Status");
        Label::muted()
            .size(11.0)
            .weight(600)
            .show(ui, inset(heading[1]), "Job");
        Label::muted().size(11.0).weight(600).show(
            ui,
            inset(heading[heading.len() - 1 - usize::from(self.controls)]),
            "Time",
        );

        let mut action = None;
        let mut rows = Stack::new(list, 0.0);
        for (index, (group, job)) in self.layout().into_iter().enumerate() {
            if let Some(group) = group {
                if let Some(a) = self.group_row(ui, id, rows.row(GROUP_ROW), group) {
                    action = Some(a);
                }
            }
            let row = rows.row(Self::row_height(job));
            if index % 2 == 1 {
                let p = ui.theme.palette;
                panel(ui, row, with_alpha(p.raised, 0.35), [0.0; 4], 8.0);
            }
            if let Some(a) = self.job_row(ui, id, row, job, time) {
                action = Some(a);
            }
        }
        action
    }

    fn group_row(&self, ui: &mut Ui, id: &str, rect: Rect, group: &JobGroup) -> Option<JobAction> {
        let p = ui.theme.palette;
        let rect = rect.inset_by(Insets::new(0.0, 6.0, 0.0, 4.0));
        panel(
            ui,
            rect,
            with_alpha(p.accent, 0.08),
            with_alpha(p.accent, 0.25),
            8.0,
        );
        let mut row = Strip::row(rect.inset_by(Insets::symmetric(10.0, 0.0)), 8.0);
        Icon::Layers.paint(ui, row.leading(18.0).centered(16.0, 16.0), p.accent);
        // The buttons line up with the job rows' buttons below them.
        let buttons = if self.controls {
            row.skip_trailing(ACTIONS_WIDTH - 4.0);
            Some(Rect::new(
                rect.right() - ACTIONS_WIDTH + 8.0,
                rect.y,
                ACTIONS_WIDTH - 8.0,
                rect.height,
            ))
        } else {
            None
        };
        let count = self
            .jobs
            .iter()
            .filter(|j| j.group.as_ref() == Some(&group.id))
            .count();
        let summary = format!(
            "{count} {}{}",
            if count == 1 { "job" } else { "jobs" },
            if group.paused { " · paused" } else { "" }
        );
        let count_label = Label::muted().size(12.0).align(TextAlign::Right);
        let count_width = count_label.width(ui, &summary) + 4.0;
        count_label.show(ui, row.trailing(count_width), &summary);
        Label::new()
            .size(13.5)
            .weight(600)
            .ellipsis()
            .show(ui, row.rest(), &group.title);

        let buttons = buttons?;
        let mut buttons = Strip::row(buttons.center_vertically(28.0), 6.0);
        let key = format!("{id}.group.{}", group.id);
        let (icon, hint, pause) = if group.paused {
            (
                Icon::Play,
                "Resume every job in the group",
                JobAction::ResumeGroup(group.id.clone()),
            )
        } else {
            (
                Icon::Pause,
                "Pause every job in the group",
                JobAction::PauseGroup(group.id.clone()),
            )
        };
        let mut action = None;
        let pause_id = format!("{key}.pause");
        if Button::icon_only(icon)
            .variant(ButtonVariant::Secondary)
            .show(ui, &pause_id, buttons.leading(30.0))
            .clicked
        {
            action = Some(pause);
        }
        ui.tooltip(&pause_id, hint);
        let stop_id = format!("{key}.stop");
        if Button::icon_only(Icon::Stop)
            .variant(ButtonVariant::Danger)
            .show(ui, &stop_id, buttons.leading(30.0))
            .clicked
        {
            action = Some(JobAction::StopGroup(group.id.clone()));
        }
        ui.tooltip(&stop_id, "Stop every job in the group");
        action
    }

    fn job_row(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        job: &Job,
        time: f32,
    ) -> Option<JobAction> {
        let cells = self.columns(rect);
        let inset = |r: Rect| r.inset_by(Insets::symmetric(8.0, 0.0));
        let key = format!("{id}.job.{}", job.id);
        let active = job.status.is_active();
        let color = job.status.color(ui);

        // Status, breathing while running.
        let badge = inset(cells[0]).center_vertically(if active { 24.0 } else { 22.0 });
        if matches!(job.status, JobStatus::Running | JobStatus::Preparing) {
            let pulse = 0.6 + 0.4 * (time * 3.0).sin().abs();
            panel(
                ui,
                badge,
                with_alpha(color, 0.12 * pulse),
                with_alpha(color, 0.3),
                6.0,
            );
        }
        let (label, hint) = match &job.status_text {
            Some((label, hint)) => (label.as_str(), hint.as_str()),
            None => (job.status.label(), job.status.hint()),
        };
        Badge::new(job.status.variant()).show(ui, badge, label);
        let status_id = format!("{key}.status");
        ui.interact(&status_id, badge);
        ui.tooltip(&status_id, hint);

        // Title and detail; progress under them while there is work left.
        let main = inset(cells[1]);
        if active {
            let barred = job.progress.is_some() || job.working;
            let (lines, bar) = if barred {
                main.center_vertically(46.0).split_bottom(6.0, 4.0)
            } else {
                (main.center_vertically(40.0), Rect::new(0.0, 0.0, 0.0, 0.0))
            };
            let mut lines = Strip::column(lines, 2.0);
            Label::new().size(14.0).weight(600).ellipsis().show(
                ui,
                lines.leading(20.0),
                &job.title,
            );
            Label::muted()
                .size(12.0)
                .ellipsis()
                .show(ui, lines.rest(), &job.detail);
            match job.progress {
                Some(progress) => ProgressBar::new().color(color).show(ui, bar, progress),
                None if job.working => ProgressBar::new().color(color).show_working(ui, bar, time),
                None => {}
            }
        } else {
            let mut line = Strip::row(main, 8.0);
            let title = Label::new().size(13.0).weight(600);
            let width = (title.width(ui, &job.title) + 4.0).min(main.width * 0.5);
            title.ellipsis().show(ui, line.leading(width), &job.title);
            Label::muted()
                .size(12.0)
                .ellipsis()
                .show(ui, line.rest(), &job.detail);
        }

        for (badge, cell) in job.badges.iter().zip(&cells[2..]) {
            Badge::new(StatusVariant::Default)
                .color(badge.color)
                .ellipsis()
                .show(ui, inset(*cell).center_vertically(24.0), &badge.text);
        }

        let time_cell = inset(cells[cells.len() - 1 - usize::from(self.controls)]);
        if active {
            let mut lines = Strip::column(time_cell.center_vertically(40.0), 2.0);
            Label::new()
                .size(13.0)
                .weight(500)
                .ellipsis()
                .show(ui, lines.leading(20.0), &job.time);
            Label::muted()
                .size(11.0)
                .ellipsis()
                .show(ui, lines.rest(), &job.time_detail);
        } else {
            Label::muted()
                .size(12.0)
                .ellipsis()
                .show(ui, time_cell, &job.time);
        }

        if !self.controls || !active {
            return None;
        }
        let mut buttons = Strip::row(inset(cells[cells.len() - 1]).center_vertically(32.0), 6.0);
        let (icon, hint, pause) = if job.status.is_paused() {
            (Icon::Play, "Resume", JobAction::Resume(job.id.clone()))
        } else {
            (Icon::Pause, "Pause", JobAction::Pause(job.id.clone()))
        };
        let mut action = None;
        let pause_id = format!("{key}.pause");
        if Button::icon_only(icon)
            .variant(ButtonVariant::Secondary)
            .show(ui, &pause_id, buttons.leading(34.0))
            .clicked
        {
            action = Some(pause);
        }
        ui.tooltip(&pause_id, hint);
        let stop_id = format!("{key}.stop");
        if Button::icon_only(Icon::Stop)
            .variant(ButtonVariant::Danger)
            .show(ui, &stop_id, buttons.leading(34.0))
            .clicked
        {
            action = Some(JobAction::Stop(job.id.clone()));
        }
        ui.tooltip(
            &stop_id,
            if matches!(
                job.status,
                JobStatus::Running | JobStatus::Preparing | JobStatus::Pausing
            ) {
                "Stop: abandon the work"
            } else {
                "Stop: withdraw it before it runs"
            },
        );
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Harness;

    fn jobs() -> (Vec<Job>, Vec<JobGroup>) {
        let jobs = vec![
            Job::new("a", "Stage A", JobStatus::Running)
                .in_group("g")
                .progress(0.5),
            Job::new("b", "Loose", JobStatus::Queued),
            Job::new("c", "Stage C", JobStatus::Paused).in_group("g"),
            Job::new("d", "Done", JobStatus::Completed),
        ];
        let groups = vec![JobGroup::new("g", "Pipeline")];
        (jobs, groups)
    }

    #[test]
    fn loose_jobs_come_before_groups_and_group_heading_comes_once() {
        let (jobs, groups) = jobs();
        let list = JobList::new(&jobs, &groups);
        let order: Vec<(bool, &str)> = list
            .layout()
            .iter()
            .map(|(g, j)| (g.is_some(), j.id.as_str()))
            .collect();
        assert_eq!(
            order,
            vec![(false, "b"), (false, "d"), (true, "a"), (false, "c")]
        );
        let expected = HEADER + 4.0 + ACTIVE_ROW + FINISHED_ROW + GROUP_ROW + ACTIVE_ROW * 2.0;
        assert_eq!(list.height(), expected);
    }

    #[test]
    fn buttons_ask_for_the_right_action() {
        let (jobs, groups) = jobs();
        let mut harness = Harness::new(Rect::new(0.0, 0.0, 900.0, 600.0));
        let action = std::cell::RefCell::new(None);
        let mut view = |ui: &mut Ui| {
            if let Some(a) = JobList::new(&jobs, &groups).show(
                ui,
                "jobs",
                Rect::new(0.0, 0.0, 900.0, 600.0),
                0.0,
            ) {
                *action.borrow_mut() = Some(a);
            }
        };
        harness.run(&mut view);
        assert!(harness.click("jobs.job.a.stop", &mut view));
        assert_eq!(
            action.borrow_mut().take(),
            Some(JobAction::Stop("a".into()))
        );
        // A paused job's control resumes it.
        assert!(harness.click("jobs.job.c.pause", &mut view));
        assert_eq!(
            action.borrow_mut().take(),
            Some(JobAction::Resume("c".into()))
        );
        assert!(harness.click("jobs.group.g.pause", &mut view));
        assert_eq!(
            action.borrow_mut().take(),
            Some(JobAction::PauseGroup("g".into()))
        );
        // A finished job has no controls.
        assert!(!harness.click("jobs.job.d.stop", &mut view));
    }
}
