use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A future type that will return [`Poll::Ready`] with the value of a closure on every poll.
#[derive(Debug, Clone, Copy)]
pub struct Done<F: ?Sized>(F);

impl<F, T> Done<F>
where
    F: Fn() -> T,
{
    /// Constructs a new [`Done`].
    pub const fn new(f: F) -> Self {
        Self(f)
    }

    /// Constructs a new [`Done`], asserting that the value is a zero-sized type.
    ///
    /// This is useful to avoid actually allocating for boxed futures with fixed results.
    pub const fn new_zst(f: F) -> Self {
        const { debug_assert!(size_of::<Self>() == 0, "Done<F> was expected to be zero-sized"); }
        Self::new(f)
    }
}

impl<F: ?Sized, T> Future for Done<F>
where
    F: Fn() -> T,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        // polling a future after it has returned Ready
        // may do anything except cause undefined behavior.
        Poll::Ready((self.0)())
    }
}
