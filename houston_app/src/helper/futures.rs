use std::time::Duration;

use serenity::futures::future::{BoxFuture, always_ready};
use tokio::time::timeout;

/// Returns a ZST boxed future that does nothing.
pub fn noop_future() -> BoxFuture<'static, ()> {
    Box::pin(always_ready(|| {}))
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
/// the input futures by reference, for example by using the
/// [`pin`](std::pin::pin) macro:
///
/// ```
/// use std::time::Duration;
/// use std::pin::pin;
/// # use crate::helper::futures::if_too_long;
/// # async fn run() {}
/// # async fn intercept() {}
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
/// # assert_eq!((run_result, intercept_result), ((), None));
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
        Err(_) => tokio::join!(fut, async { Some(intercept.await) }),
    }
}
