use std::borrow::Cow;

use crate::context::Context;

/// An error that can occur during command handling.
#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    /// The user-defined command function returned an error.
    #[error("command error: {error}")]
    Command {
        #[source]
        error: anyhow::Error,
        ctx: Context<'a>,
    },
    /// The in-memory structure did not match the received interaction.
    #[error("command structure mismatch: {message}")]
    StructureMismatch {
        message: &'static str,
        ctx: Context<'a>,
    },
    /// The argument data isn't valid for this argument type.
    #[error("invalid argument: {message}")]
    ArgInvalid {
        message: &'static str,
        ctx: Context<'a>,
    },
    /// Parsing the argument failed.
    #[error("argument `{input}` parse error: {error}")]
    ArgParse {
        #[source]
        error: anyhow::Error,
        input: Cow<'a, str>,
        ctx: Context<'a>,
    },
}

impl<'a> Error<'a> {
    /// Constructs a new [`Error::Command`] variant.
    pub fn command(ctx: Context<'a>, error: impl Into<anyhow::Error>) -> Self {
        Self::Command {
            error: error.into(),
            ctx,
        }
    }

    /// Constructs a new [`Error::StructureMismatch`] variant.
    #[cold]
    pub fn structure_mismatch(ctx: Context<'a>, message: &'static str) -> Self {
        Self::StructureMismatch { message, ctx }
    }

    /// Constructs a new [`Error::ArgInvalid`] variant.
    pub fn arg_invalid(ctx: Context<'a>, message: &'static str) -> Self {
        Self::ArgInvalid { message, ctx }
    }

    /// Constructs a new [`Error::ArgParse`] variant.
    pub fn arg_parse(
        ctx: Context<'a>,
        input: impl Into<Cow<'a, str>>,
        error: impl Into<anyhow::Error>,
    ) -> Self {
        Self::ArgParse {
            error: error.into(),
            input: input.into(),
            ctx,
        }
    }
}
