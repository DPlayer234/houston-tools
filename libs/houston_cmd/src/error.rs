use std::borrow::Cow;

/// An error that can occur during command handling.
#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    /// The user-defined command function returned an error.
    #[error("command error: {error}")]
    Command {
        /// The error returned by the command function.
        #[source]
        error: anyhow::Error,
    },
    /// The in-memory structure did not match the received interaction.
    #[error("command structure mismatch: {message}")]
    StructureMismatch {
        /// The message to show for this error.
        message: &'static str,
    },
    /// The argument data isn't valid for this argument type.
    #[error("invalid argument: {message}")]
    ArgInvalid {
        /// The message to show for this error.
        message: &'static str,
    },
    /// Parsing the argument failed.
    #[error("argument `{input}` parse error: {error}")]
    ArgParse {
        /// The error returned by the parse function.
        #[source]
        error: anyhow::Error,
        /// The original input string that failed parsing.
        input: Cow<'a, str>,
    },
}

impl<'a> Error<'a> {
    /// Constructs a new [`Error::Command`] variant.
    pub fn command(error: impl Into<anyhow::Error>) -> Self {
        Self::Command {
            error: error.into(),
        }
    }

    /// Constructs a new [`Error::StructureMismatch`] variant.
    #[cold]
    pub fn structure_mismatch(message: &'static str) -> Self {
        Self::StructureMismatch { message }
    }

    /// Constructs a new [`Error::ArgInvalid`] variant.
    pub fn arg_invalid(message: &'static str) -> Self {
        Self::ArgInvalid { message }
    }

    /// Constructs a new [`Error::ArgParse`] variant.
    pub fn arg_parse(input: impl Into<Cow<'a, str>>, error: impl Into<anyhow::Error>) -> Self {
        Self::ArgParse {
            error: error.into(),
            input: input.into(),
        }
    }
}
