//! What changed on screen since the last frame drawn, so that a frame that
//! changed nothing is not drawn and one that changed a little is drawn only
//! where it did.
//!
//! The UI is drawn into a texture of its own that keeps its pixels from
//! frame to frame. The commands last drawn into it are kept too: a frame's
//! commands are compared with them, and where they differ is drawn again,
//! every command that touches it redrawn in order, clipped to it.

use ui_kit_core::{Material, Rect};
use ui_kit_immediate::{DrawCommand, IMAGE_MATERIAL};

/// Where a frame has to be drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Region {
    /// Nothing changed: the target already shows the frame.
    Nothing,
    /// Only here changed, in whole pixels.
    Part(Rect),
    Everywhere,
}

/// Past this share of the screen, drawing a part costs about what drawing
/// it all does.
const MOST: f32 = 0.6;

#[derive(Default)]
pub(crate) struct Damage {
    /// What the target shows, or `None` when it shows nothing known.
    shown: Option<Vec<DrawCommand>>,
}

impl Damage {
    /// Forget what the target shows: it was made anew, or drawn by someone else.
    pub fn invalidate(&mut self) {
        self.shown = None;
    }

    /// The target now shows `commands`.
    pub fn shown(&mut self, commands: Vec<DrawCommand>) {
        self.shown = Some(commands);
    }

    /// Where `commands` differ from what a `size` target shows. A command
    /// `live` says is drawn from something that changes on its own -- a
    /// picture the application renders into -- is drawn again every frame.
    pub fn region(
        &self,
        commands: &[DrawCommand],
        size: (u32, u32),
        live: impl Fn(&DrawCommand) -> bool,
    ) -> Region {
        let Some(shown) = &self.shown else {
            return Region::Everywhere;
        };
        let screen = Rect::new(0.0, 0.0, size.0 as f32, size.1 as f32);
        let same_start = shown.iter().zip(commands).take_while(|(a, b)| a == b).count();
        let same_end = shown[same_start..]
            .iter()
            .rev()
            .zip(commands[same_start..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let old = &shown[same_start..shown.len() - same_end];
        let new = &commands[same_start..commands.len() - same_end];
        let mut dirty: Option<Rect> = None;
        let mut add = |rect: Rect| {
            let rect = rect.intersect(screen);
            if rect.width > 0.0 && rect.height > 0.0 {
                dirty = Some(dirty.map_or(rect, |d| union(d, rect)));
            }
        };
        if old.len() == new.len() {
            // As many as before: each command stands where it stood, and
            // only those that changed moved anything.
            for (was, is) in old.iter().zip(new).filter(|(was, is)| was != is) {
                add(bounds(was, screen));
                add(bounds(is, screen));
            }
        } else {
            for command in old.iter().chain(new) {
                add(bounds(command, screen));
            }
        }
        for command in commands.iter().filter(|c| live(c)) {
            add(bounds(command, screen));
        }
        let Some(mut dirty) = dirty else {
            return Region::Nothing;
        };
        // Glass blurs what is round it: glass near what changed is drawn
        // again whole, with all it reads, so it reads nothing drawn over it.
        loop {
            let mut grown = dirty;
            for command in commands {
                if let Material::Glass { blur } = command.style.material {
                    let glass = bounds(command, screen);
                    let reads = expand(glass, blur.clamp(0.0, 32.0).ceil() * 2.0 + 1.0);
                    if reads.overlaps(grown) {
                        grown = union(grown, reads);
                    }
                }
            }
            let grown = snap(grown.intersect(screen));
            if grown == dirty {
                break;
            }
            dirty = grown;
        }
        if dirty.width * dirty.height > MOST * screen.width * screen.height {
            Region::Everywhere
        } else {
            Region::Part(dirty)
        }
    }
}

/// The commands that draw `region` of the target as `commands` draw all of
/// it: a clear to `background` first, then every command that touches it,
/// clipped to it.
pub(crate) fn clip_to(commands: &[DrawCommand], region: Rect, screen: Rect, background: [f32; 4]) -> Vec<DrawCommand> {
    let mut clear = DrawCommand {
        rich_text: None,
        layer: Default::default(),
        rect: region,
        clip: Some(region),
        style: Default::default(),
        kind: "clear".into(),
        text: String::new(),
        value: 0.0,
        hovered: false,
        active: false,
        focused: false,
        transform: None,
    };
    clear.style.fill = background;
    let mut clipped = vec![clear];
    clipped.extend(
        commands
            .iter()
            .filter(|command| bounds(command, screen).overlaps(region))
            .map(|command| {
                let mut command = command.clone();
                command.clip = Some(command.clip.map_or(region, |clip| clip.intersect(region)));
                command
            }),
    );
    clipped
}

/// Everything a command may paint, with a pixel to spare for antialiasing.
fn bounds(command: &DrawCommand, screen: Rect) -> Rect {
    let mut rect = command.rect;
    match &command.style.material {
        // A mesh is placed from its rect's corner but may reach past it.
        Material::Custom { name, .. } if name != IMAGE_MATERIAL => rect = screen,
        _ => {}
    }
    if command.kind == "shadow" {
        rect = expand(rect, command.value.max(0.0));
    }
    if command.kind == "checkbox" {
        // Its box is placed from the left, whatever the rect's size.
        let square = Rect::new(rect.x + 16.0, rect.y + (rect.height - 18.0) * 0.5, 18.0, 18.0);
        rect = union(rect, square);
    }
    let rect = expand(rect, 1.0);
    match command.clip {
        Some(clip) => rect.intersect(clip),
        None => rect,
    }
}

fn expand(rect: Rect, by: f32) -> Rect {
    Rect::new(rect.x - by, rect.y - by, rect.width + by * 2.0, rect.height + by * 2.0)
}

fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        (a.x + a.width).max(b.x + b.width) - x,
        (a.y + a.height).max(b.y + b.height) - y,
    )
}

/// The whole pixels `rect` touches.
fn snap(rect: Rect) -> Rect {
    let x = rect.x.floor();
    let y = rect.y.floor();
    Rect::new(x, y, (rect.x + rect.width).ceil() - x, (rect.y + rect.height).ceil() - y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(x: f32, fill: f32) -> DrawCommand {
        let mut command = clip_to(&[], Rect::new(x, 10.0, 20.0, 20.0), Rect::new(0.0, 0.0, 1.0, 1.0), [fill; 4])
            .remove(0);
        command.clip = None;
        command.kind = "panel".into();
        command
    }

    const SIZE: (u32, u32) = (400, 300);

    #[test]
    fn nothing_known_is_drawn_everywhere_and_the_same_frame_not_at_all() {
        let mut damage = Damage::default();
        let frame = vec![panel(10.0, 0.5), panel(100.0, 0.5)];
        assert_eq!(damage.region(&frame, SIZE, |_| false), Region::Everywhere);
        damage.shown(frame.clone());
        assert_eq!(damage.region(&frame, SIZE, |_| false), Region::Nothing);
    }

    #[test]
    fn a_changed_command_is_drawn_where_it_was_and_is() {
        let mut damage = Damage::default();
        damage.shown(vec![panel(10.0, 0.5), panel(100.0, 0.5), panel(200.0, 0.5)]);
        let Region::Part(region) = damage.region(
            &[panel(10.0, 0.5), panel(120.0, 0.5), panel(200.0, 0.5)],
            SIZE,
            |_| false,
        ) else {
            panic!("a part changed");
        };
        assert_eq!(region, Rect::new(99.0, 9.0, 42.0, 22.0));
    }

    #[test]
    fn a_live_command_is_always_drawn() {
        let mut damage = Damage::default();
        let frame = vec![panel(10.0, 0.5), panel(100.0, 0.5)];
        damage.shown(frame.clone());
        let region = damage.region(&frame, SIZE, |c| c.rect.x == 100.0);
        assert_eq!(region, Region::Part(Rect::new(99.0, 9.0, 22.0, 22.0)));
    }

    #[test]
    fn glass_near_a_change_is_drawn_whole() {
        let mut damage = Damage::default();
        let mut glass = panel(40.0, 0.5);
        glass.rect.width = 100.0;
        glass.style.material = Material::Glass { blur: 4.0 };
        damage.shown(vec![panel(10.0, 0.5), glass.clone()]);
        let Region::Part(region) = damage.region(&[panel(10.0, 0.6), glass], SIZE, |_| false) else {
            panic!("a part changed");
        };
        assert!(region.x <= 40.0 - 8.0 && region.x + region.width >= 140.0 + 8.0);
    }

    #[test]
    fn a_region_is_drawn_clipped_over_what_it_is_cleared_to() {
        let region = Rect::new(0.0, 0.0, 50.0, 50.0);
        let screen = Rect::new(0.0, 0.0, 400.0, 300.0);
        let clipped = clip_to(&[panel(10.0, 0.5), panel(200.0, 0.5)], region, screen, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clipped.len(), 2, "the clear, and the one panel in the region");
        assert_eq!(clipped[0].style.fill, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(clipped[1].clip, Some(region));
    }
}
