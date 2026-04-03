use serenity::futures::future::BoxFuture;
use serenity::gateway::client::{Context, EventHandler};
use serenity::model::event::FullEvent;

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
        ctx: &'c Context,
        event: &'e FullEvent,
        fut: &mut BoxedJoinFut<'a>,
    ) where
        's: 'a,
        'c: 'a,
        'e: 'a;
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

/// Convenient way to implement [`PushEventHandler`].
macro_rules! impl_push_handler {
    // the weird `match _ {}` part is intended so that the syntax is something
    // that rustfmt can format. in a sense, it's just a nicety.
    ($Type:ty, |$this:pat_param, $ctx:pat_param| match _ { $($pat:pat => $block:expr),* $(,)? }) => {
        impl $crate::helper::discord::events::PushEventHandler for $Type {
            fn push_dispatch<'s, 'c, 'e, 'a>(
                &'s self,
                $ctx: &'c ::serenity::gateway::client::Context,
                event: &'e ::serenity::model::event::FullEvent,
                fut: &mut $crate::helper::futures::BoxedJoinFut<'a>,
            )
            where
                's: 'a,
                'c: 'a,
                'e: 'a,
            {
                #[allow(clippy::let_underscore_untyped)]
                let $this = self;
                match event {
                    $( $pat => fut.push($block), )*
                    // users are allowed to exhaustively match
                    #[allow(unreachable_patterns)]
                    _ => {},
                }
            }
        }
    };
}

pub(crate) use impl_push_handler;
