//! Helpers for hosts that turn a platform's events into [`Input`](crate::Input).

/// A button as the frames see it.
///
/// A host samples its buttons once per frame, so a press and its release
/// arriving between two frames -- a quick tap, a click from automation, a
/// slow frame -- would never be seen at all. The latch holds such a press
/// down for one frame and lets it go on the next, so every press reaches the
/// UI and every release follows it.
#[derive(Clone, Copy, Debug, Default)]
pub struct ButtonLatch {
    down: bool,
    /// Pressed since the last sample.
    unseen_press: bool,
    /// Released before its press was sampled; let go at the next sample.
    pending_release: bool,
}

impl ButtonLatch {
    pub fn press(&mut self) {
        self.down = true;
        self.unseen_press = true;
        self.pending_release = false;
    }

    pub fn release(&mut self) {
        if self.unseen_press {
            self.pending_release = true;
        } else {
            self.down = false;
        }
    }

    pub fn set(&mut self, pressed: bool) {
        if pressed {
            self.press()
        } else {
            self.release()
        }
    }

    /// Let go at once, whatever the frames have seen: the window lost focus.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether the button is physically held right now.
    pub fn held(&self) -> bool {
        self.down && !self.pending_release
    }

    /// Whether the frame being built sees the button down.
    pub fn sample(&mut self) -> bool {
        if self.unseen_press {
            self.unseen_press = false;
            return true;
        }
        if self.pending_release {
            self.pending_release = false;
            self.down = false;
        }
        self.down
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_released_between_frames_is_seen_for_one_frame() {
        let mut button = ButtonLatch::default();
        button.press();
        button.release();
        assert!(!button.held());
        assert!(button.sample(), "the press reaches a frame");
        assert!(!button.sample(), "and the release the next");
    }

    #[test]
    fn a_held_button_stays_down() {
        let mut button = ButtonLatch::default();
        button.press();
        assert!(button.sample());
        assert!(button.sample());
        button.release();
        assert!(!button.sample());
    }
}
