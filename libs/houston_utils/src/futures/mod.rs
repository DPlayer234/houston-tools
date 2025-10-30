use serenity::futures::future::{BoxFuture, always_ready};
use utils::mem::assert_zst;

mod boxed_join;
mod timeout;

pub use boxed_join::BoxedJoinFut;
pub use timeout::if_too_long;

/// Returns a ZST boxed future that does nothing.
pub fn noop_future<'a>() -> BoxFuture<'a, ()> {
    Box::pin(assert_zst(always_ready(|| {})))
}
