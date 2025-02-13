//! Not public API.
//!
//! Exposed for use by macros.

pub use serenity::all as serenity;
use serenity::ResolvedOption;

use crate::{Context, Error, SlashArg};

/// Helper trait to handle [`Option`] parameters uniformly.
///
/// Not public API.
pub trait SlashArgOption<'ctx>: Sized {
    /// Whether the parameter is required.
    ///
    /// `false` for [`Option`], `true` otherwise.
    const REQUIRED: bool;

    /// The required equivalent to `Self`, usually being the inner [`SlashArg`].
    ///
    /// The `T` in [`Option<T>`], `Self` otherwise.
    type Required: SlashArg<'ctx>;

    /// Tries to extract the argument value.
    ///
    /// `f` is a predicate to match the correct option.
    ///
    /// If the parameter is found, behaves like [`Self::Inner::extract`], though
    /// wrapping the optional parameters in [`Some`].
    ///
    /// If the parameter isn't found, returns `Some(None)` for optional
    /// parameters and otherwise returns an error.
    ///
    /// [`Self::Inner::extract`]: SlashArg::extract
    fn try_extract(
        ctx: &Context<'ctx>,
        f: impl FnMut(&ResolvedOption<'ctx>) -> bool,
    ) -> Result<Self, Error<'ctx>>;
}

impl<'ctx, T: SlashArg<'ctx>> SlashArgOption<'ctx> for T {
    const REQUIRED: bool = true;
    type Required = Self;

    fn try_extract(
        ctx: &Context<'ctx>,
        mut f: impl FnMut(&ResolvedOption<'ctx>) -> bool,
    ) -> Result<Self, Error<'ctx>> {
        match ctx.options().iter().find(move |o| f(o)) {
            Some(o) => Self::extract(ctx, &o.value),
            None => Err(Error::structure_mismatch(
                *ctx,
                "a required parameter is missing",
            )),
        }
    }
}

impl<'ctx, T: SlashArg<'ctx>> SlashArgOption<'ctx> for Option<T> {
    const REQUIRED: bool = false;
    type Required = T;

    fn try_extract(
        ctx: &Context<'ctx>,
        mut f: impl FnMut(&ResolvedOption<'ctx>) -> bool,
    ) -> Result<Self, Error<'ctx>> {
        match ctx.options().iter().find(move |o| f(o)) {
            Some(o) => T::extract(ctx, &o.value).map(Some),
            None => Ok(None),
        }
    }
}
