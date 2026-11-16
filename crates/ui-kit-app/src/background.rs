//! Work made on a thread of its own, taken when it is done: pipelines whose
//! shaders a slow phone's compiler takes seconds over, say. Held on the
//! event loop's thread meanwhile, Android reports the app as not responding
//! (a pause or a touch waits on that thread).
//!
//! ```ignore
//! let mut pipelines = Background::spawn("engine-pipelines", move || compile(&device));
//! // each frame:
//! if let Some(pipelines) = pipelines.get() { /* draw with them */ }
//! ```
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// What a thread is making, until it is made.
pub struct Background<T> {
    state: State<T>,
}

enum State<T> {
    Making(Receiver<T>),
    Done(T),
    Taken,
    Failed,
}

impl<T: Send + 'static> Background<T> {
    /// Makes it with `make` on a thread named `name`.
    pub fn spawn(name: &str, make: impl FnOnce() -> T + Send + 'static) -> Self {
        let (send, receive) = channel();
        let spawned = std::thread::Builder::new().name(name.into()).spawn(move || {
            let _ = send.send(make());
        });
        let state = match spawned {
            Ok(_) => State::Making(receive),
            Err(e) => {
                crate::log_error(&format!("{name}: no thread for it: {e}"));
                State::Failed
            }
        };
        Self { state }
    }
}

impl<T> Background<T> {
    /// Made already, with no thread.
    pub fn done(value: T) -> Self {
        Self { state: State::Done(value) }
    }

    /// What was made, once it is. Never waits.
    pub fn get(&mut self) -> Option<&mut T> {
        if let State::Making(receive) = &self.state {
            self.state = match receive.try_recv() {
                Ok(value) => State::Done(value),
                Err(TryRecvError::Empty) => return None,
                // The thread panicked: it is never made.
                Err(TryRecvError::Disconnected) => State::Failed,
            };
        }
        match &mut self.state {
            State::Done(value) => Some(value),
            _ => None,
        }
    }

    /// What was made, once it is, taken out of it: [`Background::get`]
    /// has nothing after.
    pub fn take(&mut self) -> Option<T> {
        self.get()?;
        match std::mem::replace(&mut self.state, State::Taken) {
            State::Done(value) => Some(value),
            _ => unreachable!("had a value"),
        }
    }

    /// Whether it will never be made: its thread panicked or could not start.
    pub fn failed(&self) -> bool {
        matches!(self.state, State::Failed)
    }

    /// Waits for it, holding this thread: for a frame rendered offline.
    pub fn wait(&mut self) -> Option<&mut T> {
        if let State::Making(receive) = &self.state {
            self.state = receive.recv().map_or(State::Failed, State::Done);
        }
        self.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_thread_makes_is_had_once_it_is_done() {
        let mut made = Background::spawn("test", || 42);
        assert_eq!(made.wait().copied(), Some(42));
        assert_eq!(made.get().copied(), Some(42));
        assert_eq!(made.take(), Some(42));
        assert_eq!(made.take(), None);
        assert!(!made.failed());
    }

    #[test]
    fn a_thread_that_panics_makes_nothing() {
        let mut made: Background<u32> = Background::spawn("test", || panic!("on purpose"));
        assert!(made.wait().is_none());
        assert!(made.failed());
    }
}
