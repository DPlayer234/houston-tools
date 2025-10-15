use serenity::futures::future::BoxFuture;
use serenity::gateway::client::{Context, EventHandler, FullEvent};

use crate::helper::futures::BoxedJoinFut;

/// Similar to [`EventHandler`], but pushes its future to [`BoxedJoinFut`]
/// instead of returning it. May do nothing if it does not want to handle the
/// event.
pub trait PushEventHandler: Send + Sync + 'static {
    /// Dispatch the event.
    ///
    /// If the event is handled, the future will be pushed to the
    /// [`BoxedJoinFut`] parameter.
    fn push_dispatch<'s, 'c, 'e, 'a>(
        &'s self,
        _ctx: &'c Context,
        _event: &'e FullEvent,
        _fut: &mut BoxedJoinFut<'a>,
    ) where
        's: 'a,
        'c: 'a,
        'e: 'a,
    {
    }
}

/// An [`EventHandler`] that combines several [`PushEventHandler`]s.
pub struct HEventHandler {
    handlers: Box<[Box<dyn PushEventHandler>]>,
}

impl HEventHandler {
    /// Creates a new handler.
    pub fn new(handlers: Box<[Box<dyn PushEventHandler>]>) -> Self {
        Self { handlers }
    }
}

impl EventHandler for HEventHandler {
    fn dispatch<'s, 'c, 'e, 'a>(
        &'s self,
        ctx: &'c Context,
        event: &'e FullEvent,
    ) -> BoxFuture<'a, ()>
    where
        's: 'a,
        'c: 'a,
        'e: 'a,
        Self: 'a,
    {
        let mut fut = BoxedJoinFut::default();
        for handler in &self.handlers {
            handler.push_dispatch(ctx, event, &mut fut);
        }
        fut.end()
    }
}
