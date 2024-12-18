use std::sync::atomic::{AtomicBool, Ordering};

/// Simple synchronization primitive.
///
/// Allows ensuring a block is only entered by the first thread that reaches it
/// by using [`OnceReset::set`]. If it is then determined that the action needs
/// to be rerun later, [`OnceReset::reset`] can be used to allow future threads
/// to enter.
#[derive(Debug)]
pub struct OnceReset {
    state: AtomicBool,
}

impl Default for OnceReset {
    fn default() -> Self {
        Self::new()
    }
}

impl OnceReset {
    /// Creates a new reset in an open state.
    pub const fn new() -> Self {
        Self {
            state: AtomicBool::new(true),
        }
    }

    /// Attempts to set the state to closed.
    ///
    /// If the caller is allowed to enter, returns `true`.
    pub fn set(&self) -> bool {
        self.state.swap(false, Ordering::Acquire)
    }

    /// Resets the state so future callers may enter on [`Self::set`].
    pub fn reset(&self) {
        self.state.store(true, Ordering::Release);
    }
}
