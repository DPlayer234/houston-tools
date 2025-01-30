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
    #[error("argument parse error: {error}")]
    ArgumentParse {
        #[source]
        error: anyhow::Error,
        input: Option<String>,
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
    pub fn structure_mismatch(ctx: Context<'a>, message: &'static str) -> Self {
        Self::StructureMismatch { message, ctx }
    }

    /// Constructs a new [`Error::ArgInvalid`] variant.
    pub fn arg_invalid(ctx: Context<'a>, message: &'static str) -> Self {
        Self::ArgInvalid { message, ctx }
    }

    /// Constructs a new [`Error::ArgumentParse`] variant.
    pub fn argument_parse(
        ctx: Context<'a>,
        input: Option<String>,
        error: impl Into<anyhow::Error>,
    ) -> Self {
        Self::ArgumentParse {
            error: error.into(),
            input,
            ctx,
        }
    }
}
