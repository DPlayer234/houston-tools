//! Not public API.
//!
//! Exposed for use by macros.

use ::serenity::model::application::ResolvedValue;
pub use serenity::all as serenity;

use crate::args::SlashArg;
use crate::context::Context;
use crate::error::Error;

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
    /// If passed [`Some`], behaves like [`Self::Inner::extract`], though
    /// wrapping the output in [`Some`].
    ///
    /// If passed [`None`], returns `Ok(None)` for optional parameters and
    /// otherwise returns an error.
    ///
    /// [`Self::Inner::extract`]: SlashArg::extract
    fn try_extract(
        ctx: &Context<'ctx>,
        resolved: Option<&ResolvedValue<'ctx>>,
    ) -> Result<Self, Error<'ctx>>;
}

impl<'ctx, T: SlashArg<'ctx>> SlashArgOption<'ctx> for T {
    const REQUIRED: bool = true;
    type Required = Self;

    #[inline]
    fn try_extract(
        ctx: &Context<'ctx>,
        resolved: Option<&ResolvedValue<'ctx>>,
    ) -> Result<Self, Error<'ctx>> {
        match resolved {
            Some(o) => Self::extract(ctx, o),
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

    #[inline]
    fn try_extract(
        ctx: &Context<'ctx>,
        resolved: Option<&ResolvedValue<'ctx>>,
    ) -> Result<Self, Error<'ctx>> {
        match resolved {
            Some(o) => T::extract(ctx, o).map(Some),
            None => Ok(None),
        }
    }
}
