use std::time::Duration;

use serenity::futures::FutureExt as _;
use serenity::futures::future::{BoxFuture, always_ready, join};
use tokio::time::timeout;
use utils::mem::assert_zst;

/// Returns a ZST boxed future that does nothing.
pub fn noop_future<'a>() -> BoxFuture<'a, ()> {
    Box::pin(assert_zst(always_ready(|| {})))
}

/// Runs `fut` to completion. If it takes longer than `after` to complete,
/// [joins](tokio::join) `fut` and `intercept`.
///
/// If `fut` completes in time and `intercept` isn't polled, this future will
/// return `(fut.await, None)`.
///
/// If `intercept` is polled, this future will only complete after both `fut`
/// and `intercept` have run to completion. In this case, it will return the
/// equivalent of `(fut.await, Some(intercept.await))` but executed
/// concurrently.
///
/// As long as this future is run to completion, `fut` will also be completed
/// and `intercept` is either never polled or completed.
///
/// # Examples
///
/// It is generally required and otherwise recommended that the caller passes
/// the input futures pre-pinned, for example by using the
/// [`pin`](std::pin::pin) macro:
///
/// ```
/// use std::time::Duration;
/// use std::pin::pin;
/// # use crate::helper::futures::if_too_long;
/// # async fn run() -> u32 { 12345 }
/// # async fn intercept() -> ! { panic!() }
///
/// // create both futures
/// let run = run();
/// let intercept = intercept();
///
/// // run and try intercept
/// let (run_result, intercept_result) =
///     if_too_long(
///         pin!(run),
///         Duration::from_secs(5),
///         pin!(intercept),
///     ).await;
///
/// println!("run result: {run_result:?}");
/// if let Some(i) = intercept_result {
///     println!("run intercepted: {i:?}");
/// }
///
/// # assert_eq!((run_result, intercept_result), (12345, None));
/// ```
///
/// # Notes
///
/// This function will simply drop `intercept` if it isn't polled. If the caller
/// has already polled `intercept` before passing it to this function, this may
/// lead to cancellation.
///
/// The [`Unpin`] requirement for the input futures only exists to allow this
/// function to avoid re-pinning the futures if they already were pinned. It is
/// also a soft lint to avoid passing large futures here by value as this may
/// unexpectedly bloat the size of the caller.
pub async fn if_too_long<F, I>(
    mut fut: F,
    after: Duration,
    intercept: I,
) -> (F::Output, Option<I::Output>)
where
    F: Future + Unpin,
    I: Future + Unpin,
{
    match timeout(after, &mut fut).await {
        Ok(f) => (f, None),
        Err(_) => tokio::join!(&mut fut, async { Some(intercept.await) }),
    }
}

/// Allows joining futures into a boxed one in an allocation-optimized manner.
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
