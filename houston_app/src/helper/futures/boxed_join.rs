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
    #[inline]
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

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::{Mutex, MutexGuard};
    use std::task::{Context, Poll, Waker};

    use super::BoxedJoinFut;

    struct YieldOnce(bool);
    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.0 {
                self.0 = false;
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        }
    }

    fn lock(m: &Mutex<u8>) -> MutexGuard<'_, u8> {
        m.lock().expect("unpoisoned")
    }

    #[test]
    fn test_logic_ok() {
        let run = async {
            YieldOnce(false).await;
            YieldOnce(false).await;
        };

        let run = std::pin::pin!(run);

        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(run.poll(&mut cx), Poll::Ready(()));
    }

    #[test]
    fn single() {
        let state = Mutex::new(0);
        let one = async {
            *lock(&state) = 1;
            YieldOnce(true).await;
            *lock(&state) = 2;
        };

        let mut f = BoxedJoinFut::default();
        f.push(one);
        let mut f = f.end();

        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(*lock(&state), 1);
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Ready(()));
        assert_eq!(*lock(&state), 2);
    }

    #[test]
    fn multi() {
        let state = Mutex::new(0);
        let one = async {
            *lock(&state) |= 0x1;
        };
        let two = async {
            *lock(&state) |= 0x2;
            YieldOnce(true).await;
            *lock(&state) |= 0x8;
        };
        let three = async {
            *lock(&state) |= 0x4;
        };

        let mut f = BoxedJoinFut::default();
        f.push(one);
        f.push(two);
        f.push(three);
        let mut f = f.end();

        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(*lock(&state), 0x7);
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Ready(()));
        assert_eq!(*lock(&state), 0xF);
    }
}
