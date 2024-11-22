use crate::context::Context;

#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    #[error("command error")]
    Command {
        #[source] error: anyhow::Error,
        ctx: Context<'a>,
    },
    #[error("command structure mismatch: {message}")]
    StructureMismatch {
        message: &'static str,
        ctx: Context<'a>,
    },
    #[error("invalid argument: {message}")]
    SlashArgInvalid {
        message: &'static str,
        ctx: Context<'a>,
    },
    #[error("argument error: {error}")]
    ArgumentParse {
        #[source] error: anyhow::Error,
        input: Option<String>,
        ctx: Context<'a>,
    },
}

impl<'a> Error<'a> {
    pub fn command(ctx: Context<'a>, error: impl Into<anyhow::Error>) -> Self {
        Self::Command { error: error.into(), ctx }
    }

    pub fn slash_arg_invalid(ctx: Context<'a>, message: &'static str) -> Self {
        Self::SlashArgInvalid { message, ctx }
    }

    pub fn structure_mismatch(ctx: Context<'a>, message: &'static str) -> Self {
        Self::StructureMismatch { message, ctx }
    }

    pub fn argument_parse(ctx: Context<'a>, input: Option<String>, error: impl Into<anyhow::Error>) -> Self {
        Self::ArgumentParse { error: error.into(), input, ctx }
    }
}
