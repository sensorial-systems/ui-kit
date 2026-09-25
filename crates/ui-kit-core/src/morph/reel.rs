//! The reel: twelve UI states one shape morphs through in seven bars.
//!
//! Everything is laid out in the shape's own units ("local": a button is
//! 220 wide) around the shell's centre, then a camera zooms so each state
//! fills the stage. Every moving number is a [`SpringTrack`] built once from
//! the beat grid, and a cursor path drives what the pointer changes: while a
//! drag is held the value is computed from where the cursor is, and on
//! release it springs back from there.

use super::{arc, fade, hex, mix, trim, Item, Layer, Scene, Stroke};
use crate::motion::{rubber_band, BeatGrid, Blend, Spring, SpringTrack};
use crate::{Color, Point, Rect, TextAlign};
use serde::{Deserialize, Serialize};

/// The stage is a square this many units across: one unit per pixel of the
/// 1440 × 1440 render.
pub const STAGE: f32 = 1440.0;
/// Seven bars of four.
pub const REEL_BEATS: f32 = 28.0;

const CANVAS: Color = hex(0xE8E6E1);
const INK: Color = hex(0x0B0B0C);
const PAPER: Color = hex(0xFFFFFF);
const MUTED: Color = hex(0x77767B);
const TRACK: Color = hex(0xE7E5E1);
const SOFT: Color = hex(0xF1EFEC);
const OFF: Color = hex(0xC8C4BE);
const DISC: Color = hex(0x2B2B2E);
/// Every icon in the reel is drawn with this stroke, in local units.
const ICON_STROKE: f32 = 2.0;
/// How big the cursor is on the stage; it is not zoomed with the camera.
const CURSOR_SCALE: f32 = 2.6;

/// The states the shape passes through, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReelState {
    Button,
    Loader,
    Check,
    Island,
    Player,
    Volume,
    Toggle,
    Tabs,
    Chart,
    Search,
    Palette,
    Toast,
}

impl ReelState {
    pub const ALL: [ReelState; 12] = [
        ReelState::Button,
        ReelState::Loader,
        ReelState::Check,
        ReelState::Island,
        ReelState::Player,
        ReelState::Volume,
        ReelState::Toggle,
        ReelState::Tabs,
        ReelState::Chart,
        ReelState::Search,
        ReelState::Palette,
        ReelState::Toast,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ReelState::Button => "Button",
            ReelState::Loader => "Loader",
            ReelState::Check => "Check",
            ReelState::Island => "Dynamic island",
            ReelState::Player => "Music player",
            ReelState::Volume => "Volume slider",
            ReelState::Toggle => "Toggle",
            ReelState::Tabs => "Tabs",
            ReelState::Chart => "Chart",
            ReelState::Search => "Command-K",
            ReelState::Palette => "Command palette",
            ReelState::Toast => "Toast",
        }
    }
}

/// The beat each state begins on. The button spans the loop: it starts on
/// the last beat and is still there on the first.
const TIMELINE: [(f32, ReelState); 12] = [
    (1.0, ReelState::Loader),
    (3.0, ReelState::Check),
    (4.0, ReelState::Island),
    (6.0, ReelState::Player),
    (11.0, ReelState::Volume),
    (15.0, ReelState::Toggle),
    (17.0, ReelState::Tabs),
    (19.0, ReelState::Chart),
    (23.0, ReelState::Search),
    (24.0, ReelState::Palette),
    (26.0, ReelState::Toast),
    (27.0, ReelState::Button),
];

/// What happens on each beat, for a beat sheet or a caption.
const BEATS: [&str; 28] = [
    "Cursor on the button",
    "Click: button to loader",
    "Loader sweeps",
    "Ring closes: check",
    "Check to dynamic island",
    "Island opens, title slides in",
    "Island opens into the player",
    "Click: play morphs to pause",
    "Press the progress bar",
    "Scrub",
    "Release",
    "Player to volume slider",
    "Press the knob, drag",
    "Past max: it stretches",
    "Release, springs back",
    "Knob becomes a toggle",
    "Click: toggle flips on",
    "Knob becomes the tab indicator",
    "Click: Day",
    "Click: Week opens the chart",
    "Chart draws itself",
    "Hover: tooltip",
    "Tooltip follows",
    "Chart collapses into Command-K",
    "Click: palette opens, type",
    "Filter to one",
    "Enter: toast",
    "Toast back to the button",
];

/// A UI sound, placed where the thing it belongs to happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sound {
    /// A pointer pressing down.
    Press,
    /// A pointer letting go.
    Release,
    /// A full click, down and up.
    Click,
    Toggle,
    Success,
    Type,
    Enter,
    Pop,
}

impl Sound {
    pub fn name(self) -> &'static str {
        match self {
            Sound::Press => "press",
            Sound::Release => "release",
            Sound::Click => "click",
            Sound::Toggle => "toggle",
            Sound::Success => "success",
            Sound::Type => "type",
            Sound::Enter => "enter",
            Sound::Pop => "pop",
        }
    }
}

/// A sound at a time: where its audible peak should land.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    pub time: f32,
    pub beat: f32,
    pub sound: Sound,
}

/// How the shell looks when it has settled into a state.
#[derive(Clone, Copy, Debug)]
struct Look {
    center: Point,
    width: f32,
    height: f32,
    radius: f32,
    fill: Color,
    zoom: f32,
}

impl Look {
    fn new(width: f32, height: f32, fill: Color, zoom: f32) -> Self {
        Self {
            center: Point::new(0.0, 0.0),
            width,
            height,
            radius: width.min(height) * 0.5,
            fill,
            zoom,
        }
    }

    fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    fn center(mut self, x: f32, y: f32) -> Self {
        self.center = Point::new(x, y);
        self
    }

    /// Where a local point lands on the stage once the camera has settled.
    fn stage(&self, p: Point) -> Point {
        Point::new(
            STAGE * 0.5 + (p.x - self.center.x) * self.zoom,
            STAGE * 0.5 + (p.y - self.center.y) * self.zoom,
        )
    }
}

// Layout constants, in local units.
const PLAYER: (f32, f32) = (340.0, 200.0);
const SCRUB_Y: f32 = 8.0;
const SCRUB_X: (f32, f32) = (-150.0, 150.0);
const PLAY_Y: f32 = 62.0;
const SONG_SECONDS: f32 = 228.0;
const VOLUME: (f32, f32) = (300.0, 60.0);
const VOLUME_X: (f32, f32) = (-98.0, 124.0);
const TOGGLE: (f32, f32) = (96.0, 56.0);
const TABS: (f32, f32) = (336.0, 56.0);
const TAB_NAMES: [&str; 3] = ["Day", "Week", "Month"];
const CHART: (f32, f32) = (420.0, 300.0);
const PLOT: Rect = Rect {
    x: -190.0,
    y: -20.0,
    width: 380.0,
    height: 120.0,
};
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const PLAYS: [f32; 7] = [1.2, 1.9, 1.5, 2.6, 2.1, 3.1, 2.7];
const SEARCH: (f32, f32) = (260.0, 52.0);
const PALETTE_WIDTH: f32 = 420.0;
const PALETTE_TOP: f32 = -132.0;
const PALETTE_HEADER: f32 = 52.0;
const ROW: f32 = 40.0;
const COMMANDS: [&str; 5] = [
    "Play next",
    "Add to queue",
    "Shuffle library",
    "Share track",
    "Show lyrics",
];
const TOAST: (f32, f32) = (300.0, 60.0);

/// The query typed into the palette, and the beat each version appears on.
const TYPING: [(f32, &str); 4] = [(24.0, ""), (24.5, "s"), (24.75, "sh"), (25.0, "sha")];

/// What the one small shape that travels with the shell -- scrub knob,
/// volume knob, toggle knob, tab indicator, keycap, highlighted row, toast
/// icon -- is being, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Puck {
    BeforeScrub,
    Scrub,
    Volume,
    Toggle,
    Tab,
    Keycap,
    Row,
    ToastIcon,
    AfterToast,
}
const PUCKS: usize = 9;

/// Content that fades and blurs in and out with its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Content {
    Button,
    Loader,
    Check,
    Island,
    IslandTitle,
    Player,
    Volume,
    Tabs,
    Chart,
    Tooltip,
    Search,
    Palette,
    Toast,
}
const CONTENTS: usize = 13;

/// A pointer press: down at `down`, up at `up`, in seconds.
#[derive(Clone, Copy, Debug)]
struct Press {
    down: f32,
    up: f32,
}

struct Tracks {
    // Shell and camera.
    x: SpringTrack,
    y: SpringTrack,
    width: SpringTrack,
    height: SpringTrack,
    radius: SpringTrack,
    fill: [SpringTrack; 3],
    zoom: SpringTrack,
    cam_x: SpringTrack,
    cam_y: SpringTrack,
    squash: SpringTrack,
    hover: SpringTrack,
    // Cursor.
    cursor_x: SpringTrack,
    cursor_y: SpringTrack,
    cursor_down: SpringTrack,
    // Content.
    presence: Vec<SpringTrack>,
    sweep: SpringTrack,
    check: SpringTrack,
    playing: SpringTrack,
    knob: SpringTrack,
    stretch: SpringTrack,
    toggle: [SpringTrack; 2],
    tab: [SpringTrack; 2],
    chart_open: SpringTrack,
    chart_draw: SpringTrack,
    rows: Vec<(SpringTrack, SpringTrack)>,
    puck: Blend,
}

/// The choreography, ready to be seeked.
pub struct Reel {
    grid: BeatGrid,
    accent: Option<Color>,
    period: f32,
    tracks: Tracks,
    /// Scrub and volume drags: (press, released value).
    scrub: (Press, f32),
    volume: (Press, f32),
    /// The chart line, smoothed, in local units.
    curve: Vec<Point>,
    cues: Vec<Cue>,
}

impl Default for Reel {
    fn default() -> Self {
        Self::new(BeatGrid::default(), None)
    }
}

impl Reel {
    /// The reel on `grid`, in black and white or with one accent colour.
    pub fn new(grid: BeatGrid, accent: Option<Color>) -> Self {
        Builder::new(grid, accent).build()
    }

    pub fn grid(&self) -> BeatGrid {
        self.grid
    }

    pub fn accent(&self) -> Option<Color> {
        self.accent
    }

    /// One lap, in seconds.
    pub fn duration(&self) -> f32 {
        self.period
    }

    /// Where each UI sound belongs, in seconds from the first downbeat.
    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    /// The beat each state begins on, in order.
    pub fn timeline() -> impl Iterator<Item = (f32, ReelState)> {
        TIMELINE.into_iter()
    }

    /// What happens on `beat` (0..28).
    pub fn beat_caption(beat: usize) -> &'static str {
        BEATS[beat % BEATS.len()]
    }

    /// The state the shell is in (or heading into) at `time`.
    pub fn state_at(&self, time: f32) -> ReelState {
        let beat = self.beat_of(time.rem_euclid(self.period));
        TIMELINE
            .iter()
            .rev()
            .find(|(start, _)| beat >= *start)
            .map_or(ReelState::Button, |(_, state)| *state)
    }

    /// The beat at `t` seconds into the reel.
    fn beat_of(&self, t: f32) -> f32 {
        t / self.grid.beat_length()
    }

    fn secs(&self, beat: f32) -> f32 {
        beat * self.grid.beat_length()
    }

    /// Where the cursor is on the stage at `t`.
    pub fn cursor(&self, time: f32) -> Point {
        let t = time.rem_euclid(self.period);
        Point::new(self.tracks.cursor_x.value(t), self.tracks.cursor_y.value(t))
    }

    /// The cursor's speed on the stage at `t`, units per second.
    pub fn cursor_velocity(&self, time: f32) -> Point {
        let t = time.rem_euclid(self.period);
        Point::new(
            self.tracks.cursor_x.velocity(t),
            self.tracks.cursor_y.velocity(t),
        )
    }

    fn camera(&self, t: f32) -> (f32, Point) {
        let k = &self.tracks;
        (
            k.zoom.value(t) * (1.0 - 0.035 * k.squash.value(t)),
            Point::new(k.cam_x.value(t), k.cam_y.value(t)),
        )
    }

    /// The cursor in local units, through the camera as it is at `t`.
    fn cursor_local(&self, t: f32) -> Point {
        let (zoom, cam) = self.camera(t);
        let c = self.cursor(t);
        Point::new(
            cam.x + (c.x - STAGE * 0.5) / zoom,
            cam.y + (c.y - STAGE * 0.5) / zoom,
        )
    }

    fn held(press: Press, t: f32) -> bool {
        t >= press.down && t < press.up
    }

    /// The song position, 0..1, on the progress bar.
    fn progress(&self, t: f32) -> f32 {
        const START: f32 = 0.22;
        const RATE: f32 = 0.012;
        let play = self.secs(7.0);
        let (press, released) = self.scrub;
        if t < play {
            START
        } else if t < press.down {
            START + (t - play) * RATE
        } else if Self::held(press, t) {
            scrub_fraction(self.cursor_local(t).x)
        } else {
            released + (t - press.up) * RATE
        }
    }

    /// The volume 0..1 and how far the slider is stretched past its end.
    fn volume(&self, t: f32) -> (f32, f32) {
        let (press, released) = self.volume;
        if t < press.down {
            (0.45, 0.0)
        } else if Self::held(press, t) {
            volume_drag(self.cursor_local(t).x)
        } else {
            (released, self.tracks.stretch.value(t))
        }
    }

    fn accent_or(&self, color: Color) -> Color {
        self.accent.unwrap_or(color)
    }

    /// What the stage looks like at `time` seconds from the first downbeat.
    /// Times past the end wrap around: the reel loops.
    pub fn seek(&self, time: f32) -> Scene {
        let t = time.rem_euclid(self.period);
        let k = &self.tracks;
        let (zoom, cam) = self.camera(t);
        let (_, stretch) = self.volume(t);
        let shell = Rect::new(
            k.x.value(t) - k.width.value(t) * 0.5,
            k.y.value(t) - k.height.value(t) * 0.5,
            k.width.value(t).max(0.0) + stretch,
            (k.height.value(t) - stretch * 0.12).max(0.0),
        );
        let shell = Rect::new(shell.x, shell.y + stretch * 0.06, shell.width, shell.height);
        let radius = k
            .radius
            .value(t)
            .min(shell.width.min(shell.height) * 0.5)
            .max(0.0);
        let fill = [
            k.fill[0].value(t).clamp(0.0, 1.0),
            k.fill[1].value(t).clamp(0.0, 1.0),
            k.fill[2].value(t).clamp(0.0, 1.0),
            1.0,
        ];

        let mut body = Layer::new(1.0, 0.0);
        body.push(Item::Shadow {
            rect: Rect::new(shell.x, shell.y + 6.0, shell.width, shell.height),
            radius,
            blur: 22.0,
            color: [0.0, 0.0, 0.0, 0.07],
        });
        body.push(Item::Rect {
            rect: shell,
            radius,
            fill,
            stroke: None,
        });

        let mut unders = vec![];
        let mut overs = vec![];
        for (index, content) in CONTENT_ORDER.iter().enumerate() {
            let presence = k.presence[index].value(t).clamp(0.0, 1.0);
            if presence < 0.003 {
                continue;
            }
            let (under, over) = self.content(*content, t, shell, presence);
            let blur = (1.0 - presence) * 5.0;
            if !under.is_empty() {
                unders.push(Layer {
                    opacity: presence,
                    blur,
                    items: under,
                });
            }
            if !over.is_empty() {
                overs.push(Layer {
                    opacity: presence,
                    blur,
                    items: over,
                });
            }
        }

        let mut local = vec![body];
        local.extend(unders);
        local.extend(self.puck(t, shell));
        local.extend(overs);

        // Through the camera: local units around the camera's centre, zoomed.
        let offset = Point::new(STAGE * 0.5 - cam.x * zoom, STAGE * 0.5 - cam.y * zoom);
        let mut layers: Vec<Layer> = local
            .into_iter()
            .map(|layer| Layer {
                opacity: layer.opacity,
                // Blur is kept in stage units so it reads the same at any zoom.
                blur: layer.blur * 2.4,
                items: layer
                    .items
                    .iter()
                    .map(|item| item.transformed(zoom, offset))
                    .collect(),
            })
            .collect();
        layers.extend(self.cursor_layers(t));
        Scene {
            size: STAGE,
            background: CANVAS,
            layers,
        }
    }

    fn cursor_layers(&self, t: f32) -> [Layer; 2] {
        let tip = self.cursor(t);
        let scale = CURSOR_SCALE * (1.0 - 0.12 * self.tracks.cursor_down.value(t));
        let arrow: Vec<Point> = [
            (0.0, 0.0),
            (0.0, 16.5),
            (3.9, 12.8),
            (6.6, 19.0),
            (9.2, 17.9),
            (6.6, 11.9),
            (11.9, 11.9),
        ]
        .iter()
        .map(|(x, y)| Point::new(tip.x + x * scale, tip.y + y * scale))
        .collect();
        let shadow: Vec<Point> = arrow
            .iter()
            .map(|p| Point::new(p.x + 1.0 * scale, p.y + 1.6 * scale))
            .collect();
        [
            Layer {
                opacity: 1.0,
                blur: 2.0 * scale,
                items: vec![Item::Polygon {
                    points: shadow,
                    fill: [0.0, 0.0, 0.0, 0.22],
                    stroke: None,
                }],
            },
            Layer {
                opacity: 1.0,
                blur: 0.0,
                items: vec![Item::Polygon {
                    points: arrow,
                    fill: INK,
                    stroke: Some(Stroke {
                        width: 1.3 * scale,
                        color: PAPER,
                    }),
                }],
            },
        ]
    }

    /// The traveling shape, a mix of what it is being in each state.
    fn puck(&self, t: f32, shell: Rect) -> Option<Layer> {
        let weights = self.tracks.puck.weights(t);
        let hidden = weights[Puck::BeforeScrub as usize] + weights[Puck::AfterToast as usize];
        if hidden > 0.998 {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1, mut radius) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let mut color = [0.0; 4];
        let mut shadow = 0.0;
        for (index, weight) in weights.iter().enumerate() {
            if weight.abs() < 1e-4 {
                continue;
            }
            let (rect, r, c) = self.puck_as(PUCK_ORDER[index], t, shell);
            x0 += rect.x * weight;
            y0 += rect.y * weight;
            x1 += (rect.x + rect.width) * weight;
            y1 += (rect.y + rect.height) * weight;
            radius += r * weight;
            for (channel, value) in color.iter_mut().zip(c) {
                *channel += value * weight;
            }
            if PUCK_ORDER[index] == Puck::Toggle {
                shadow += weight;
            }
        }
        let rect = Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0));
        let radius = radius.min(rect.width.min(rect.height) * 0.5).max(0.0);
        let mut layer = Layer::new(1.0, 0.0);
        if shadow > 0.01 {
            layer.push(Item::Shadow {
                rect: Rect::new(rect.x, rect.y + 1.5, rect.width, rect.height),
                radius,
                blur: 5.0,
                color: [0.0, 0.0, 0.0, 0.16 * shadow],
            });
        }
        layer.push(Item::Rect {
            rect,
            radius,
            fill: color,
            stroke: None,
        });
        Some(layer)
    }

    fn puck_as(&self, puck: Puck, t: f32, shell: Rect) -> (Rect, f32, Color) {
        let k = &self.tracks;
        let circle = |c: Point, d: f32| Rect::new(c.x - d * 0.5, c.y - d * 0.5, d, d);
        match puck {
            Puck::BeforeScrub => (circle(Point::new(scrub_x(0.22), SCRUB_Y), 0.0), 0.0, INK),
            Puck::Scrub => {
                let d = k.knob.value(t);
                (
                    circle(Point::new(scrub_x(self.progress(t)), SCRUB_Y), d),
                    d * 0.5,
                    INK,
                )
            }
            Puck::Volume => {
                let (v, stretch) = self.volume(t);
                let x = VOLUME_X.0 + v * (VOLUME_X.1 - VOLUME_X.0) + stretch;
                let d = 22.0;
                (
                    circle(Point::new(x, shell.y + shell.height * 0.5), d),
                    d * 0.5,
                    INK,
                )
            }
            Puck::Toggle => {
                let travel = TOGGLE.0 - TOGGLE.1;
                let left = -TOGGLE.0 * 0.5 + 6.0 + k.toggle[0].value(t) * travel;
                let right = -TOGGLE.0 * 0.5 + 6.0 + 44.0 + k.toggle[1].value(t) * travel;
                (Rect::new(left, -22.0, right - left, 44.0), 22.0, PAPER)
            }
            Puck::Tab => {
                let row = self.tab_row(t);
                let cell = row.width / 3.0;
                let left = row.x + k.tab[0].value(t) * cell;
                let right = row.x + (k.tab[1].value(t) + 1.0) * cell;
                (
                    Rect::new(left, row.y, right - left, row.height),
                    row.height * 0.5,
                    self.accent_or(INK),
                )
            }
            Puck::Keycap => (
                Rect::new(SEARCH.0 * 0.5 - 12.0 - 40.0, -13.0, 40.0, 26.0),
                7.0,
                SOFT,
            ),
            Puck::Row => {
                let top = PALETTE_TOP + PALETTE_HEADER + 6.0;
                (
                    Rect::new(
                        -PALETTE_WIDTH * 0.5 + 8.0,
                        top + 2.0,
                        PALETTE_WIDTH - 16.0,
                        ROW - 4.0,
                    ),
                    10.0,
                    SOFT,
                )
            }
            Puck::ToastIcon => (
                circle(Point::new(-TOAST.0 * 0.5 + 30.0, 0.0), 30.0),
                15.0,
                self.accent_or(PAPER),
            ),
            Puck::AfterToast => (
                circle(Point::new(-TOAST.0 * 0.5 + 30.0, 0.0), 0.0),
                0.0,
                PAPER,
            ),
        }
    }

    /// The tab strip: filling the tabs bar, then moving up into the chart's
    /// header as the chart opens.
    fn tab_row(&self, t: f32) -> Rect {
        let open = self.tracks.chart_open.value(t);
        let bar = Rect::new(
            -TABS.0 * 0.5 + 6.0,
            -TABS.1 * 0.5 + 6.0,
            TABS.0 - 12.0,
            TABS.1 - 12.0,
        );
        let header = Rect::new(
            -CHART.0 * 0.5 + 16.0,
            -CHART.1 * 0.5 + 16.0,
            CHART.0 - 32.0,
            40.0,
        );
        Rect::new(
            bar.x + (header.x - bar.x) * open,
            bar.y + (header.y - bar.y) * open,
            bar.width + (header.width - bar.width) * open,
            bar.height + (header.height - bar.height) * open,
        )
    }

    /// A state's content: what the traveling shape rides on, and what is
    /// drawn over it.
    fn content(
        &self,
        content: Content,
        t: f32,
        shell: Rect,
        presence: f32,
    ) -> (Vec<Item>, Vec<Item>) {
        let mut under = vec![];
        let mut over = vec![];
        let k = &self.tracks;
        match content {
            Content::Button => {
                let hover = k.hover.value(t);
                over.push(Item::Rect {
                    rect: shell,
                    radius: shell.height * 0.5,
                    fill: [1.0, 1.0, 1.0, 0.1 * hover],
                    stroke: None,
                });
                over.push(text(
                    center(shell),
                    "Listen now",
                    16.0,
                    560,
                    PAPER,
                    TextAlign::Center,
                ));
            }
            Content::Loader => {
                let c = center(shell);
                over.push(path(
                    arc(c, 13.0, 0.0, std::f32::consts::TAU),
                    fade(PAPER, 0.2),
                ));
                let spin = t * std::f32::consts::TAU * 1.15;
                let sweep = k.sweep.value(t) * std::f32::consts::TAU;
                over.push(path(arc(c, 13.0, spin - sweep, sweep), PAPER));
            }
            Content::Check => {
                let c = center(shell);
                let check = [
                    Point::new(c.x - 7.5, c.y + 0.5),
                    Point::new(c.x - 2.5, c.y + 5.5),
                    Point::new(c.x + 7.5, c.y - 5.5),
                ];
                let drawn = trim(&check, k.check.value(t));
                if drawn.len() > 1 {
                    over.push(path(drawn, PAPER));
                }
            }
            Content::Island => {
                let disc = Point::new(shell.x + 22.0, center(shell).y);
                over.push(Item::Rect {
                    rect: Rect::new(disc.x - 14.0, disc.y - 14.0, 28.0, 28.0),
                    radius: 14.0,
                    fill: DISC,
                    stroke: None,
                });
                over.push(Item::Rect {
                    rect: Rect::new(disc.x - 3.0, disc.y - 3.0, 6.0, 6.0),
                    radius: 3.0,
                    fill: PAPER,
                    stroke: None,
                });
                // Four bars that jump on every beat and settle between.
                let beat = self.beat_of(t);
                let pulse = (-beat.fract() * 3.2).exp();
                for i in 0..4 {
                    let x = shell.x + shell.width - 20.0 - i as f32 * 6.5;
                    let wobble = 0.5 + 0.5 * (t * 9.0 + i as f32 * 1.7).sin();
                    let h = 4.0 + 12.0 * (0.35 + 0.65 * pulse) * wobble;
                    over.push(Item::Rect {
                        rect: Rect::new(x - 1.5, center(shell).y - h * 0.5, 3.0, h),
                        radius: 1.5,
                        fill: PAPER,
                        stroke: None,
                    });
                }
            }
            Content::IslandTitle => {
                let x = shell.x + 46.0 + (1.0 - presence) * 14.0;
                over.push(text(
                    Point::new(x, center(shell).y),
                    "Midnight Drive",
                    13.0,
                    520,
                    PAPER,
                    TextAlign::Left,
                ));
            }
            Content::Player => self.player(t, &mut under, &mut over),
            Content::Volume => {
                let (v, stretch) = self.volume(t);
                let y = center(shell).y;
                let end = VOLUME_X.1 + stretch;
                under.push(bar(VOLUME_X.0, end, y, 6.0, TRACK));
                let knob = VOLUME_X.0 + v * (VOLUME_X.1 - VOLUME_X.0) + stretch;
                under.push(bar(VOLUME_X.0, knob, y, 6.0, INK));
                let s = shell.x + 24.0;
                over.push(Item::Path {
                    points: vec![
                        Point::new(s, y - 3.5),
                        Point::new(s + 4.0, y - 3.5),
                        Point::new(s + 9.0, y - 8.0),
                        Point::new(s + 9.0, y + 8.0),
                        Point::new(s + 4.0, y + 3.5),
                        Point::new(s, y + 3.5),
                    ],
                    stroke: Stroke {
                        width: ICON_STROKE,
                        color: INK,
                    },
                    closed: true,
                });
                over.push(path(arc(Point::new(s + 10.0, y), 5.0, -0.8, 1.6), INK));
                let loud = ((v - 0.5) * 4.0).clamp(0.0, 1.0);
                over.push(path(
                    arc(Point::new(s + 10.0, y), 9.5, -0.85, 1.7),
                    fade(INK, loud),
                ));
            }
            Content::Tabs => {
                let row = self.tab_row(t);
                let cell = row.width / 3.0;
                let left = row.x + k.tab[0].value(t) * cell;
                let right = row.x + (k.tab[1].value(t) + 1.0) * cell;
                for (i, name) in TAB_NAMES.iter().enumerate() {
                    let x0 = row.x + i as f32 * cell;
                    let covered = ((right.min(x0 + cell) - left.max(x0)) / cell).clamp(0.0, 1.0);
                    let color = mix(INK, PAPER, covered);
                    over.push(text(
                        Point::new(x0 + cell * 0.5, row.y + row.height * 0.5),
                        name,
                        14.0,
                        540,
                        color,
                        TextAlign::Center,
                    ));
                }
            }
            Content::Chart => self.chart(t, &mut under, &mut over),
            Content::Tooltip => self.tooltip(t, &mut over),
            Content::Search => {
                search_icon(&mut over, Point::new(-SEARCH.0 * 0.5 + 24.0, 0.0), MUTED);
                over.push(text(
                    Point::new(-SEARCH.0 * 0.5 + 42.0, 0.0),
                    "Search",
                    15.0,
                    450,
                    MUTED,
                    TextAlign::Left,
                ));
                // The command key is drawn, not typed: not every font has it.
                command_key(&mut over, Point::new(SEARCH.0 * 0.5 - 38.5, 0.0), MUTED);
                over.push(text(
                    Point::new(SEARCH.0 * 0.5 - 27.0, 0.0),
                    "K",
                    12.0,
                    600,
                    MUTED,
                    TextAlign::Center,
                ));
            }
            Content::Palette => self.palette(t, &mut under, &mut over),
            Content::Toast => {
                let icon = Point::new(-TOAST.0 * 0.5 + 30.0, 0.0);
                let check = [
                    Point::new(icon.x - 5.5, icon.y + 0.5),
                    Point::new(icon.x - 1.8, icon.y + 4.0),
                    Point::new(icon.x + 5.5, icon.y - 4.0),
                ];
                over.push(path(check.to_vec(), INK));
                over.push(text(
                    Point::new(icon.x + 26.0, 0.0),
                    "Link copied",
                    15.0,
                    520,
                    PAPER,
                    TextAlign::Left,
                ));
                over.push(text(
                    Point::new(TOAST.0 * 0.5 - 22.0, 0.0),
                    "Undo",
                    14.0,
                    520,
                    fade(PAPER, 0.55),
                    TextAlign::Right,
                ));
            }
        }
        (under, over)
    }

    fn player(&self, t: f32, under: &mut Vec<Item>, over: &mut Vec<Item>) {
        let (w, h) = PLAYER;
        let (left, top) = (-w * 0.5, -h * 0.5);
        let art = Rect::new(left + 20.0, top + 20.0, 60.0, 60.0);
        over.push(Item::Rect {
            rect: art,
            radius: 14.0,
            fill: INK,
            stroke: None,
        });
        let disc = center(art);
        over.push(path(arc(disc, 12.0, 0.0, std::f32::consts::TAU), PAPER));
        over.push(Item::Rect {
            rect: Rect::new(disc.x - 2.5, disc.y - 2.5, 5.0, 5.0),
            radius: 2.5,
            fill: PAPER,
            stroke: None,
        });
        over.push(text(
            Point::new(art.x + art.width + 16.0, art.y + 19.0),
            "Midnight Drive",
            16.0,
            600,
            INK,
            TextAlign::Left,
        ));
        over.push(text(
            Point::new(art.x + art.width + 16.0, art.y + 42.0),
            "Neon Coast",
            13.0,
            450,
            MUTED,
            TextAlign::Left,
        ));

        let progress = self.progress(t).clamp(0.0, 1.0);
        under.push(bar(SCRUB_X.0, SCRUB_X.1, SCRUB_Y, 4.0, TRACK));
        under.push(bar(SCRUB_X.0, scrub_x(progress), SCRUB_Y, 4.0, INK));
        let elapsed = progress * SONG_SECONDS;
        over.push(text(
            Point::new(SCRUB_X.0, SCRUB_Y + 18.0),
            &clock(elapsed),
            11.0,
            500,
            MUTED,
            TextAlign::Left,
        ));
        over.push(text(
            Point::new(SCRUB_X.1, SCRUB_Y + 18.0),
            &format!("-{}", clock(SONG_SECONDS - elapsed)),
            11.0,
            500,
            MUTED,
            TextAlign::Right,
        ));

        // Previous, play/pause, next.
        for side in [-1.0f32, 1.0] {
            let x = side * 58.0;
            let tip = x + side * 5.0;
            let back = x - side * 4.0;
            over.push(Item::Polygon {
                points: vec![
                    Point::new(back, PLAY_Y - 6.0),
                    Point::new(tip, PLAY_Y),
                    Point::new(back, PLAY_Y + 6.0),
                ],
                fill: INK,
                stroke: Some(Stroke {
                    width: 1.0,
                    color: INK,
                }),
            });
            over.push(Item::Path {
                points: vec![
                    Point::new(tip + side * 0.5, PLAY_Y - 6.0),
                    Point::new(tip + side * 0.5, PLAY_Y + 6.0),
                ],
                stroke: Stroke {
                    width: ICON_STROKE,
                    color: INK,
                },
                closed: false,
            });
        }
        over.push(Item::Rect {
            rect: Rect::new(-21.0, PLAY_Y - 21.0, 42.0, 42.0),
            radius: 21.0,
            fill: INK,
            stroke: None,
        });
        // Play is a triangle cut into two quads; pause is two bars. The
        // corners travel from one to the other.
        let p = self.tracks.playing.value(t);
        let (l, r, m) = (-4.5, 8.5, 2.0);
        let edge = -7.5 * (1.0 - (m - l) / (r - l));
        let play = [
            [(l, -7.5), (m, edge), (m, -edge), (l, 7.5)],
            [(m, edge), (r, 0.0), (r, 0.0), (m, -edge)],
        ];
        let pause = [
            [(-6.5, -7.5), (-1.8, -7.5), (-1.8, 7.5), (-6.5, 7.5)],
            [(1.8, -7.5), (6.5, -7.5), (6.5, 7.5), (1.8, 7.5)],
        ];
        for (a, b) in play.iter().zip(pause.iter()) {
            over.push(Item::Polygon {
                points: a
                    .iter()
                    .zip(b.iter())
                    .map(|(a, b)| Point::new(a.0 + (b.0 - a.0) * p, PLAY_Y + a.1 + (b.1 - a.1) * p))
                    .collect(),
                fill: PAPER,
                stroke: None,
            });
        }
    }

    fn chart(&self, t: f32, under: &mut Vec<Item>, over: &mut Vec<Item>) {
        let top = -CHART.1 * 0.5;
        over.push(text(
            Point::new(PLOT.x, top + 88.0),
            "12,480 plays",
            20.0,
            620,
            INK,
            TextAlign::Left,
        ));
        for i in 0..3 {
            let y = PLOT.y + PLOT.height * i as f32 / 2.0;
            under.push(Item::Rect {
                rect: Rect::new(PLOT.x, y - 0.5, PLOT.width, 1.0),
                radius: 0.0,
                fill: TRACK,
                stroke: None,
            });
        }
        for (i, day) in DAYS.iter().enumerate() {
            over.push(text(
                Point::new(day_x(i), PLOT.y + PLOT.height + 20.0),
                &day[..1],
                11.0,
                500,
                MUTED,
                TextAlign::Center,
            ));
        }
        let drawn = self.tracks.chart_draw.value(t).clamp(0.0, 1.0);
        let line = trim(&self.curve, drawn);
        if line.len() > 1 {
            let base = PLOT.y + PLOT.height;
            let mut area = line.clone();
            area.push(Point::new(line[line.len() - 1].x, base));
            area.push(Point::new(line[0].x, base));
            under.push(Item::Polygon {
                points: area,
                fill: fade(self.accent_or(INK), 0.07),
                stroke: None,
            });
            under.push(Item::Path {
                points: line,
                stroke: Stroke {
                    width: 2.5,
                    color: self.accent_or(INK),
                },
                closed: false,
            });
        }
    }

    fn tooltip(&self, t: f32, over: &mut Vec<Item>) {
        let x = self.cursor_local(t).x.clamp(PLOT.x, PLOT.x + PLOT.width);
        let y = curve_y(&self.curve, x);
        over.push(Item::Rect {
            rect: Rect::new(x - 0.75, PLOT.y, 1.5, PLOT.height),
            radius: 0.75,
            fill: fade(INK, 0.18),
            stroke: None,
        });
        over.push(Item::Rect {
            rect: Rect::new(x - 6.0, y - 6.0, 12.0, 12.0),
            radius: 6.0,
            fill: PAPER,
            stroke: Some(Stroke {
                width: 3.0,
                color: self.accent_or(INK),
            }),
        });
        let day = ((x - PLOT.x) / (PLOT.width / 6.0)).round().clamp(0.0, 6.0) as usize;
        let label = format!("{}  {:.1}k", DAYS[day], PLAYS[day]);
        let tip = Rect::new(x - 50.0, y - 46.0, 100.0, 30.0);
        over.push(Item::Rect {
            rect: tip,
            radius: 8.0,
            fill: INK,
            stroke: None,
        });
        over.push(text(
            center(tip),
            &label,
            12.0,
            560,
            PAPER,
            TextAlign::Center,
        ));
    }

    fn palette(&self, t: f32, _under: &mut Vec<Item>, over: &mut Vec<Item>) {
        let left = -PALETTE_WIDTH * 0.5;
        let header = PALETTE_TOP + PALETTE_HEADER * 0.5;
        search_icon(over, Point::new(left + 24.0, header), INK);
        let beat = self.beat_of(t);
        let query = TYPING
            .iter()
            .rev()
            .find(|(at, _)| beat >= *at)
            .map_or("", |(_, q)| q);
        let caret_on = ((beat - 24.0) * 2.0).rem_euclid(1.0) < 0.6 || query.len() < 3;
        let caret = caret_on.then_some(self.accent_or(INK));
        if query.is_empty() {
            over.push(Item::Text {
                at: Point::new(left + 42.0, header),
                text: String::new(),
                size: 15.0,
                weight: 500,
                color: INK,
                align: TextAlign::Left,
                caret,
            });
            over.push(text(
                Point::new(left + 46.0, header),
                "Type a command",
                15.0,
                450,
                MUTED,
                TextAlign::Left,
            ));
        } else {
            over.push(Item::Text {
                at: Point::new(left + 42.0, header),
                text: query.to_string(),
                size: 15.0,
                weight: 520,
                color: INK,
                align: TextAlign::Left,
                caret,
            });
        }
        over.push(Item::Rect {
            rect: Rect::new(left, PALETTE_TOP + PALETTE_HEADER - 0.5, PALETTE_WIDTH, 1.0),
            radius: 0.0,
            fill: TRACK,
            stroke: None,
        });
        let rows_top = PALETTE_TOP + PALETTE_HEADER + 6.0;
        for (i, name) in COMMANDS.iter().enumerate() {
            let (slot, presence) = &self.tracks.rows[i];
            let presence = presence.value(t).clamp(0.0, 1.0);
            if presence < 0.01 {
                continue;
            }
            let y = rows_top + slot.value(t) * ROW + ROW * 0.5;
            over.push(text(
                Point::new(left + 20.0, y),
                name,
                14.0,
                480,
                fade(INK, presence),
                TextAlign::Left,
            ));
        }
        // The enter key on the highlighted row.
        let y = rows_top + ROW * 0.5;
        let x = -left - 26.0;
        let enter = fade(MUTED, 0.9);
        over.push(Item::Path {
            points: vec![
                Point::new(x + 6.0, y - 5.0),
                Point::new(x + 6.0, y + 1.0),
                Point::new(x - 6.0, y + 1.0),
            ],
            stroke: Stroke {
                width: ICON_STROKE * 0.8,
                color: enter,
            },
            closed: false,
        });
        over.push(Item::Path {
            points: vec![
                Point::new(x - 3.0, y - 2.0),
                Point::new(x - 6.0, y + 1.0),
                Point::new(x - 3.0, y + 4.0),
            ],
            stroke: Stroke {
                width: ICON_STROKE * 0.8,
                color: enter,
            },
            closed: false,
        });
    }
}

/// Where the fades happen, bottom first.
const CONTENT_ORDER: [Content; CONTENTS] = [
    Content::Button,
    Content::Loader,
    Content::Check,
    Content::Island,
    Content::IslandTitle,
    Content::Player,
    Content::Volume,
    Content::Chart,
    Content::Tabs,
    Content::Tooltip,
    Content::Search,
    Content::Palette,
    Content::Toast,
];

const PUCK_ORDER: [Puck; PUCKS] = [
    Puck::BeforeScrub,
    Puck::Scrub,
    Puck::Volume,
    Puck::Toggle,
    Puck::Tab,
    Puck::Keycap,
    Puck::Row,
    Puck::ToastIcon,
    Puck::AfterToast,
];

fn scrub_x(fraction: f32) -> f32 {
    SCRUB_X.0 + fraction * (SCRUB_X.1 - SCRUB_X.0)
}

fn scrub_fraction(x: f32) -> f32 {
    ((x - SCRUB_X.0) / (SCRUB_X.1 - SCRUB_X.0)).clamp(0.0, 1.0)
}

/// Volume and stretch for a cursor at local `x` while the knob is held.
fn volume_drag(x: f32) -> (f32, f32) {
    let v = ((x - VOLUME_X.0) / (VOLUME_X.1 - VOLUME_X.0)).clamp(0.0, 1.0);
    (v, rubber_band(x - VOLUME_X.1, 60.0))
}

fn day_x(i: usize) -> f32 {
    PLOT.x + PLOT.width * i as f32 / 6.0
}

fn day_y(value: f32) -> f32 {
    let (lo, hi) = (0.8, 3.4);
    PLOT.y + PLOT.height * (1.0 - (value - lo) / (hi - lo))
}

/// The chart line through the week's plays, as a smooth Catmull-Rom curve.
fn chart_curve() -> Vec<Point> {
    let keys: Vec<Point> = PLAYS
        .iter()
        .enumerate()
        .map(|(i, v)| Point::new(day_x(i), day_y(*v)))
        .collect();
    let at = |i: isize| keys[i.clamp(0, keys.len() as isize - 1) as usize];
    let mut points = vec![keys[0]];
    for i in 0..keys.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for s in 1..=12 {
            let u = s as f32 / 12.0;
            let (u2, u3) = (u * u, u * u * u);
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b
                    + (-a + c) * u
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * u2
                    + (-a + 3.0 * b - 3.0 * c + d) * u3)
            };
            points.push(Point::new(
                f(p0.x, p1.x, p2.x, p3.x),
                f(p0.y, p1.y, p2.y, p3.y),
            ));
        }
    }
    points
}

/// The height of the curve at `x`.
fn curve_y(curve: &[Point], x: f32) -> f32 {
    for w in curve.windows(2) {
        if x >= w[0].x && x <= w[1].x {
            let t = if w[1].x > w[0].x {
                (x - w[0].x) / (w[1].x - w[0].x)
            } else {
                0.0
            };
            return w[0].y + (w[1].y - w[0].y) * t;
        }
    }
    curve.last().map_or(0.0, |p| p.y)
}

fn clock(seconds: f32) -> String {
    let s = seconds.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

fn center(r: Rect) -> Point {
    Point::new(r.x + r.width * 0.5, r.y + r.height * 0.5)
}

fn text(at: Point, text: &str, size: f32, weight: u16, color: Color, align: TextAlign) -> Item {
    Item::Text {
        at,
        text: text.to_string(),
        size,
        weight,
        color,
        align,
        caret: None,
    }
}

fn path(points: Vec<Point>, color: Color) -> Item {
    Item::Path {
        points,
        stroke: Stroke {
            width: ICON_STROKE,
            color,
        },
        closed: false,
    }
}

/// A horizontal bar with round ends from `x0` to `x1` around `y`.
fn bar(x0: f32, x1: f32, y: f32, height: f32, fill: Color) -> Item {
    Item::Rect {
        rect: Rect::new(x0, y - height * 0.5, (x1 - x0).max(0.0), height),
        radius: height * 0.5,
        fill,
        stroke: None,
    }
}

/// The command key's looped square, about as tall as a capital letter.
fn command_key(items: &mut Vec<Item>, c: Point, color: Color) {
    let stroke = Stroke { width: 1.3, color };
    let (inner, loop_center, radius) = (1.5, 3.0, 1.5);
    for (a, b) in [
        ((-loop_center, -inner), (loop_center, -inner)),
        ((-loop_center, inner), (loop_center, inner)),
        ((-inner, -loop_center), (-inner, loop_center)),
        ((inner, -loop_center), (inner, loop_center)),
    ] {
        items.push(Item::Path {
            points: vec![
                Point::new(c.x + a.0, c.y + a.1),
                Point::new(c.x + b.0, c.y + b.1),
            ],
            stroke,
            closed: false,
        });
    }
    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let center = Point::new(c.x + sx * loop_center, c.y + sy * loop_center);
        items.push(Item::Path {
            points: arc(center, radius, 0.0, std::f32::consts::TAU),
            stroke,
            closed: true,
        });
    }
}

fn search_icon(items: &mut Vec<Item>, c: Point, color: Color) {
    items.push(path(
        arc(
            Point::new(c.x - 1.5, c.y - 1.5),
            6.0,
            0.0,
            std::f32::consts::TAU,
        ),
        color,
    ));
    items.push(path(
        vec![
            Point::new(c.x + 3.0, c.y + 3.0),
            Point::new(c.x + 7.0, c.y + 7.0),
        ],
        color,
    ));
}

/// Builds every track from the beat grid, once.
struct Builder {
    grid: BeatGrid,
    accent: Option<Color>,
    period: f32,
    spb: f32,
}

/// Content fades in a moment after its state begins and out as it ends, so
/// the outgoing and incoming text never sit on top of each other.
const ENTER: Spring = Spring::new(0.2, 1.0);
const ENTER_DELAY: f32 = 0.05;
const EXIT: Spring = Spring::new(0.12, 1.0);
const SHELL: Spring = Spring::new(0.34, 0.86);
const CURSOR: Spring = Spring::new(0.42, 0.92);
const DRAG: Spring = Spring::new(0.7, 1.0);

impl Builder {
    fn new(grid: BeatGrid, accent: Option<Color>) -> Self {
        let spb = grid.beat_length();
        Self {
            grid,
            accent,
            period: REEL_BEATS * spb,
            spb,
        }
    }

    fn at(&self, beat: f32) -> f32 {
        beat * self.spb
    }

    fn look(&self, state: ReelState) -> Look {
        let accent = self.accent.unwrap_or(INK);
        match state {
            ReelState::Button => Look::new(220.0, 60.0, INK, 3.6),
            ReelState::Loader => Look::new(60.0, 60.0, INK, 5.6),
            ReelState::Check => Look::new(60.0, 60.0, accent, 5.6),
            ReelState::Island => Look::new(190.0, 44.0, INK, 3.1),
            ReelState::Player => Look::new(PLAYER.0, PLAYER.1, PAPER, 2.65).radius(30.0),
            ReelState::Volume => Look::new(VOLUME.0, VOLUME.1, PAPER, 2.8),
            ReelState::Toggle => Look::new(TOGGLE.0, TOGGLE.1, OFF, 4.8),
            ReelState::Tabs => Look::new(TABS.0, TABS.1, PAPER, 2.7),
            ReelState::Chart => Look::new(CHART.0, CHART.1, PAPER, 2.2).radius(28.0),
            ReelState::Search => Look::new(SEARCH.0, SEARCH.1, PAPER, 3.2),
            ReelState::Palette => palette_look(COMMANDS.len()),
            ReelState::Toast => Look::new(TOAST.0, TOAST.1, INK, 2.9),
        }
    }

    fn build(self) -> Reel {
        let period = self.period;
        let first = self.look(ReelState::Button);
        let mut shell = ShellTracks::new(first);

        // The shell and camera through every state, with the in-state changes
        // (the island opening, the toggle filling, the palette filtering).
        for (beat, state) in TIMELINE {
            let look = self.look(state);
            shell.to(self.at(beat), look);
            match state {
                ReelState::Island => {
                    let open = Look::new(300.0, 48.0, INK, 3.1);
                    shell.to(self.at(5.0), open);
                }
                ReelState::Toggle => {
                    let on = Look {
                        fill: self.accent.unwrap_or(INK),
                        ..look
                    };
                    shell.to(self.at(16.0), on);
                }
                ReelState::Palette => {
                    for (beat, query) in &TYPING[1..] {
                        let count = COMMANDS
                            .iter()
                            .filter(|c| c.to_lowercase().contains(query))
                            .count();
                        shell.to(self.at(*beat), palette_look(count));
                    }
                }
                _ => {}
            }
        }

        // The cursor: where it heads, and from when.
        let rest = |look: Look, x: f32, y: f32| look.stage(Point::new(x, y));
        let player = self.look(ReelState::Player);
        let volume = self.look(ReelState::Volume);
        let tabs = self.look(ReelState::Tabs);
        let chart = self.look(ReelState::Chart);
        let search = self.look(ReelState::Search);
        let palette = palette_look(1);
        let button = self.look(ReelState::Button);
        let toggle = self.look(ReelState::Toggle);
        let loader = self.look(ReelState::Loader);
        let island = Look::new(300.0, 48.0, INK, 3.1);
        let toast = self.look(ReelState::Toast);
        let tab_center = |i: usize| {
            let cell = (TABS.0 - 12.0) / 3.0;
            Point::new(-TABS.0 * 0.5 + 6.0 + cell * (i as f32 + 0.5), 0.0)
        };
        let aim = button.stage(Point::new(14.0, 8.0));
        let mut cursor = CursorPath::new(aim);
        cursor.go(self.at(1.25), rest(loader, 34.0, 44.0), CURSOR);
        cursor.go(self.at(4.25), rest(island, 110.0, 50.0), CURSOR);
        cursor.go(
            self.at(6.25),
            player.stage(Point::new(2.0, PLAY_Y + 4.0)),
            CURSOR,
        );
        let scrub_press = scrub_x(0.36);
        cursor.go(
            self.at(7.3),
            player.stage(Point::new(scrub_press, SCRUB_Y + 1.0)),
            CURSOR,
        );
        cursor.go(
            self.at(8.4),
            player.stage(Point::new(scrub_x(0.74), SCRUB_Y + 3.0)),
            DRAG,
        );
        cursor.go(self.at(10.25), rest(player, 150.0, 118.0), CURSOR);
        let knob = VOLUME_X.0 + 0.45 * (VOLUME_X.1 - VOLUME_X.0);
        cursor.go(
            self.at(11.2),
            volume.stage(Point::new(knob + 1.0, 2.0)),
            CURSOR,
        );
        cursor.go(
            self.at(12.2),
            volume.stage(Point::new(VOLUME_X.1 + 118.0, 5.0)),
            Spring::new(1.0, 1.0),
        );
        cursor.go(self.at(14.2), rest(volume, 120.0, 62.0), CURSOR);
        cursor.go(self.at(15.25), toggle.stage(Point::new(4.0, 6.0)), CURSOR);
        cursor.go(self.at(16.3), rest(toggle, 44.0, 44.0), CURSOR);
        let day = tab_center(0);
        cursor.go(
            self.at(17.35),
            tabs.stage(Point::new(day.x + 4.0, day.y + 6.0)),
            CURSOR,
        );
        let week = tab_center(1);
        cursor.go(
            self.at(18.3),
            tabs.stage(Point::new(week.x + 4.0, week.y + 6.0)),
            CURSOR,
        );
        cursor.go(self.at(19.3), rest(chart, 170.0, 190.0), CURSOR);
        cursor.go(
            self.at(20.5),
            chart.stage(Point::new(day_x(3), day_y(PLAYS[3]) + 36.0)),
            CURSOR,
        );
        cursor.go(
            self.at(21.5),
            chart.stage(Point::new(day_x(5), day_y(PLAYS[5]) + 36.0)),
            DRAG,
        );
        cursor.go(self.at(23.2), search.stage(Point::new(-30.0, 6.0)), CURSOR);
        cursor.go(self.at(24.3), rest(palette, 150.0, 60.0), CURSOR);
        cursor.go(self.at(26.3), rest(toast, 150.0, 60.0), CURSOR);
        cursor.go(self.at(27.2), aim, CURSOR);

        // Presses: clicks are a short down-up that lands on the beat; drags
        // hold from one beat to another.
        let click = |beat: f32| Press {
            down: self.at(beat) - 0.09,
            up: self.at(beat),
        };
        let hold = |from: f32, to: f32| Press {
            down: self.at(from),
            up: self.at(to),
        };
        let scrub_press_ = hold(8.0, 10.0);
        let volume_press = hold(12.0, 14.0);
        let presses = vec![
            click(1.0),
            click(7.0),
            scrub_press_,
            volume_press,
            click(16.0),
            click(18.0),
            click(19.0),
            click(24.0),
        ];
        let mut cursor_down = SpringTrack::new(0.0);
        let mut squash = SpringTrack::new(0.0);
        for press in &presses {
            cursor_down.set_to(press.down, 1.0, Spring::QUICK);
            cursor_down.set_to(press.up, 0.0, Spring::SNAPPY);
        }
        for press in [presses[0], presses[4], presses[7]] {
            squash.set_to(press.down, 1.0, Spring::QUICK);
            squash.set_to(press.up, 0.0, SHELL);
        }

        let cursor_x = cursor.x.looped(period);
        let cursor_y = cursor.y.looped(period);
        let zoom = shell.zoom.looped(period);
        let cam_x = shell.cam_x.looped(period);
        let cam_y = shell.cam_y.looped(period);
        let squash = squash.looped(period);
        // Where the drags let go: the value under the cursor at that moment,
        // seen through the camera at that moment.
        let local_x = |t: f32| {
            let zoom = zoom.value(t) * (1.0 - 0.035 * squash.value(t));
            cam_x.value(t) + (cursor_x.value(t) - STAGE * 0.5) / zoom
        };
        let scrub_released = scrub_fraction(local_x(scrub_press_.up));
        let (volume_released, stretch_released) = volume_drag(local_x(volume_press.up));
        let stretch = SpringTrack::new(0.0)
            .jump(volume_press.up, stretch_released)
            .to(volume_press.up, 0.0, Spring::new(0.36, 0.8));

        // Content fades.
        let windows: [(Content, f32, f32); CONTENTS] = [
            (Content::Button, 27.0, 1.0),
            (Content::Loader, 1.0, 3.0),
            (Content::Check, 3.0, 4.0),
            (Content::Island, 4.0, 6.0),
            (Content::IslandTitle, 5.0, 6.0),
            (Content::Player, 6.0, 11.0),
            (Content::Volume, 11.0, 15.0),
            (Content::Chart, 19.0, 23.0),
            (Content::Tabs, 17.0, 23.0),
            (Content::Tooltip, 20.85, 23.0),
            (Content::Search, 23.0, 24.0),
            (Content::Palette, 24.0, 26.0),
            (Content::Toast, 26.0, 27.0),
        ];
        let presence = CONTENT_ORDER
            .iter()
            .map(|content| {
                let (_, enter, exit) = windows.iter().find(|(c, ..)| c == content).unwrap();
                let (enter, exit) = (self.at(*enter), self.at(*exit));
                // The toast's text waits for its icon to shrink out of the way.
                let enter = enter
                    + if *content == Content::Toast {
                        0.05
                    } else {
                        0.0
                    };
                let track = if enter > exit {
                    // Spans the loop: on at the start, off, then on again.
                    SpringTrack::new(1.0)
                        .to(exit, 0.0, EXIT)
                        .to(enter + ENTER_DELAY, 1.0, ENTER)
                } else {
                    SpringTrack::new(0.0)
                        .to(enter + ENTER_DELAY, 1.0, ENTER)
                        .to(exit, 0.0, EXIT)
                };
                track.looped(period)
            })
            .collect();

        let hover = SpringTrack::new(1.0)
            .to(self.at(1.0) - 0.09, 0.0, Spring::QUICK)
            .to(self.at(27.7), 1.0, Spring::QUICK)
            .looped(period);
        let sweep = SpringTrack::new(0.22)
            .to(self.at(2.0), 0.64, SHELL)
            .to(self.at(3.0), 1.0, SHELL)
            .to(self.at(6.0), 0.22, Spring::QUICK)
            .looped(period);
        let check = SpringTrack::new(0.0)
            .to(self.at(3.0) + 0.02, 1.0, Spring::new(0.24, 1.0))
            .to(self.at(6.0), 0.0, Spring::QUICK)
            .looped(period);
        let playing = SpringTrack::new(0.0)
            .to(self.at(7.0), 1.0, Spring::SNAPPY)
            .to(self.at(12.0), 0.0, Spring::QUICK)
            .looped(period);
        let knob = SpringTrack::new(16.0)
            .to(self.at(10.0), 12.0, Spring::SNAPPY)
            .to(self.at(13.0), 16.0, Spring::QUICK)
            .looped(period);
        // The knob's edges on different springs: the edge in front leads, the
        // one behind follows, and the knob stretches while it travels.
        let toggle = [
            SpringTrack::new(0.0)
                .to(self.at(16.0), 1.0, Spring::TRAILING)
                .to(self.at(19.0), 0.0, Spring::QUICK)
                .looped(period),
            SpringTrack::new(0.0)
                .to(self.at(16.0), 1.0, Spring::LEADING)
                .to(self.at(19.0), 0.0, Spring::QUICK)
                .looped(period),
        ];
        let tab = [
            SpringTrack::new(2.0)
                .to(self.at(18.0), 0.0, Spring::LEADING)
                .to(self.at(19.0), 1.0, Spring::TRAILING)
                .to(self.at(24.5), 2.0, Spring::QUICK)
                .looped(period),
            SpringTrack::new(2.0)
                .to(self.at(18.0), 0.0, Spring::TRAILING)
                .to(self.at(19.0), 1.0, Spring::LEADING)
                .to(self.at(24.5), 2.0, Spring::QUICK)
                .looped(period),
        ];
        let chart_open = SpringTrack::new(0.0)
            .to(self.at(19.0), 1.0, SHELL)
            .to(self.at(24.5), 0.0, Spring::QUICK)
            .looped(period);
        let chart_draw = SpringTrack::new(0.0)
            .to(self.at(20.0), 1.0, Spring::new(0.95, 1.0))
            .to(self.at(24.5), 0.0, Spring::QUICK)
            .looped(period);

        // Palette rows: which slot each takes as the query narrows, and
        // whether it is still listed.
        let rows = (0..COMMANDS.len())
            .map(|i| {
                let mut slot = SpringTrack::new(i as f32);
                let mut shown = SpringTrack::new(1.0);
                for (beat, query) in &TYPING[1..] {
                    let visible: Vec<usize> = (0..COMMANDS.len())
                        .filter(|j| COMMANDS[*j].to_lowercase().contains(query))
                        .collect();
                    match visible.iter().position(|j| *j == i) {
                        Some(position) => slot.set_to(self.at(*beat), position as f32, SHELL),
                        None => shown.set_to(self.at(*beat), 0.0, EXIT),
                    }
                }
                slot.set_to(self.at(27.0), i as f32, Spring::QUICK);
                shown.set_to(self.at(27.0), 1.0, Spring::QUICK);
                (slot.looped(period), shown.looped(period))
            })
            .collect();

        let puck = Blend::new(PUCKS, Puck::AfterToast as usize)
            .to(self.at(7.5), Puck::BeforeScrub as usize, Spring::QUICK)
            .to(self.at(8.0), Puck::Scrub as usize, Spring::SNAPPY)
            .to(self.at(11.0), Puck::Volume as usize, SHELL)
            .to(self.at(15.0), Puck::Toggle as usize, SHELL)
            .to(self.at(17.0), Puck::Tab as usize, SHELL)
            .to(self.at(23.0), Puck::Keycap as usize, SHELL)
            .to(self.at(24.0), Puck::Row as usize, SHELL)
            .to(
                self.at(26.0),
                Puck::ToastIcon as usize,
                Spring::new(0.22, 0.95),
            )
            .to(self.at(27.0), Puck::AfterToast as usize, SHELL)
            .looped(period);

        let mut cues = vec![];
        let mut cue = |beat: f32, sound: Sound| {
            cues.push(Cue {
                time: self.at(beat),
                beat,
                sound,
            })
        };
        for beat in [1.0, 7.0, 18.0, 19.0, 24.0] {
            cue(beat, Sound::Click);
        }
        cue(3.0, Sound::Success);
        cue(8.0, Sound::Press);
        cue(10.0, Sound::Release);
        cue(12.0, Sound::Press);
        cue(14.0, Sound::Release);
        cue(16.0, Sound::Toggle);
        for (beat, _) in &TYPING[1..] {
            cue(*beat, Sound::Type);
        }
        cue(26.0, Sound::Enter);
        cue(26.0, Sound::Pop);
        cues.sort_by(|a, b| a.time.total_cmp(&b.time));

        Reel {
            grid: self.grid,
            accent: self.accent,
            period,
            tracks: Tracks {
                x: shell.x.looped(period),
                y: shell.y.looped(period),
                width: shell.width.looped(period),
                height: shell.height.looped(period),
                radius: shell.radius.looped(period),
                fill: shell.fill.map(|c| c.looped(period)),
                zoom,
                cam_x,
                cam_y,
                squash,
                hover,
                cursor_x,
                cursor_y,
                cursor_down: cursor_down.looped(period),
                presence,
                sweep,
                check,
                playing,
                knob,
                stretch: stretch.looped(period),
                toggle,
                tab,
                chart_open,
                chart_draw,
                rows,
                puck,
            },
            scrub: (scrub_press_, scrub_released),
            volume: (volume_press, volume_released),
            curve: chart_curve(),
            cues,
        }
    }
}

/// The palette with `rows` commands listed, its top edge kept in place.
fn palette_look(rows: usize) -> Look {
    let height = PALETTE_HEADER + 12.0 + rows as f32 * ROW;
    Look::new(PALETTE_WIDTH, height, PAPER, 2.2)
        .radius(22.0)
        .center(0.0, PALETTE_TOP + height * 0.5)
}

struct ShellTracks {
    x: SpringTrack,
    y: SpringTrack,
    width: SpringTrack,
    height: SpringTrack,
    radius: SpringTrack,
    fill: [SpringTrack; 3],
    zoom: SpringTrack,
    cam_x: SpringTrack,
    cam_y: SpringTrack,
}

impl ShellTracks {
    fn new(look: Look) -> Self {
        Self {
            x: SpringTrack::new(look.center.x),
            y: SpringTrack::new(look.center.y),
            width: SpringTrack::new(look.width),
            height: SpringTrack::new(look.height),
            radius: SpringTrack::new(look.radius),
            fill: [0, 1, 2].map(|i| SpringTrack::new(look.fill[i])),
            zoom: SpringTrack::new(look.zoom),
            cam_x: SpringTrack::new(look.center.x),
            cam_y: SpringTrack::new(look.center.y),
        }
    }

    fn to(&mut self, time: f32, look: Look) {
        self.x.set_to(time, look.center.x, SHELL);
        self.y.set_to(time, look.center.y, SHELL);
        self.width.set_to(time, look.width, SHELL);
        self.height.set_to(time, look.height, SHELL);
        self.radius.set_to(time, look.radius, SHELL);
        for i in 0..3 {
            self.fill[i].set_to(time, look.fill[i], Spring::new(0.3, 1.0));
        }
        // The camera is heavier than what it films.
        self.zoom.set_to(time, look.zoom, Spring::SMOOTH);
        self.cam_x.set_to(time, look.center.x, Spring::SMOOTH);
        self.cam_y.set_to(time, look.center.y, Spring::SMOOTH);
    }
}

struct CursorPath {
    x: SpringTrack,
    y: SpringTrack,
}

impl CursorPath {
    fn new(start: Point) -> Self {
        Self {
            x: SpringTrack::new(start.x),
            y: SpringTrack::new(start.y),
        }
    }

    fn go(&mut self, time: f32, to: Point, spring: Spring) {
        self.x.set_to(time, to.x, spring);
        self.y.set_to(time, to.y, spring);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reel() -> Reel {
        Reel::default()
    }

    #[test]
    fn seven_bars_at_120_bpm_last_fourteen_seconds() {
        assert_eq!(reel().duration(), 14.0);
    }

    #[test]
    fn seeking_is_a_pure_function_of_time() {
        let reel = reel();
        let a = reel.seek(8.7);
        reel.seek(2.0);
        reel.seek(13.9);
        assert_eq!(a, reel.seek(8.7));
    }

    #[test]
    fn the_last_frame_leads_into_the_first() {
        let reel = reel();
        let end = reel.duration() - 1.0 / 240.0;
        let start = reel.cursor(0.0);
        let before = reel.cursor(end);
        assert!((start.x - before.x).abs() < 2.0 && (start.y - before.y).abs() < 2.0);
        let (v0, v1) = (reel.cursor_velocity(0.0), reel.cursor_velocity(end));
        assert!((v0.x - v1.x).abs() < 20.0 && (v0.y - v1.y).abs() < 20.0);
        let (z0, c0) = reel.camera(0.0);
        let (z1, c1) = reel.camera(end);
        assert!((z0 - z1).abs() < 0.01 && (c0.x - c1.x).abs() < 0.1 && (c0.y - c1.y).abs() < 0.1);
    }

    #[test]
    fn every_track_the_loop_depends_on_is_closed() {
        let reel = reel();
        let k = &reel.tracks;
        for track in [
            &k.x, &k.y, &k.width, &k.height, &k.radius, &k.zoom, &k.cam_x, &k.cam_y,
        ]
        .into_iter()
        .chain(k.fill.iter())
        .chain(k.presence.iter())
        .chain([
            &k.cursor_x,
            &k.cursor_y,
            &k.stretch,
            &k.chart_open,
            &k.chart_draw,
        ]) {
            assert!(track.is_closed());
        }
    }

    #[test]
    fn states_follow_the_timeline() {
        let reel = reel();
        assert_eq!(reel.state_at(0.2), ReelState::Button);
        assert_eq!(reel.state_at(reel.secs(1.0)), ReelState::Loader);
        assert_eq!(reel.state_at(reel.secs(9.0)), ReelState::Player);
        assert_eq!(reel.state_at(reel.secs(25.0)), ReelState::Palette);
        assert_eq!(reel.state_at(reel.secs(27.5)), ReelState::Button);
    }

    #[test]
    fn a_drag_follows_the_cursor_and_is_let_go_where_it_was() {
        let reel = reel();
        let (press, released) = reel.scrub;
        let before = reel.progress(press.up - 1e-3);
        assert!((before - released).abs() < 0.01, "{before} vs {released}");
        assert!(
            released > 0.6,
            "the scrub moved the song forward: {released}"
        );
        let (_, stretch) = reel.volume(reel.secs(13.9));
        assert!(stretch > 10.0, "dragging past max stretches: {stretch}");
        let (_, after) = reel.volume(reel.volume.0.up + 1e-3);
        assert!((after - stretch).abs() < 3.0, "released from where it was");
        let (_, settled) = reel.volume(reel.secs(14.9));
        assert!(settled.abs() < 1.0);
    }

    #[test]
    fn every_beat_has_a_caption_and_sounds_sit_on_the_grid() {
        let reel = reel();
        assert_eq!(BEATS.len(), REEL_BEATS as usize);
        for cue in reel.cues() {
            assert!(
                (cue.beat * 4.0).fract() == 0.0,
                "{cue:?} is off the sixteenth grid"
            );
        }
    }

    #[test]
    fn content_fades_with_its_own_timing() {
        let reel = reel();
        // Just after the click the button label is leaving and the loader
        // has not arrived yet: they never overlap at full strength.
        let t = reel.secs(1.0) + 0.03;
        let k = &reel.tracks;
        let button = k.presence[0].value(t);
        let loader = k.presence[1].value(t);
        assert!(button + loader < 1.2, "{button} + {loader}");
    }

    #[test]
    fn a_slower_song_stretches_the_reel() {
        let reel = Reel::new(BeatGrid::new(100.0), Some(hex(0x3366FF)));
        assert!((reel.duration() - 16.8).abs() < 1e-4);
        assert_eq!(reel.state_at(reel.secs(19.5)), ReelState::Chart);
    }
}
