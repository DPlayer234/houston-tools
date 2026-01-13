//! Private helpers for the [`button_value`] macro.

use std::fmt;

use houston_cmd::BoxFuture;
use serde::Deserialize;

use crate::{
    AnyContext, AnyInteraction, ButtonAction, ButtonContext, ButtonReply, ModalContext, Result,
    encoding,
};

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
        invoke_button: |ctx, buf| invoke(ctx, buf, T::This::reply, on_button),
        invoke_modal: |ctx, buf| invoke(ctx, buf, T::This::modal_reply, on_modal),
    }
}

/// Provides shared code for invoking button value actions.
fn invoke<'ctx, T, I, F>(
    ctx: AnyContext<'ctx, I>,
    buf: encoding::Decoder<'ctx>,
    f: impl Fn(T, AnyContext<'ctx, I>) -> F,
    on: impl Fn(AnyContext<'ctx, I>, &dyn fmt::Debug),
) -> BoxFuture<'ctx, Result>
where
    T: fmt::Debug + Deserialize<'ctx>,
    I: AnyInteraction,
    F: Future<Output = Result> + Send + 'ctx,
{
    match buf.into_button_value::<T>() {
        Ok(this) => {
            on(ctx, &this);
            Box::pin(f(this, ctx))
        },
        Err(why) => err_fut(why),
    }
}

fn on_button(ctx: ButtonContext<'_>, args: &dyn fmt::Debug) {
    if let Some(hooks) = ctx.inner.state.hooks.as_deref() {
        hooks.on_button(ctx, args)
    }
}

fn on_modal(ctx: ModalContext<'_>, args: &dyn fmt::Debug) {
    if let Some(hooks) = ctx.inner.state.hooks.as_deref() {
        hooks.on_modal(ctx, args)
    }
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
#[macro_export]
macro_rules! button_value {
    (for<$l:lifetime> $Ty:ty, $key:expr) => {
        impl<$l> $crate::ButtonValue for $Ty {
            const ACTION: $crate::ButtonAction = {
                enum __Dispatch {}
                impl $crate::private::dispatch::ButtonDispatch for __Dispatch {
                    type This<$l> = $Ty;
                }

                $crate::private::dispatch::make_action::<__Dispatch>($key)
            };

            fn to_nav(&self) -> $crate::Nav<'_> {
                $crate::Nav::from_button_value(self)
            }
        }
    };
    ($Ty:ty, $key:expr) => {
        $crate::button_value!(for<'__ignore> $Ty, $key);
    };
}
