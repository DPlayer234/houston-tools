//! Private helpers for the [`button_value`] macro.

use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;

use houston_cmd::BoxFuture;
use serde::Deserialize;
pub use serenity::all as serenity;
pub use thiserror;

use super::{AnyContext, AnyInteraction, ButtonAction, ButtonReply, encoding};
use crate::{ButtonContext, ButtonValue, ModalContext, Result};

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

pub const fn alias_action<T: ButtonValue>(key: usize) -> ButtonAction {
    ButtonAction { key, ..T::ACTION }
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
                impl $crate::private::ButtonDispatch for __Dispatch {
                    type This<$l> = $Ty;
                }

                $crate::private::make_action::<__Dispatch>($key)
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

/// Error type for `text_ref` kind [`modal_parser`] fields.
///
/// This type is uninhabited and cannot occur.
#[derive(Debug, thiserror::Error)]
#[error("unreachable")]
pub struct TextRefError<T> {
    /// Public field so users can match on this type being empty.
    pub infallible: Infallible,
    marker: PhantomData<T>,
}

/// Emits utility types to parse a specified modal structure.
///
/// The emitted types are:
/// - `Fields`: The successful return type matching the macro input syntax.
/// - `Error`: An error type for failures.
/// - `parse`: The entry point function.
///
/// # Examples
///
/// ```
/// houston_btn::modal_parser! {
///     required text page: u16,
///     optional text key: String,
/// }
/// ```
#[macro_export]
macro_rules! modal_parser {
    (@field_ty required $Ty:ty) => { $Ty };
    (@field_ty optional $Ty:ty) => { ::std::option::Option<$Ty> };

    (@error_ty text $Ty:ty) => { <$Ty as ::std::str::FromStr>::Err };
    (@error_ty text_ref $Ty:ty) => { $crate::private::TextRefError<$Ty> };

    (@parse required text $key:ident : $Ty:ty) => {
        match &$key.component {
            $crate::private::serenity::LabelComponent::InputText($crate::private::serenity::InputText {
                value: ::std::option::Option::Some(value),
                custom_id,
                ..
            }) if custom_id == ::std::stringify!($key) => {
                match value.parse::<$Ty>() {
                    ::std::result::Result::Ok(value) => value,
                    ::std::result::Result::Err(err) => return ::std::result::Result::Err(Error::$key(err)),
                }
            },
            _ => return ::std::result::Result::Err(Error::Invalid),
        }
    };
    (@parse optional text $key:ident : $Ty:ty) => {
        match &$key.component {
            $crate::private::serenity::LabelComponent::InputText($crate::private::serenity::InputText {
                value: ::std::option::Option::Some(value),
                custom_id,
                ..
            }) if custom_id == ::std::stringify!($key) => {
                match value.parse::<$Ty>() {
                    ::std::result::Result::Ok(value) => ::std::option::Option::Some(value),
                    ::std::result::Result::Err(err) => return ::std::result::Result::Err(Error::$key(err)),
                }
            },
            $crate::private::serenity::LabelComponent::InputText(_) => return ::std::result::Result::Err(Error::Invalid),
            _ => ::std::option::Option::None,
        }
    };

    (@parse required text_ref $key:ident : $Ty:ty) => {
        match &$key.component {
            $crate::private::serenity::LabelComponent::InputText($crate::private::serenity::InputText {
                value: ::std::option::Option::Some(value),
                custom_id,
                ..
            }) if custom_id == ::std::stringify!($key) => value,
            _ => return ::std::result::Result::Err(Error::Invalid),
        }
    };
    (@parse optional text_ref $key:ident : $Ty:ty) => {
        match &$key.component {
            $crate::private::serenity::LabelComponent::InputText($crate::private::serenity::InputText {
                value: ::std::option::Option::Some(value),
                custom_id,
                ..
            }) if custom_id == ::std::stringify!($key) => ::std::option::Option::Some(value as $Ty),
            $crate::private::serenity::LabelComponent::InputText(_) => return ::std::result::Result::Err(Error::Invalid),
            _ => ::std::option::Option::None,
        }
    };

    (
        $($life:lifetime =>)?
        $( $required:ident $kind:ident $key:ident : $Ty:ty ),*
        $(,)?
    ) => {
        /// The fields returned and loaded by [`parse`].
        pub struct Fields$(<$life>)? {
            $(pub $key: $crate::modal_parser!(@field_ty $required $Ty)),*
        }

        /// An error for [`parse`].
        #[derive(::std::fmt::Debug, $crate::private::thiserror::Error)]
        #[expect(non_camel_case_types)]
        pub enum Error$(<$life>)? {
            /// The interaction does not match the expected modal shape.
            #[error("interaction does not match expected modal")]
            Invalid,
            $(
                /// Parsing the field failed.
                #[error("field {}: {}", ::std::stringify!($key), _0)]
                $key($crate::modal_parser!(@error_ty $kind $Ty))
            ),*
        }

        /// Parses a modal interaction and returns the loaded fields.
        pub fn parse$(<$life>)?(interaction: &$($life)? $crate::private::serenity::ModalInteraction) -> ::std::result::Result<Fields$(<$life>)?, Error$(<$life>)?> {
            let [$($crate::private::serenity::Component::Label($key)),*] = interaction.data.components.as_slice() else {
                return ::std::result::Result::Err(Error::Invalid);
            };

            $(
                let $key = $crate::modal_parser!(@parse $required $kind $key: $Ty);
            )*

            ::std::result::Result::Ok(Fields {
                $($key),*
            })
        }
    };
}
