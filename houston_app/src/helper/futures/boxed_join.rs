use serenity::futures::FutureExt as _;
use serenity::futures::future::{BoxFuture, join};

use super::noop_future;

/// Allows joining unit-returning futures into a boxed one in an
/// allocation-optimized manner.
#[derive(Default)]
pub struct BoxedJoinFut<'a> {
    acc: Option<BoxFuture<'a, ()>>,
}

impl<'a> BoxedJoinFut<'a> {
    /// Completes the builder and returns the final boxed future.
    pub fn end(self) -> BoxFuture<'a, ()> {
        self.acc.unwrap_or_else(noop_future)
    }

    /// Pushes another future to the end of the set.
    pub fn push<F>(&mut self, fut: F)
    where
        F: Future<Output = ()> + Send + 'a,
    {
        if let Some(last) = self.acc.take() {
            // if there already is a future stored, we join it while `F` is still known,
            // thus getting a `Join<BoxFuture, F>`, which is then mapped to reduce the
            // return type and boxed, thus only needing 1 extra box rather than 2 as it
            // would be necessary for a `Join<BoxFuture, BoxFuture>`. downside is that the
            // left side may do some recursive poll calls, but the join counts we're
            // expecting are like 3 at most, usually just 2 if even.
            let join = join(last, fut).map(|((), ())| ());
            self.acc = Some(Box::pin(join));
        } else {
            // if this is first future, we just box and store it
            self.acc = Some(Box::pin(fut));
        }
    }
}
