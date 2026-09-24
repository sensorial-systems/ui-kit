//! A curve editor: any number of parameters over time, as keyframes and the
//! curves between them.
//!
//! The editor is the bottom half of a video editor -- a track per animated
//! parameter down the left, time across the top, and the curves themselves in
//! the middle, where the keys are dragged. It edits a
//! [`ui_kit_core::animation::Animation`] in place, so whatever renders the
//! animation reads the same tracks the pointer just moved, and it has no idea
//! what any of them mean: a track is an id, a name, a range and a unit, which
//! is what lets one editor drive a camera, a colour, a synthesiser or a
//! simulation.
//!
//! Tracks are drawn against their own [`Track::range`], normalised, so a
//! field of view in degrees and a splat size in multiples share one box
//! without either being squashed flat. The value axis is therefore a
//! fraction of each track's range; the numbers beside it are the selected
//! track's.
//!
//! A key is dragged by its diamond and typed in the toolbar: selecting one
//! puts its time and its value in fields there, for the numbers a pointer
//! cannot say exactly.
//!
//! What can be added comes from the caller as [`TrackOption`]s -- the
//! parameters it is willing to animate, and what each is worth now -- and the
//! "Add parameter" menu turns the chosen one into a track keyed at the
//! playhead. Nothing else is needed to make a new parameter animatable.

use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::input::NumberField;
use super::menu::{Menu, MenuItem, MenuState};
use super::theme::{with_alpha, DANGER, INFO, SUCCESS, WARNING};
use super::Label;
use crate::icon::Icon;
use crate::vector::{circle, line, polyline};
use crate::{CursorIcon, Key, Stack, Strip, Ui};
use ui_kit_core::animation::{Animation, Handle, Interpolation, Keyframe, Track};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// The colours new tracks are given, in turn.
pub const TRACK_COLORS: [Color; 6] = [
    INFO,
    SUCCESS,
    WARNING,
    DANGER,
    [0.639, 0.475, 0.973, 1.0],
    [0.086, 0.761, 0.784, 1.0],
];

/// The toolbar across the top, the time ruler under it, and a track's row.
const TOOLBAR: f32 = 26.0;
const RULER: f32 = 18.0;
const ROW: f32 = 22.0;
/// How wide the column of track names is by default.
const NAMES: f32 = 208.0;
/// How big a keyframe is drawn, and how near the pointer has to be to catch
/// one. The catch is deliberately much larger than the diamond.
const KEY: f32 = 4.5;
const GRAB: f32 = 15.0;
/// A Bezier tangent's end.
const TANGENT: f32 = 3.5;
/// The selected key's own numbers in the toolbar: how wide a field is, the
/// caption beside it, and the gap between them. A pair is a caption, a gap
/// and a field, and the value's pair is drawn before the time's is given
/// room, so a narrow toolbar keeps the number that is usually wanted exactly.
const CAPTION: f32 = 30.0;
const FIELD: f32 = 64.0;
const FIELD_GAP: f32 = 6.0;
/// How many decimals a key's time is typed to. Time here is whatever the
/// tracks count in, so this is the same three places the toolbar reads a
/// continuous playhead to.
const TIME_DECIMALS: u8 = 3;

/// How near a curve the pointer has to be for a double click to add a key to
/// that track rather than to the selected one.
const NEAR_CURVE: f32 = 14.0;
/// How many points a segment is drawn with. A curve is only ever a couple of
/// hundred pixels wide here, so this is plenty and costs nothing.
const SAMPLES: usize = 24;

/// A parameter the editor offers to animate, and what it is worth now.
///
/// The caller builds one of these per number it is willing to have keyed;
/// choosing one from the menu makes a [`Track`] out of it, keyed at the
/// playhead with [`TrackOption::value`].
#[derive(Clone, Debug, PartialEq)]
pub struct TrackOption {
    pub id: String,
    pub name: String,
    /// What the parameter is set to right now, which the first key takes.
    pub value: f32,
    /// How far the parameter can be dragged, which is also the band the
    /// track's curve is drawn across.
    pub range: (f32, f32),
    /// Shown after the numbers: `"\u{b0}"`, `"x"`, `"px"`.
    pub unit: String,
    pub decimals: u8,
}

impl TrackOption {
    pub fn new(id: impl Into<String>, name: impl Into<String>, value: f32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            value,
            range: (0.0, 1.0),
            unit: String::new(),
            decimals: 2,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.range = (min, max);
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// The track this option makes, keyed once at `time` so the curve starts
    /// out holding the value the parameter already had.
    pub fn track(&self, time: f32, color: Color) -> Track {
        Track::new(self.id.clone(), self.name.clone())
            .with_range(self.range.0, self.range.1)
            .with_default(self.value)
            .with_unit(self.unit.clone())
            .with_decimals(self.decimals)
            .with_color(color)
            .with_keys([Keyframe::new(time, self.value)])
    }
}

/// What part of the animation is on screen: a span of time, and a band of
/// every track's own range, 0 at the bottom of the range and 1 at the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveView {
    pub start: f32,
    pub end: f32,
    pub low: f32,
    pub high: f32,
}

impl Default for CurveView {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            low: -0.08,
            high: 1.08,
        }
    }
}

impl CurveView {
    /// The whole of `duration` with a little room above and below the ranges.
    pub fn whole(duration: f32) -> Self {
        Self {
            start: 0.0,
            end: duration.max(1e-3),
            ..Self::default()
        }
    }

    fn span(&self) -> f32 {
        (self.end - self.start).max(1e-6)
    }

    fn band(&self) -> f32 {
        (self.high - self.low).max(1e-6)
    }

    /// Zoom by `factor` about the fractions `at` of the view, so what is
    /// under the pointer stays under it.
    fn zoom(&mut self, factor: (f32, f32), at: (f32, f32)) {
        let (time, value) = (
            self.start + self.span() * at.0,
            self.low + self.band() * at.1,
        );
        let (span, band) = (
            (self.span() * factor.0).clamp(1e-3, 1e6),
            (self.band() * factor.1).clamp(1e-3, 1e4),
        );
        self.start = time - span * at.0;
        self.end = self.start + span;
        self.low = value - band * at.1;
        self.high = self.low + band;
    }
}

/// What the editor is in the middle of: a key being dragged, a tangent being
/// pulled, the playhead being scrubbed, or the view being pushed about.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Grab {
    /// A key, with the offset from the pointer it was caught at so it does
    /// not jump under the cursor.
    Key {
        offset: (f32, f32),
    },
    Tangent {
        outgoing: bool,
    },
    Scrub,
    Pan {
        pointer: Point,
        view: CurveView,
    },
}

/// What the editor keeps between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveEditorState {
    /// The track whose values the axis is labelled with, and whose curve the
    /// toolbar acts on.
    pub track: Option<usize>,
    /// The key selected within [`Self::track`].
    pub key: Option<usize>,
    pub view: CurveView,
    /// Whether keys land on whole frames.
    pub snap: bool,
    /// How far the list of tracks is scrolled.
    pub scroll: f32,
    /// The "Add parameter" menu.
    pub menu: MenuState,
    grab: Option<Grab>,
    /// Whether the view has been put over the curves yet. Done once, the
    /// first frame there is anything to put it over.
    fitted: bool,
}

impl CurveEditorState {
    /// Showing the whole of a `duration`-long animation, snapping to frames.
    pub fn new(duration: f32) -> Self {
        Self {
            view: CurveView::whole(duration),
            snap: true,
            ..Self::default()
        }
    }

    /// Forget the selection: what a caller does when it removes a track
    /// behind the editor's back.
    pub fn deselect(&mut self) {
        self.track = None;
        self.key = None;
    }

    /// The key the toolbar and the Delete key act on.
    fn selected(&self, animation: &Animation) -> Option<(usize, usize)> {
        let track = self.track?;
        let key = self.key?;
        (animation.tracks.get(track)?.keys.len() > key).then_some((track, key))
    }
}

/// What the editor did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveEditorResponse {
    /// A curve changed: a key moved, was added, removed or reshaped.
    pub changed: bool,
    /// The playhead was dragged, so whatever plays the animation should stop
    /// and go where it was put.
    pub scrubbed: bool,
    /// The pointer is over the editor, so a press belongs to it.
    pub hovered: bool,
    /// The id of a parameter added as a track this frame.
    pub added: Option<String>,
    /// The id of a track removed this frame.
    pub removed: Option<String>,
}

/// Parameters over time: a track list, a time ruler and the curves.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveEditor<'a> {
    duration: f32,
    fps: f32,
    options: &'a [TrackOption],
    names: f32,
}

impl<'a> CurveEditor<'a> {
    /// An editor over an animation `duration` long, in whatever the tracks
    /// count time in.
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(1e-3),
            fps: 0.0,
            options: &[],
            names: NAMES,
        }
    }

    /// How many frames a unit of time is, which the ruler counts in and keys
    /// snap to. Zero leaves the time continuous.
    pub fn fps(mut self, fps: f32) -> Self {
        self.fps = fps.max(0.0);
        self
    }

    /// The parameters the "Add parameter" menu offers. None hides the menu.
    pub fn options(mut self, options: &'a [TrackOption]) -> Self {
        self.options = options;
        self
    }

    pub fn names_width(mut self, width: f32) -> Self {
        self.names = width.max(60.0);
        self
    }

    /// How tall an editor showing `tracks` tracks wants to be, up to `most`
    /// of them before the list starts scrolling.
    pub fn height(tracks: usize, most: usize) -> f32 {
        TOOLBAR + 6.0 + RULER + 2.0 + (tracks.clamp(2, most.max(2)) as f32 * ROW).max(120.0) + 16.0
    }

    /// Snap `time` to the nearest frame, where there are frames to snap to.
    fn snapped(&self, time: f32, state: &CurveEditorState) -> f32 {
        if state.snap && self.fps > 0.0 {
            (time * self.fps).round() / self.fps
        } else {
            time
        }
    }

    /// Draw the editor in `rect` and act on whatever the pointer does to it.
    ///
    /// `time` is the playhead, in and out: the editor draws it where the
    /// caller has it, and moves it when it is scrubbed.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: &mut f32,
    ) -> CurveEditorResponse {
        let p = ui.theme.palette;
        let mut response = CurveEditorResponse {
            hovered: ui.pointer_over(rect),
            ..Default::default()
        };
        panel(ui, rect, p.surface, p.border, 10.0);

        // A view left at its default, or one the duration has outgrown, is
        // put over the whole animation rather than drawn as a sliver.
        if state.view.end <= state.view.start {
            state.view = CurveView::whole(self.duration);
        }
        if state
            .track
            .is_some_and(|index| index >= animation.tracks.len())
        {
            state.deselect();
        }
        // Something is always the track whose numbers the value axis is
        // labelled with, or the axis would read in percentages while there
        // is a perfectly good track to read it in.
        if state.track.is_none() && !animation.tracks.is_empty() {
            state.track = Some(0);
        }
        // A parameter's range is as wide as its control lets it be dragged,
        // which is usually far wider than the part of it a curve uses, so
        // curves handed to the editor rather than drawn in it would arrive
        // as flat lines along one edge.
        if !state.fitted && !animation.tracks.is_empty() {
            state.view = self.fitted(animation);
            state.fitted = true;
        }

        let inner = rect.inset(8.0);
        let (toolbar, body) = inner.split_top(TOOLBAR, 6.0);
        let (names, right) = body.split_left(self.names, 8.0);
        let (ruler, plot) = right.split_top(RULER, 2.0);
        let (heading, rows) = names.split_top(RULER, 2.0);

        let map = Map {
            id,
            rect: plot,
            view: state.view,
        };
        response.changed |= self.toolbar(ui, id, toolbar, animation, state, *time);
        response.added = self.adder(ui, id, heading, animation, state, *time);
        response.changed |= response.added.is_some();
        let listed = self.rows(ui, id, rows, animation, state, *time);
        response.changed |= listed.changed;
        response.removed = listed.removed;
        response.scrubbed |= self.ruler(ui, ruler, &map, state, time);
        self.plot(ui, &map, animation, state, time, &mut response);

        // Delete acts on the selection wherever the pointer is, as long as
        // the editor is what was last pressed inside -- and as long as a
        // field does not have the keyboard, where Backspace takes a digit
        // off a number rather than the key that number belongs to.
        if let Some((track, key)) = state.selected(animation) {
            let typing = ui.wants_keyboard();
            if response.hovered
                && !typing
                && (ui.consume_key(Key::Delete) || ui.consume_key(Key::Backspace))
            {
                animation.tracks[track].remove(key);
                state.key = None;
                response.changed = true;
            }
        }
        response
    }

    /// What the selected key is, and what can be done to it.
    fn toolbar(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: f32,
    ) -> bool {
        let mut changed = false;
        let selected = state.selected(animation);
        let (left, right) = rect.split_right(268.0, 8.0);

        // The interpolation of the selected key, which is the shape of the
        // segment leaving it.
        let current = selected.map(|(t, k)| animation.tracks[t].keys[k].interpolation);
        let (modes, fields) = left.split_left(240.0, 8.0);
        let cells = modes.columns(Interpolation::ALL.len(), 4.0);
        for (mode, cell) in Interpolation::ALL.into_iter().zip(cells) {
            let chosen = current == Some(mode);
            let pressed = Button::new(mode.name())
                .variant(if chosen {
                    ButtonVariant::Selected
                } else {
                    ButtonVariant::Secondary
                })
                .font_size(11.0)
                .disabled(selected.is_none())
                .show(ui, &format!("{id}.mode.{mode:?}"), cell)
                .clicked;
            if let (true, Some((track, key))) = (pressed, selected) {
                animation.tracks[track].keys[key].interpolation = mode;
                changed = true;
            }
        }
        changed |= self.key_fields(ui, id, fields, animation, state);

        let mut bar = right;
        let (snap, rest) = bar.split_left(58.0, 4.0);
        bar = rest;
        if Button::new("Snap")
            .variant(if state.snap {
                ButtonVariant::Selected
            } else {
                ButtonVariant::Secondary
            })
            .font_size(11.0)
            .disabled(self.fps <= 0.0)
            .show(ui, &format!("{id}.snap"), snap)
            .clicked
        {
            state.snap = !state.snap;
        }
        let (fit, rest) = bar.split_left(46.0, 4.0);
        bar = rest;
        if Button::new("Fit")
            .variant(ButtonVariant::Secondary)
            .font_size(11.0)
            .show(ui, &format!("{id}.fit"), fit)
            .clicked
        {
            state.view = self.fitted(animation);
        }
        let (out, rest) = bar.split_left(28.0, 4.0);
        let (into, readout) = rest.split_left(28.0, 8.0);
        for (index, cell) in [out, into].into_iter().enumerate() {
            let icon = if index == 0 { Icon::Minus } else { Icon::Plus };
            if Button::icon_only(icon)
                .variant(ButtonVariant::Secondary)
                .show(ui, &format!("{id}.zoom.{index}"), cell)
                .clicked
            {
                let factor = if index == 0 { 1.4 } else { 1.0 / 1.4 };
                state.view.zoom((factor, 1.0), (0.5, 0.5));
            }
        }
        Label::muted()
            .size(11.0)
            .align(TextAlign::Right)
            .show(ui, readout, &self.moment(time));
        changed
    }

    /// The selected key's own numbers, typed rather than dragged.
    ///
    /// Dragging a diamond is how a curve is shaped, but some numbers are
    /// arrived at rather than found -- a turn that ends at exactly 360
    /// degrees, a key two seconds in on the nose -- and no pointer can say
    /// them. The fields write through the same move a drag does, so a key
    /// typed past its neighbours slides past them just the same.
    ///
    /// What is typed is taken exactly as it is written: snapping is what a
    /// pointer needs, not someone who has just said 1.234. The value is
    /// still held to the track's range, which is the one bound a curve may
    /// never leave.
    fn key_fields(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        animation: &mut Animation,
        state: &mut CurveEditorState,
    ) -> bool {
        // What the toolbar has left after the modes and the view controls,
        // which on a narrow window is nothing at all.
        let pair = CAPTION + FIELD_GAP + FIELD;
        if rect.width < pair {
            return false;
        }
        let Some((index, key)) = state.selected(animation) else {
            Label::muted()
                .size(11.0)
                .ellipsis()
                .show(ui, rect, "Select a key to type its numbers");
            return false;
        };
        let track = &animation.tracks[index];
        let (locked, decimals, range) = (track.locked, track.decimals, track.range);
        let unit = track.unit.clone();
        let at = track.keys[key];
        let (mut time, mut value) = (at.time, at.value);

        let caption = |ui: &mut Ui, rect: Rect, text: &str| {
            Label::muted().size(11.0).show(ui, rect, text);
        };
        let number = |bounds: (f32, f32), decimals: u8| {
            NumberField::new(bounds.0, bounds.1)
                .decimals(decimals)
                .font_size(11.0)
                .disabled(locked)
        };

        let mut bar = Strip::row(rect, FIELD_GAP);
        let unit_room = if unit.is_empty() { 0.0 } else { 20.0 };
        // The time only where both pairs fit: the value is the one a curve is
        // usually being told exactly.
        if bar.remaining() >= pair * 2.0 + FIELD_GAP + unit_room {
            caption(ui, bar.leading(CAPTION), "Time");
            number((0.0, self.duration), TIME_DECIMALS).show(
                ui,
                &format!("{id}.selected.time"),
                bar.leading(FIELD),
                &mut time,
            );
        }
        caption(ui, bar.leading(CAPTION), "Value");
        number(range, decimals).show(
            ui,
            &format!("{id}.selected.value"),
            bar.leading(FIELD),
            &mut value,
        );
        if unit_room > 0.0 {
            caption(ui, bar.leading(unit_room), &unit);
        }

        if time == at.time && value == at.value {
            return false;
        }
        state.key = Some(animation.tracks[index].move_key(key, time, value));
        true
    }

    /// The heading over the track names, and the menu that adds one.
    fn adder(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: f32,
    ) -> Option<String> {
        if self.options.is_empty() {
            Label::muted().size(11.0).show(ui, rect, "Parameters");
            return None;
        }
        // Only what is not already a track: adding the same one twice would
        // replace the curve that is there.
        let children: Vec<MenuItem> = self
            .options
            .iter()
            .filter(|option| animation.track(&option.id).is_none())
            .map(|option| MenuItem::new(option.id.clone(), option.name.clone()))
            .collect();
        let label = if children.is_empty() {
            "All parameters added"
        } else {
            "Add parameter"
        };
        let items = [MenuItem::new("add", label).with_children(children)];
        let chosen = Menu::horizontal(&items)
            .widths(&[rect.width])
            .popup_width(220.0)
            .show(ui, &format!("{id}.add"), rect, &mut state.menu)?;
        let option = self.options.iter().find(|option| option.id == chosen)?;
        let color = TRACK_COLORS[animation.tracks.len() % TRACK_COLORS.len()];
        let index = animation.add(option.track(self.snapped(time, state), color));
        state.track = Some(index);
        state.key = Some(0);
        state.view = self.fitted(animation);
        Some(chosen)
    }

    /// A row per track: key it here, what it is worth now, show it, drop it.
    fn rows(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: f32,
    ) -> Rows {
        let mut listed = Rows::default();
        let p = ui.theme.palette;
        if animation.tracks.is_empty() {
            Label::muted().size(12.0).show(
                ui,
                rect.split_top(40.0, 0.0).0,
                if self.options.is_empty() {
                    "No tracks."
                } else {
                    "No tracks yet. Add a parameter to key it."
                },
            );
            return listed;
        }
        // The list scrolls when there are more tracks than rows to show them.
        let content = animation.tracks.len() as f32 * ROW;
        let most = (content - rect.height).max(0.0);
        if ui.take_scroll(rect).y != 0.0 {
            state.scroll += ui.input().scroll.y;
        }
        state.scroll = state.scroll.clamp(0.0, most);

        let mut remove = None;
        let scrolled = Rect {
            y: rect.y - state.scroll,
            ..rect
        };
        let mut stack = Stack::new(scrolled, 0.0);
        for index in 0..animation.tracks.len() {
            let row = stack.row(ROW);
            if !row.overlaps(rect) {
                continue;
            }
            ui.with_clip(rect, |ui| {
                let selected = state.track == Some(index);
                if selected {
                    panel(
                        ui,
                        row.inset(1.0),
                        with_alpha(p.accent, 0.10),
                        [0.0; 4],
                        4.0,
                    );
                }
                let (key, rest) = row.split_left(22.0, 2.0);
                let (body, controls) = rest.split_right(44.0, 2.0);
                let (name, value) = body.split_right(66.0, 4.0);

                // The diamond keys the track here, or takes that key away
                // again: the one gesture a curve editor is built around.
                let here = animation.tracks[index].index_at(self.snapped(time, state));
                let color = animation.tracks[index].color;
                if Button::icon_only(if here.is_some() {
                    Icon::Check
                } else {
                    Icon::Plus
                })
                .variant(ButtonVariant::Text)
                .font_size(11.0)
                .show(
                    ui,
                    &format!("{id}.key.{index}"),
                    key.center_vertically(18.0),
                )
                .clicked
                {
                    let at = self.snapped(time, state);
                    let track = &mut animation.tracks[index];
                    match here {
                        Some(key) => {
                            track.remove(key);
                        }
                        None => {
                            let key = track.pin(at);
                            state.track = Some(index);
                            state.key = Some(key);
                        }
                    }
                    listed.changed = true;
                }

                if ui.interact(&format!("{id}.row.{index}"), name).clicked {
                    state.track = Some(index);
                    state.key = None;
                }
                let track = &animation.tracks[index];
                Label::new()
                    .size(12.0)
                    .color(if track.visible { color } else { p.muted })
                    .ellipsis()
                    .show(ui, name, &track.name);
                Label::muted().size(11.0).align(TextAlign::Right).show(
                    ui,
                    value,
                    &shown(track, track.value_at(time)),
                );

                let cells = controls.columns(2, 2.0);
                let visible = track.visible;
                if Button::icon_only(if visible { Icon::Circle } else { Icon::Dot })
                    .variant(ButtonVariant::Text)
                    .show(
                        ui,
                        &format!("{id}.visible.{index}"),
                        cells[0].center_vertically(18.0),
                    )
                    .clicked
                {
                    animation.tracks[index].visible = !visible;
                }
                if Button::icon_only(Icon::Close)
                    .variant(ButtonVariant::Text)
                    .show(
                        ui,
                        &format!("{id}.drop.{index}"),
                        cells[1].center_vertically(18.0),
                    )
                    .clicked
                {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            listed.removed = Some(animation.tracks.remove(index).id);
            state.deselect();
            listed.changed = true;
        }
        listed
    }

    /// Time across the top, with the playhead's head on it.
    fn ruler(
        &self,
        ui: &mut Ui,
        rect: Rect,
        map: &Map<'_>,
        state: &mut CurveEditorState,
        time: &mut f32,
    ) -> bool {
        let (id, plot) = (map.id, map.rect);
        let mut scrubbed = false;
        let p = ui.theme.palette;
        let view = state.view;
        let step = nice_step(view.span() / (rect.width / 74.0).max(2.0));
        let first = (view.start / step).ceil() * step;
        let mut at = first;
        while at <= view.end && step > 0.0 {
            let x = plot.x + (at - view.start) / view.span() * plot.width;
            line(
                ui,
                Point::new(x, rect.bottom() - 4.0),
                Point::new(x, rect.bottom()),
                1.0,
                p.border,
            );
            Label::muted().size(10.0).show(
                ui,
                Rect::new(x + 3.0, rect.y, 60.0, rect.height - 4.0),
                &format_time(at, step),
            );
            at += step;
        }

        // Scrubbing: anywhere along the ruler, and carried on wherever the
        // pointer goes once it has hold.
        let scrub = ui.interact(&format!("{id}.scrub"), rect);
        if scrub.hovered {
            ui.set_cursor(CursorIcon::ResizeHorizontal);
        }
        if scrub.active {
            state.grab = Some(Grab::Scrub);
        }
        if state.grab == Some(Grab::Scrub) {
            if let Some(pt) = ui.input().pointer.filter(|_| ui.input().down) {
                let at = view.start + (pt.x - plot.x) / plot.width.max(1.0) * view.span();
                *time = self.snapped(at, state).clamp(0.0, self.duration);
                scrubbed = true;
            }
        }

        let x = plot.x + (*time - view.start) / view.span() * plot.width;
        if (plot.x..=plot.right()).contains(&x) {
            panel(
                ui,
                Rect::new(x - 5.0, rect.bottom() - 7.0, 10.0, 7.0),
                p.accent,
                [0.0; 4],
                2.0,
            );
        }
        scrubbed
    }

    /// The curves themselves, and everything the pointer does to them.
    fn plot(
        &self,
        ui: &mut Ui,
        map: &Map<'_>,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: &mut f32,
        response: &mut CurveEditorResponse,
    ) {
        let rect = map.rect;
        let p = ui.theme.palette;
        panel(ui, rect, with_alpha(p.background, 0.45), p.border, 6.0);

        // The value axis, in the selected track's own numbers where there is
        // one and as a share of each track's range where there is not. The
        // lines follow the view rather than sitting at fixed fractions, so
        // they are still there once it has been zoomed into a curve.
        let labelled = state.track.and_then(|index| animation.tracks.get(index));
        let (base, span) = labelled.map_or((0.0, 1.0), |track| (track.range.0, track_span(track)));
        let (from, to) = (base + map.view.low * span, base + map.view.high * span);
        let step = nice_step((to - from) / (rect.height / 34.0).max(2.0));
        let mut at = (from / step).ceil() * step;
        for _ in 0..24 {
            if at > to {
                break;
            }
            let y = map.y_normalised((at - base) / span);
            line(
                ui,
                Point::new(rect.x, y),
                Point::new(rect.right(), y),
                1.0,
                with_alpha(p.border, 0.5),
            );
            let text = match labelled {
                Some(track) => shown(track, at),
                None => format!("{:.0}%", at * 100.0),
            };
            Label::muted().size(10.0).show(
                ui,
                Rect::new(rect.x + 4.0, y - 12.0, 84.0, 12.0),
                &text,
            );
            at += step;
        }
        // The same times the ruler labelled, carried down the plot.
        let step = nice_step(map.view.span() / (rect.width / 74.0).max(2.0));
        let mut at = (map.view.start / step).ceil() * step;
        while at <= map.view.end && step > 0.0 {
            let x = map.x(at);
            line(
                ui,
                Point::new(x, rect.y),
                Point::new(x, rect.bottom()),
                1.0,
                with_alpha(p.border, 0.35),
            );
            at += step;
        }

        ui.with_clip(rect, |ui| {
            for index in 0..animation.tracks.len() {
                if animation.tracks[index].visible {
                    self.curve(
                        ui,
                        map,
                        &animation.tracks[index],
                        state.track == Some(index),
                    );
                }
            }
        });

        // Keys take the pointer before the tangents, the tangents before the
        // playhead and the playhead before the pan, so the smallest thing
        // under the pointer is always what gets caught.
        let deleted = self.keys(ui, map, animation, state, response);
        self.tangents(ui, map, animation, state, response);
        self.playhead(ui, map, state, time, response);
        self.navigate(ui, map, animation, state, response, deleted);
    }

    /// One track's curve, and a line either side of the keys it holds at.
    fn curve(&self, ui: &mut Ui, map: &Map<'_>, track: &Track, selected: bool) {
        let width = if selected { 2.0 } else { 1.5 };
        let color = if track.locked {
            with_alpha(track.color, 0.4)
        } else {
            track.color
        };
        let points: Vec<Point> = track
            .sample(SAMPLES)
            .into_iter()
            .map(|(time, value)| Point::new(map.x(time), map.y(track, value)))
            .collect();
        if points.len() >= 2 {
            polyline(ui, &points, width, color);
        }
        // Before the first key and after the last the value holds, which is
        // as much a part of the curve as the segments between them.
        if let (Some(first), Some(last)) = (track.keys.first(), track.keys.last()) {
            let held = with_alpha(color, 0.45);
            let (left, right) = (map.x(first.time), map.x(last.time));
            line(
                ui,
                Point::new(map.rect.x, map.y(track, first.value)),
                Point::new(left, map.y(track, first.value)),
                width,
                held,
            );
            line(
                ui,
                Point::new(right, map.y(track, last.value)),
                Point::new(map.rect.right(), map.y(track, last.value)),
                width,
                held,
            );
        }
    }

    /// Every key of every visible track, caught and dragged -- and taken
    /// away by a double click on its diamond.
    ///
    /// Says whether a key took this frame's double click, which the
    /// background needs to know: the same double click is over the curve, and
    /// adding a key back where one was just deleted is not what was asked.
    fn keys(
        &self,
        ui: &mut Ui,
        map: &Map<'_>,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        response: &mut CurveEditorResponse,
    ) -> bool {
        let id = map.id;
        let mut deleted = false;
        let white = [1.0, 1.0, 1.0, 1.0];
        for index in 0..animation.tracks.len() {
            if !animation.tracks[index].visible {
                continue;
            }
            for key in 0..animation.tracks[index].keys.len() {
                let track = &animation.tracks[index];
                let at = track.keys[key];
                let centre = Point::new(map.x(at.time), map.y(track, at.value));
                let hit = Rect::new(centre.x - GRAB * 0.5, centre.y - GRAB * 0.5, GRAB, GRAB);
                let locked = track.locked;
                let color = track.color;
                let caught = ui.interact(&format!("{id}.k.{index}.{key}"), hit);
                if caught.hovered && !locked {
                    ui.set_cursor(CursorIcon::Grab);
                }
                if caught.hovered || caught.active {
                    response.hovered = true;
                }
                // Double clicked: gone. The press that began it has already
                // selected and caught this key, so the grab is left where it
                // is -- it holds off the pan until the button comes up, and
                // matches nothing now that the selection has let go.
                if caught.double_clicked && !locked {
                    animation.tracks[index].remove(key);
                    if state.track == Some(index) {
                        state.key = match state.key {
                            Some(selected) if selected == key => None,
                            // The keys after it have each moved down one.
                            Some(selected) if selected > key => Some(selected - 1),
                            selected => selected,
                        };
                    }
                    response.changed = true;
                    deleted = true;
                    // The keys of this track are re-indexed, so this loop's
                    // are no longer the ones it is counting through.
                    break;
                }
                if caught.active && !locked {
                    if state.grab.is_none() {
                        state.track = Some(index);
                        state.key = Some(key);
                        let pointer = ui.input().pointer.unwrap_or(centre);
                        state.grab = Some(Grab::Key {
                            offset: (
                                at.time - map.time(pointer.x),
                                at.value - map.value(track, pointer.y),
                            ),
                        });
                    }
                    let mine = state.track == Some(index) && state.key == Some(key);
                    if let (Some(Grab::Key { offset }), Some(pointer), true) =
                        (state.grab, ui.input().pointer, mine)
                    {
                        let modifiers = ui.input().modifiers;
                        let track = &mut animation.tracks[index];
                        // Shift holds the time, so a value can be nudged
                        // without the key sliding along the timeline.
                        let at_time = if modifiers.shift {
                            at.time
                        } else {
                            self.snapped(map.time(pointer.x) + offset.0, state)
                                .clamp(0.0, self.duration)
                        };
                        let at_value = if modifiers.alt {
                            at.value
                        } else {
                            map.value(track, pointer.y) + offset.1
                        };
                        if at_time != at.time || at_value != at.value {
                            let landed = track.move_key(key, at_time, at_value);
                            state.key = Some(landed);
                            response.changed = true;
                        }
                    }
                }
                let selected = state.track == Some(index) && state.key == Some(key);
                let radius = if selected || caught.hovered {
                    KEY + 1.5
                } else {
                    KEY
                };
                ui.with_clip(map.rect, |ui| {
                    diamond(ui, centre, radius, color);
                    if selected {
                        diamond(ui, centre, radius * 0.45, white);
                    }
                });
            }
        }
        deleted
    }

    /// The selected key's Bezier tangents, while that is what shapes it.
    fn tangents(
        &self,
        ui: &mut Ui,
        map: &Map<'_>,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        response: &mut CurveEditorResponse,
    ) {
        let id = map.id;
        let Some((index, key)) = state.selected(animation) else {
            return;
        };
        let track = &animation.tracks[index];
        if track.locked {
            return;
        }
        let at = track.keys[key];
        // A key's outgoing tangent shapes the segment after it and its
        // incoming one the segment before, so each is only shown where the
        // segment it belongs to is a Bezier.
        let outgoing = at.interpolation == Interpolation::Bezier && key + 1 < track.keys.len();
        let incoming = key
            .checked_sub(1)
            .is_some_and(|before| track.keys[before].interpolation == Interpolation::Bezier);
        let centre = Point::new(map.x(at.time), map.y(track, at.value));
        let color = track.color;
        for (is_out, drawn) in [(true, outgoing), (false, incoming)] {
            if !drawn {
                continue;
            }
            // Worked out and let go of before anything is dragged into the
            // track, so the tangent's own borrow of it is over by then.
            let end = {
                let track = &animation.tracks[index];
                let segment = track.segment(if is_out { key } else { key - 1 });
                let handle = if is_out {
                    segment.out
                } else {
                    segment.incoming
                };
                Point::new(
                    map.x(at.time + handle.time),
                    map.y(track, at.value + handle.value),
                )
            };
            let hit = Rect::new(end.x - GRAB * 0.5, end.y - GRAB * 0.5, GRAB, GRAB);
            let caught = ui.interact(&format!("{id}.t.{index}.{key}.{is_out}"), hit);
            if caught.hovered {
                ui.set_cursor(CursorIcon::Grab);
                response.hovered = true;
            }
            if caught.active {
                if state.grab.is_none() {
                    state.grab = Some(Grab::Tangent { outgoing: is_out });
                }
                let mine = state.grab == Some(Grab::Tangent { outgoing: is_out });
                if let (true, Some(pointer)) = (mine, ui.input().pointer) {
                    let track = &mut animation.tracks[index];
                    // A tangent only ever reaches towards its own segment;
                    // letting it cross the key would fold the curve over.
                    let reach = map.time(pointer.x) - at.time;
                    let pulled = Handle::new(
                        if is_out {
                            reach.max(1e-4)
                        } else {
                            reach.min(-1e-4)
                        },
                        map.value(track, pointer.y) - at.value,
                    );
                    if is_out {
                        track.keys[key].outgoing = pulled;
                    } else {
                        track.keys[key].incoming = pulled;
                    }
                    response.changed = true;
                }
            }
            ui.with_clip(map.rect, |ui| {
                line(ui, centre, end, 1.0, with_alpha(color, 0.6));
                circle(
                    ui,
                    end,
                    TANGENT + if caught.hovered { 1.5 } else { 0.0 },
                    color,
                );
            });
        }
    }

    /// The playhead down the plot, which can also be dragged from there.
    fn playhead(
        &self,
        ui: &mut Ui,
        map: &Map<'_>,
        state: &mut CurveEditorState,
        time: &mut f32,
        response: &mut CurveEditorResponse,
    ) {
        let (id, rect) = (map.id, map.rect);
        let x = map.x(*time);
        if !(rect.x..=rect.right()).contains(&x) {
            return;
        }
        let hit = Rect::new(x - 4.0, rect.y, 8.0, rect.height);
        let caught = ui.interact(&format!("{id}.playhead"), hit);
        if caught.hovered {
            ui.set_cursor(CursorIcon::ResizeHorizontal);
        }
        if caught.active {
            state.grab = Some(Grab::Scrub);
            if let Some(pointer) = ui.input().pointer {
                *time = self
                    .snapped(map.time(pointer.x), state)
                    .clamp(0.0, self.duration);
                response.scrubbed = true;
            }
        }
        line(
            ui,
            Point::new(map.x(*time), rect.y),
            Point::new(map.x(*time), rect.bottom()),
            1.0,
            ui.theme.palette.accent,
        );
    }

    /// Panning, zooming, adding a key by double click, and letting go.
    ///
    /// `deleted` is a key having taken this frame's double click: the plot
    /// under a diamond is still the curve, and a double click that meant
    /// "take this one away" must not put another in its place.
    fn navigate(
        &self,
        ui: &mut Ui,
        map: &Map<'_>,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        response: &mut CurveEditorResponse,
        deleted: bool,
    ) {
        let (id, rect) = (map.id, map.rect);
        let background = ui.interact(&format!("{id}.pan"), rect);
        if background.hovered {
            response.hovered = true;
        }

        // A double click on empty space keys whichever curve it landed
        // nearest, so a track is shaped without going back to the row.
        if let (true, Some(pointer)) = (background.double_clicked && !deleted, ui.input().pointer) {
            let at = self.snapped(map.time(pointer.x), state);
            if let Some(index) = self.nearest(map, animation, pointer, state) {
                let track = &mut animation.tracks[index];
                let key = track.set_value(at, map.value(track, pointer.y));
                state.track = Some(index);
                state.key = Some(key);
                response.changed = true;
            }
        } else if background.active && state.grab.is_none() {
            if let Some(pointer) = ui.input().pointer {
                state.grab = Some(Grab::Pan {
                    pointer,
                    view: state.view,
                });
            }
        }
        if let (Some(Grab::Pan { pointer, view }), Some(now)) = (state.grab, ui.input().pointer) {
            let moved = (
                (now.x - pointer.x) / rect.width.max(1.0) * view.span(),
                (now.y - pointer.y) / rect.height.max(1.0) * view.band(),
            );
            state.view = CurveView {
                start: view.start - moved.0,
                end: view.end - moved.0,
                // Screen y grows downward and the value axis does not, so
                // dragging down raises the band under the pointer.
                low: view.low + moved.1,
                high: view.high + moved.1,
            };
        }

        // The wheel zooms about the pointer: time on its own, or the values
        // too while a modifier is held.
        let wheel = ui.take_scroll(rect).y;
        if let (true, Some(pointer)) = (wheel != 0.0, ui.input().pointer) {
            let factor = (1.0 + wheel.clamp(-120.0, 120.0) / 400.0).clamp(0.4, 2.5);
            let at = (
                ((pointer.x - rect.x) / rect.width.max(1.0)).clamp(0.0, 1.0),
                1.0 - ((pointer.y - rect.y) / rect.height.max(1.0)).clamp(0.0, 1.0),
            );
            let modifiers = ui.input().modifiers;
            state.view.zoom(
                (
                    if modifiers.shift { 1.0 } else { factor },
                    if modifiers.shift || modifiers.alt {
                        factor
                    } else {
                        1.0
                    },
                ),
                at,
            );
        }

        if !ui.input().down {
            state.grab = None;
        }
    }

    /// The visible track whose curve passes nearest `pointer`, or the
    /// selected one where nothing is near enough to have been aimed at.
    fn nearest(
        &self,
        map: &Map<'_>,
        animation: &Animation,
        pointer: Point,
        state: &CurveEditorState,
    ) -> Option<usize> {
        let at = map.time(pointer.x);
        let mut best: Option<(f32, usize)> = None;
        for (index, track) in animation.tracks.iter().enumerate() {
            if !track.visible || track.locked {
                continue;
            }
            let distance = (map.y(track, track.value_at(at)) - pointer.y).abs();
            if best.is_none_or(|(nearest, _)| distance < nearest) {
                best = Some((distance, index));
            }
        }
        match best {
            Some((distance, index)) if distance <= NEAR_CURVE => Some(index),
            _ => state
                .track
                .filter(|index| animation.tracks.get(*index).is_some_and(|t| !t.locked)),
        }
    }

    /// A view holding every key of every visible track, with a little room
    /// around them.
    fn fitted(&self, animation: &Animation) -> CurveView {
        let mut band: Option<(f32, f32)> = None;
        for track in animation.tracks.iter().filter(|track| track.visible) {
            let span = track_span(track);
            for key in &track.keys {
                let value = (key.value - track.range.0) / span;
                let (low, high) = band.unwrap_or((value, value));
                band = Some((low.min(value), high.max(value)));
            }
        }
        let (low, high) = band.unwrap_or((0.0, 1.0));
        let margin = ((high - low) * 0.12).max(0.04);
        // A little either side of the whole run, so a key on the first or
        // last frame is a diamond rather than half of one on the edge.
        CurveView {
            start: -self.duration * 0.02,
            end: self.duration * 1.02,
            low: low - margin,
            high: high + margin,
        }
    }

    /// The playhead as the toolbar says it: the time, and the frame it is on
    /// where there are frames.
    fn moment(&self, time: f32) -> String {
        if self.fps > 0.0 {
            format!(
                "{time:.2}s  |  frame {}",
                (time * self.fps).round() as i64 + 1
            )
        } else {
            format!("{time:.3}")
        }
    }
}

/// What a track's row gave back: whether a curve changed, and a track the
/// button on its row dropped.
#[derive(Clone, Debug, Default, PartialEq)]
struct Rows {
    changed: bool,
    removed: Option<String>,
}

/// Where a time and a value land in the plot, and back again -- and the id
/// everything drawn inside it claims its share of the pointer with.
struct Map<'a> {
    id: &'a str,
    rect: Rect,
    view: CurveView,
}

impl Map<'_> {
    fn x(&self, time: f32) -> f32 {
        self.rect.x + (time - self.view.start) / self.view.span() * self.rect.width
    }

    fn time(&self, x: f32) -> f32 {
        self.view.start + (x - self.rect.x) / self.rect.width.max(1.0) * self.view.span()
    }

    /// Where a fraction of every track's range lands.
    fn y_normalised(&self, fraction: f32) -> f32 {
        self.rect.bottom() - (fraction - self.view.low) / self.view.band() * self.rect.height
    }

    fn y(&self, track: &Track, value: f32) -> f32 {
        self.y_normalised((value - track.range.0) / track_span(track))
    }

    fn value(&self, track: &Track, y: f32) -> f32 {
        let fraction =
            self.view.low + (self.rect.bottom() - y) / self.rect.height.max(1.0) * self.view.band();
        track.clamp(track.range.0 + fraction * track_span(track))
    }
}

/// How wide a track's range is, never zero.
fn track_span(track: &Track) -> f32 {
    let span = track.range.1 - track.range.0;
    if span.abs() < 1e-9 {
        1.0
    } else {
        span
    }
}

/// A value as its track says it: to its decimals, with its unit.
pub fn shown(track: &Track, value: f32) -> String {
    format!(
        "{:.*}{}",
        track.decimals as usize,
        zeroed(value),
        track.unit
    )
}

/// `value`, with a negative zero made a plain one: a tick a hair below the
/// origin should read `0` and not `-0.00`.
fn zeroed(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

/// A keyframe's diamond.
fn diamond(ui: &mut Ui, centre: Point, radius: f32, color: Color) {
    let points = [
        Point::new(centre.x, centre.y - radius),
        Point::new(centre.x + radius, centre.y),
        Point::new(centre.x, centre.y + radius),
        Point::new(centre.x - radius, centre.y),
    ];
    crate::vector::fill_triangles(
        ui,
        &[
            points[0], points[1], points[2], points[0], points[2], points[3],
        ],
        color,
    );
}

/// A round number near `rough`, so the ruler is labelled 0.5 and 1 rather
/// than 0.4736.
fn nice_step(rough: f32) -> f32 {
    if !(rough.is_finite() && rough > 0.0) {
        return 1.0;
    }
    let magnitude = 10f32.powf(rough.log10().floor());
    let steps = [1.0, 2.0, 5.0, 10.0];
    let normalised = rough / magnitude;
    magnitude
        * steps
            .into_iter()
            .find(|step| normalised <= *step)
            .unwrap_or(10.0)
}

/// A tick's label, to as many decimals as the spacing needs.
fn format_time(value: f32, step: f32) -> String {
    let decimals = if step >= 1.0 {
        0
    } else if step >= 0.1 {
        1
    } else if step >= 0.01 {
        2
    } else {
        3
    };
    format!("{:.decimals$}", zeroed(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;
    use ui_kit_core::animation::Keyframe;
    use ui_kit_core::Point as Vec2;

    fn animation() -> Animation {
        let mut animation = Animation::new();
        animation.add(
            Track::new("camera.distance", "Distance")
                .with_range(0.0, 10.0)
                .with_keys([Keyframe::new(0.0, 2.0), Keyframe::new(4.0, 8.0)]),
        );
        animation
    }

    fn rect() -> Rect {
        Rect::new(0.0, 0.0, 800.0, 220.0)
    }

    fn frame(
        ui: &mut Ui,
        editor: &CurveEditor<'_>,
        input: Input,
        animation: &mut Animation,
        state: &mut CurveEditorState,
        time: &mut f32,
    ) -> CurveEditorResponse {
        ui.begin(input);
        let response = editor.show(ui, "curves", rect(), animation, state, time);
        ui.end();
        response
    }

    #[test]
    fn an_editor_with_nothing_in_it_still_draws() {
        let mut ui = Ui::default();
        let mut animation = Animation::new();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0).fps(25.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        assert!(!ui.commands.is_empty());
    }

    #[test]
    fn a_view_left_at_its_default_is_put_over_the_curves_it_is_handed() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::default();
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        // The whole run, and only the part of the track's range the keys
        // actually use -- a distance keyed between 2 and 8 out of a possible
        // 0 to 10 should not be drawn as a line along the bottom.
        assert!(state.view.start <= 0.0, "{}", state.view.start);
        assert!(state.view.end >= 4.0, "{}", state.view.end);
        assert!(state.view.high - state.view.low < 1.0, "{state:?}");
        assert_eq!(state.track, Some(0), "nothing labels the value axis");
    }

    #[test]
    fn an_empty_editor_is_left_over_the_whole_of_the_time_it_was_given() {
        let mut ui = Ui::default();
        let mut animation = Animation::new();
        let mut state = CurveEditorState::default();
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        assert_eq!(state.view.start, 0.0);
        assert_eq!(state.view.end, 4.0);
    }

    #[test]
    fn a_key_dragged_across_the_plot_takes_its_curve_with_it() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.snap = false;
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        // Where the second key is drawn, which the drag has to start from.
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        let at = ui
            .widget_rect("curves.k.0.1")
            .expect("the second key is drawn");
        let centre = Vec2::new(at.x + at.width * 0.5, at.y + at.height * 0.5);
        let was = animation.tracks[0].keys[1].value;
        for (pointer, down) in [
            (centre, false),
            (centre, true),
            (Vec2::new(centre.x, centre.y + 40.0), true),
        ] {
            frame(
                &mut ui,
                &editor,
                Input {
                    pointer: Some(pointer),
                    down,
                    ..Default::default()
                },
                &mut animation,
                &mut state,
                &mut time,
            );
        }
        // Dragging down lowers the value, and the key is the selected one.
        assert!(animation.tracks[0].keys[1].value < was);
        assert_eq!(state.track, Some(0));
        assert_eq!(state.key, Some(1));
    }

    #[test]
    fn dragging_the_ruler_moves_the_playhead_and_snaps_it_to_a_frame() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0).fps(25.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        let ruler = ui.widget_rect("curves.scrub").expect("the ruler is drawn");
        let at = Vec2::new(ruler.x + ruler.width * 0.5, ruler.y + ruler.height * 0.5);
        let mut scrubbed = false;
        for (pointer, down) in [(at, false), (at, true)] {
            scrubbed |= frame(
                &mut ui,
                &editor,
                Input {
                    pointer: Some(pointer),
                    down,
                    ..Default::default()
                },
                &mut animation,
                &mut state,
                &mut time,
            )
            .scrubbed;
        }
        assert!(scrubbed);
        assert!(time > 0.0 && time <= 4.0, "{time}");
        assert!(((time * 25.0).round() - time * 25.0).abs() < 1e-3, "{time}");
    }

    #[test]
    fn the_row_button_keys_the_track_at_the_playhead_and_takes_it_back() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 2.0;
        let editor = CurveEditor::new(4.0).fps(25.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        let button = ui.widget_rect("curves.key.0").expect("the row is drawn");
        let at = Vec2::new(
            button.x + button.width * 0.5,
            button.y + button.height * 0.5,
        );
        let press = |ui: &mut Ui, animation: &mut Animation, state: &mut CurveEditorState| {
            for down in [false, true, false] {
                frame(
                    ui,
                    &editor,
                    Input {
                        pointer: Some(at),
                        down,
                        ..Default::default()
                    },
                    animation,
                    state,
                    &mut 2.0,
                );
            }
        };
        press(&mut ui, &mut animation, &mut state);
        assert_eq!(animation.tracks[0].keys.len(), 3);
        // Keying where the curve already was leaves it exactly as it was.
        assert!((animation.tracks[0].value_at(1.0) - 3.5).abs() < 1.0);
        press(&mut ui, &mut animation, &mut state);
        assert_eq!(animation.tracks[0].keys.len(), 2);
    }

    #[test]
    fn a_track_is_dropped_by_the_button_on_its_row() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        let button = ui.widget_rect("curves.drop.0").expect("the row is drawn");
        let at = Vec2::new(
            button.x + button.width * 0.5,
            button.y + button.height * 0.5,
        );
        let mut removed = None;
        for down in [false, true, false] {
            removed = removed.or(frame(
                &mut ui,
                &editor,
                Input {
                    pointer: Some(at),
                    down,
                    ..Default::default()
                },
                &mut animation,
                &mut state,
                &mut time,
            )
            .removed);
        }
        assert_eq!(removed.as_deref(), Some("camera.distance"));
        assert!(animation.is_empty());
    }

    #[test]
    fn double_clicking_a_curve_adds_a_key_to_it() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0).fps(25.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        // Half way along the plot, on the curve the track draws there.
        let plot = ui.widget_rect("curves.pan").expect("the plot is drawn");
        let first = ui.widget_rect("curves.k.0.0").expect("the first key");
        let last = ui.widget_rect("curves.k.0.1").expect("the last key");
        let at = Vec2::new(
            plot.x + plot.width * 0.5,
            (first.y + last.y) * 0.5 + first.height * 0.5,
        );
        let mut clock = 0.0;
        for _ in 0..2 {
            for down in [true, false] {
                clock += 0.05;
                frame(
                    &mut ui,
                    &editor,
                    Input {
                        pointer: Some(at),
                        down,
                        time: Some(clock),
                        ..Default::default()
                    },
                    &mut animation,
                    &mut state,
                    &mut time,
                );
            }
        }
        assert_eq!(
            animation.tracks[0].keys.len(),
            3,
            "{:?}",
            animation.tracks[0].keys
        );
        assert_eq!(state.key, Some(1), "the new key is the selected one");
    }

    /// Double clicking a diamond takes that key away, and does not leave
    /// another behind it: the same click is over the curve, which on empty
    /// plot would add one.
    #[test]
    fn double_clicking_a_key_deletes_it() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0).fps(25.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        assert_eq!(animation.tracks[0].keys.len(), 2);
        let first = ui.widget_rect("curves.k.0.0").expect("the first key");
        let at = Vec2::new(first.x + first.width * 0.5, first.y + first.height * 0.5);
        let mut clock = 0.0;
        let mut response = None;
        for _ in 0..2 {
            for down in [true, false] {
                clock += 0.05;
                response = Some(frame(
                    &mut ui,
                    &editor,
                    Input {
                        pointer: Some(at),
                        down,
                        time: Some(clock),
                        ..Default::default()
                    },
                    &mut animation,
                    &mut state,
                    &mut time,
                ));
            }
        }
        assert!(response.expect("a frame was drawn").changed);
        assert_eq!(
            animation.tracks[0].keys.len(),
            1,
            "{:?}",
            animation.tracks[0].keys
        );
        assert_eq!(state.key, None, "the key that went is not still selected");
    }

    #[test]
    fn delete_takes_the_selected_key_away() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.track = Some(0);
        state.key = Some(1);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        let response = frame(
            &mut ui,
            &editor,
            Input {
                // Over the editor, which is what the key acts on.
                pointer: Some(Vec2::new(400.0, 120.0)),
                keys: vec![crate::Key::Delete],
                ..Default::default()
            },
            &mut animation,
            &mut state,
            &mut time,
        );
        assert!(response.changed);
        assert_eq!(animation.tracks[0].keys.len(), 1);
        assert_eq!(state.key, None);
    }

    /// Focus a field, select what is in it, and type `text` into it.
    fn type_into(
        ui: &mut Ui,
        editor: &CurveEditor<'_>,
        field: &str,
        text: &str,
        animation: &mut Animation,
        state: &mut CurveEditorState,
    ) {
        let mut time = 0.0;
        frame(ui, editor, Input::default(), animation, state, &mut time);
        ui.request_focus(field);
        frame(
            ui,
            editor,
            Input {
                select_all: true,
                text: text.into(),
                ..Default::default()
            },
            animation,
            state,
            &mut time,
        );
    }

    #[test]
    fn a_value_typed_into_the_toolbar_is_the_selected_key_s() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.track = Some(0);
        state.key = Some(1);
        let editor = CurveEditor::new(4.0).fps(25.0);
        type_into(
            &mut ui,
            &editor,
            "curves.selected.value",
            "3.5",
            &mut animation,
            &mut state,
        );
        assert_eq!(animation.tracks[0].keys[1].value, 3.5);
        // The other key, and the time of the one typed into, are untouched.
        assert_eq!(animation.tracks[0].keys[0].value, 2.0);
        assert_eq!(animation.tracks[0].keys[1].time, 4.0);
    }

    #[test]
    fn a_typed_value_is_held_to_the_track_s_range() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.track = Some(0);
        state.key = Some(1);
        let editor = CurveEditor::new(4.0);
        type_into(
            &mut ui,
            &editor,
            "curves.selected.value",
            "400",
            &mut animation,
            &mut state,
        );
        assert_eq!(animation.tracks[0].keys[1].value, 10.0, "the range's top");
    }

    #[test]
    fn a_key_typed_past_its_neighbour_slides_past_it_and_stays_selected() {
        let mut ui = Ui::default();
        let mut animation = Animation::new();
        animation.add(
            Track::new("camera.distance", "Distance")
                .with_range(0.0, 10.0)
                .with_keys([
                    Keyframe::new(0.0, 2.0),
                    Keyframe::new(1.0, 5.0),
                    Keyframe::new(2.0, 8.0),
                ]),
        );
        let mut state = CurveEditorState::new(4.0);
        // The middle key, typed to a time past the last one.
        state.track = Some(0);
        state.key = Some(1);
        let editor = CurveEditor::new(4.0).fps(25.0);
        type_into(
            &mut ui,
            &editor,
            "curves.selected.time",
            "3",
            &mut animation,
            &mut state,
        );
        let times: Vec<f32> = animation.tracks[0]
            .keys
            .iter()
            .map(|key| key.time)
            .collect();
        assert_eq!(times, vec![0.0, 2.0, 3.0]);
        // The key that was typed is still the selected one, wherever the
        // sorting left it, and it took its value with it.
        assert_eq!(state.key, Some(2));
        assert_eq!(animation.tracks[0].keys[2].value, 5.0);
    }

    #[test]
    fn a_toolbar_too_narrow_for_both_fields_keeps_the_value_one() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.track = Some(0);
        state.key = Some(1);
        let editor = CurveEditor::new(4.0);
        ui.begin(Input::default());
        editor.show(
            &mut ui,
            "curves",
            Rect::new(0.0, 0.0, 680.0, 220.0),
            &mut animation,
            &mut state,
            &mut 0.0,
        );
        ui.end();
        assert!(ui.widget_rect("curves.selected.value").is_some());
        assert!(ui.widget_rect("curves.selected.time").is_none());
    }

    #[test]
    fn backspace_in_a_field_takes_a_digit_and_not_the_key() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        state.track = Some(0);
        state.key = Some(1);
        let editor = CurveEditor::new(4.0);
        let mut time = 0.0;
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        ui.request_focus("curves.selected.value");
        frame(
            &mut ui,
            &editor,
            Input {
                // Over the editor, where Delete would otherwise act on the
                // selection, with the field holding the keyboard.
                pointer: Some(Vec2::new(400.0, 120.0)),
                backspace: true,
                keys: vec![crate::Key::Backspace],
                ..Default::default()
            },
            &mut animation,
            &mut state,
            &mut time,
        );
        assert_eq!(animation.tracks[0].keys.len(), 2, "the key is still there");
        assert_eq!(state.key, Some(1));
    }

    #[test]
    fn there_are_no_fields_to_type_into_until_a_key_is_selected() {
        let mut ui = Ui::default();
        let mut animation = animation();
        let mut state = CurveEditorState::new(4.0);
        let mut time = 0.0;
        let editor = CurveEditor::new(4.0);
        frame(
            &mut ui,
            &editor,
            Input::default(),
            &mut animation,
            &mut state,
            &mut time,
        );
        assert_eq!(state.key, None);
        assert!(ui.widget_rect("curves.selected.value").is_none());
    }

    #[test]
    fn an_option_becomes_a_track_keyed_where_the_playhead_is() {
        let option = TrackOption::new("camera.fov", "Field of view", 50.0)
            .range(5.0, 150.0)
            .unit("\u{b0}")
            .decimals(1);
        let track = option.track(1.5, INFO);
        assert_eq!(track.id, "camera.fov");
        assert_eq!(track.keys.len(), 1);
        assert_eq!(track.keys[0].time, 1.5);
        assert_eq!(track.value_at(0.0), 50.0);
        assert_eq!(track.value_at(9.0), 50.0);
    }

    #[test]
    fn the_ruler_is_labelled_in_round_numbers() {
        assert_eq!(nice_step(0.37), 0.5);
        assert_eq!(nice_step(1.3), 2.0);
        assert_eq!(nice_step(7.0), 10.0);
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(format_time(1.5, 0.5), "1.5");
        assert_eq!(format_time(2.0, 1.0), "2");
        // A tick a hair below the origin reads as the origin, not as -0.
        assert_eq!(format_time(-0.0, 0.5), "0.0");
    }

    #[test]
    fn a_track_of_no_width_at_all_still_maps_without_dividing_by_nothing() {
        let track = Track::new("t", "T").with_range(3.0, 3.0);
        let map = Map {
            id: "curves",
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            view: CurveView::whole(1.0),
        };
        assert!(map.y(&track, 3.0).is_finite());
        assert!(map.value(&track, 50.0).is_finite());
    }
}
