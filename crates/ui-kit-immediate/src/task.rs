//! Work that finishes later -- a request to a server, a file read, a model
//! loading -- run on the UI thread alongside the frames.
//!
//! An immediate-mode application asks each frame whether its work is done:
//!
//! ```ignore
//! if self.projects.is_none() {
//!     self.projects = Some(task::spawn(client.list_projects()));
//! }
//! if let Some(list) = self.projects.as_mut().and_then(Task::take) {
//!     self.shown = list;
//! }
//! ```
//!
//! Futures run on the thread that spawned them, so they need not be `Send`,
//! and the frame loop keeps them moving: natively the runner polls what was
//! woken before each frame (and wakes the event loop when something is), in
//! a browser they are the page's own promises. With the `tokio` feature the
//! native runner also enters a Tokio runtime, so Tokio's timers and I/O work
//! inside them.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

/// The result of work started with [`spawn`], once it is done.
///
/// Dropping the handle cancels the work: natively it is not polled again,
/// and in a browser its result is thrown away. Call [`Task::detach`] to let
/// it run to the end regardless.
pub struct Task<T> {
    slot: Rc<RefCell<Option<T>>>,
    finished: Rc<Cell<bool>>,
    cancelled: Rc<Cell<bool>>,
}

impl<T> Task<T> {
    /// The result, the first time it is asked for after the work finished;
    /// `None` before then and after.
    pub fn take(&mut self) -> Option<T> {
        self.slot.borrow_mut().take()
    }

    /// Whether the work has finished, whether or not its result was taken.
    pub fn is_finished(&self) -> bool {
        self.finished.get()
    }

    /// Let the work run to the end without anyone waiting for its result.
    pub fn detach(self) {
        let cancelled = self.cancelled.clone();
        drop(self);
        cancelled.set(false);
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.cancelled.set(true);
    }
}

/// Start `future` on this thread and return a handle to its result.
pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> Task<T> {
    let slot = Rc::new(RefCell::new(None));
    let finished = Rc::new(Cell::new(false));
    let cancelled = Rc::new(Cell::new(false));
    let task = Task {
        slot: slot.clone(),
        finished: finished.clone(),
        cancelled: cancelled.clone(),
    };
    let run = async move {
        let value = future.await;
        if !cancelled.get() {
            *slot.borrow_mut() = Some(value);
        }
        finished.set(true);
    };
    executor::spawn(run, task.cancelled.clone());
    task
}

/// Start `future` on this thread with nothing waiting for it.
pub fn spawn_detached(future: impl Future<Output = ()> + 'static) {
    executor::spawn(future, Rc::new(Cell::new(false)));
}

/// Poll every task that was woken since the last call, until none is left
/// ready. The native runner does this before each frame; a test or a tool
/// driving an application without a runner calls it itself.
pub fn run_ready() {
    #[cfg(not(target_arch = "wasm32"))]
    executor::run_ready();
}

/// Called from any thread when a task is woken, so the event loop can come
/// round and poll it. The native runner installs it.
#[cfg(not(target_arch = "wasm32"))]
pub fn set_wake_hook(hook: impl Fn() + Send + Sync + 'static) {
    let _ = executor::WAKE_HOOK.set(Box::new(hook));
}

#[cfg(not(target_arch = "wasm32"))]
mod executor {
    use std::cell::{Cell, RefCell};
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::task::{Context, Wake, Waker};

    pub(super) static WAKE_HOOK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

    struct Entry {
        future: Pin<Box<dyn Future<Output = ()>>>,
        cancelled: Rc<Cell<bool>>,
    }

    /// The ids of the tasks woken since they were last polled.
    #[derive(Default)]
    struct Ready(Mutex<Vec<usize>>);

    struct TaskWaker {
        id: usize,
        ready: Arc<Ready>,
    }

    impl Wake for TaskWaker {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.ready.0.lock().unwrap().push(self.id);
            if let Some(hook) = WAKE_HOOK.get() {
                hook();
            }
        }
    }

    #[derive(Default)]
    struct Executor {
        tasks: Vec<Option<Entry>>,
        free: Vec<usize>,
        ready: Arc<Ready>,
    }

    thread_local! {
        static EXECUTOR: RefCell<Executor> = RefCell::default();
    }

    pub(super) fn spawn(future: impl Future<Output = ()> + 'static, cancelled: Rc<Cell<bool>>) {
        EXECUTOR.with_borrow_mut(|executor| {
            let entry = Entry {
                future: Box::pin(future),
                cancelled,
            };
            let id = match executor.free.pop() {
                Some(id) => {
                    executor.tasks[id] = Some(entry);
                    id
                }
                None => {
                    executor.tasks.push(Some(entry));
                    executor.tasks.len() - 1
                }
            };
            executor.ready.0.lock().unwrap().push(id);
        });
        if let Some(hook) = WAKE_HOOK.get() {
            hook();
        }
    }

    pub(super) fn run_ready() {
        loop {
            let (ids, ready) = EXECUTOR.with_borrow(|executor| {
                let ids = std::mem::take(&mut *executor.ready.0.lock().unwrap());
                (ids, executor.ready.clone())
            });
            if ids.is_empty() {
                return;
            }
            for id in ids {
                // Taken out while it runs, so that it can spawn more.
                let Some(mut entry) = EXECUTOR.with_borrow_mut(|e| e.tasks.get_mut(id).and_then(Option::take))
                else {
                    continue;
                };
                let done = entry.cancelled.get() || {
                    let waker = Waker::from(Arc::new(TaskWaker {
                        id,
                        ready: ready.clone(),
                    }));
                    let mut context = Context::from_waker(&waker);
                    entry.future.as_mut().poll(&mut context).is_ready()
                };
                EXECUTOR.with_borrow_mut(|executor| {
                    if done {
                        executor.free.push(id);
                    } else {
                        executor.tasks[id] = Some(entry);
                    }
                });
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod executor {
    use std::cell::Cell;
    use std::future::Future;
    use std::rc::Rc;

    pub(super) fn spawn(future: impl Future<Output = ()> + 'static, _cancelled: Rc<Cell<bool>>) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::task::Poll;

    #[test]
    fn a_result_is_taken_once() {
        let mut task = spawn(async { 7 });
        assert_eq!(task.take(), None, "nothing runs until the executor does");
        run_ready();
        assert!(task.is_finished());
        assert_eq!(task.take(), Some(7));
        assert_eq!(task.take(), None);
    }

    #[test]
    fn a_woken_task_runs_again() {
        let polls = Rc::new(Cell::new(0));
        let counted = polls.clone();
        let mut task = spawn(std::future::poll_fn(move |cx| {
            counted.set(counted.get() + 1);
            if counted.get() < 3 {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready("done")
            }
        }));
        run_ready();
        assert_eq!(polls.get(), 3);
        assert_eq!(task.take(), Some("done"));
    }

    #[test]
    fn a_dropped_task_is_not_polled() {
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        drop(spawn(async move { flag.set(true) }));
        run_ready();
        assert!(!ran.get());
    }

    #[test]
    fn a_detached_task_still_runs() {
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        spawn(async move { flag.set(true) }).detach();
        run_ready();
        assert!(ran.get());
    }
}
