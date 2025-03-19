use std::str::FromStr;

use serenity::all::{CommandOptionType, CreateCommandOption, ResolvedValue};

use super::SlashArg;
use crate::{Context, Error};

/// Wraps a [`FromStr`] implementation to be used as a [`SlashArg`].
///
/// The implementation's [`FromStr::Err`] must be [`Error`] and `'static` to be
/// supported.
///
/// To get the value, access the only field or match it.
///
/// [`Error`]: std::error::Error
#[derive(Debug, Clone, Copy)]
pub struct FromStrArg<T>(pub T);

impl<'ctx, T> SlashArg<'ctx> for FromStrArg<T>
where
    T: FromStr + 'ctx,
    T::Err: Into<anyhow::Error>,
{
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match resolved {
            ResolvedValue::String(value) => T::from_str(value)
                .map(FromStrArg)
                .map_err(|e| Error::arg_parse(*ctx, *value, e)),
            _ => Err(Error::structure_mismatch(*ctx, "expected string argument")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::String)
    }
}
