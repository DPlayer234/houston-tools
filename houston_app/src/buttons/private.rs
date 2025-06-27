use std::fmt;

use houston_cmd::BoxFuture;
use serde::Deserialize;

use super::{AnyContext, AnyInteraction, ButtonAction, ButtonReply, encoding};
use crate::fmt::discord::interaction_location;
use crate::prelude::*;

/// Provides a way to dispatch [`ButtonReply`] with different lifetimes.
pub trait ButtonDispatch {
    type This<'v>: ButtonReply + fmt::Debug + Deserialize<'v>;
}

/// Shared code to create a button action.
///
/// Used by the [`super::button_value`] macro.
pub const fn make_action<T: ButtonDispatch + 'static>(key: usize) -> ButtonAction {
    ButtonAction {
        key,
        invoke_button: |ctx, buf| invoke(ctx, buf, T::This::reply, "Button"),
        invoke_modal: |ctx, buf| invoke(ctx, buf, T::This::modal_reply, "Modal"),
    }
}

/// Provides shared code for invoking button value actions.
fn invoke<'ctx, T, I, F>(
    ctx: AnyContext<'ctx, I>,
    buf: encoding::Decoder<'ctx>,
    f: impl Fn(T, AnyContext<'ctx, I>) -> F,
    kind: &str,
) -> BoxFuture<'ctx, Result>
where
    T: fmt::Debug + Deserialize<'ctx>,
    I: AnyInteraction,
    F: Future<Output = Result> + Send + 'ctx,
{
    match buf.into_button_value::<T>() {
        Ok(this) => {
            log_interaction(kind, ctx.interaction, &this);
            Box::pin(f(this, ctx))
        },
        Err(why) => err_fut(why),
    }
}

// less generic interaction logging
fn log_interaction<I: AnyInteraction>(kind: &str, interaction: &I, args: &dyn fmt::Debug) {
    log::info!(
        "[{kind}] {}, {}: {args:?}",
        interaction_location(interaction.guild_id(), interaction.channel()),
        interaction.user().name,
    );
}

// shared boxed future type for the outer error case
#[cold]
fn err_fut<'ctx>(why: anyhow::Error) -> BoxFuture<'ctx, Result> {
    Box::pin(async move { Err(why) })
}

/// Implements the [`ButtonValue`] trait.
/// Accepts the type and its action key as a [`usize`].
///
/// The type in question needs to implement the following:
/// - [`ButtonReply`]
/// - [`fmt::Debug`]
/// - [`serde::Deserialize`]
/// - [`serde::Serialize`]
///
/// If the type has lifetimes, specify the type similar to: `for<'v> MyType<'v>`
///
/// [`ButtonValue`]: super::ButtonValue
macro_rules! button_value {
    (for<$l:lifetime> $Ty:ty, $key:expr) => {
        impl<$l> $crate::buttons::ButtonValue for $Ty {
            const ACTION: $crate::buttons::ButtonAction = {
                enum __Dispatch {}
                impl $crate::buttons::private::ButtonDispatch for __Dispatch {
                    type This<$l> = $Ty;
                }

                $crate::buttons::private::make_action::<__Dispatch>($key)
            };

            fn to_nav(&self) -> $crate::buttons::Nav<'_> {
                $crate::buttons::Nav::from_button_value(self)
            }
        }
    };
    ($Ty:ty, $key:literal) => {
        $crate::buttons::private::button_value!(for<'__ignore> $Ty, $key);
    };
}

pub(crate) use button_value;
