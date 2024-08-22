use crate::prelude::*;

mod ast;
mod parse;
mod eval;

/// Evaluates a mathematical equation.
#[poise::command(slash_command)]
pub async fn calc(
    ctx: HContext<'_>,
    mut expression: String,
) -> HResult {
    expression.make_ascii_lowercase();

    macro_rules! error_embed {
        ($($t:tt)*) => {
            CreateEmbed::new()
                .description(format!($($t)*))
                .color(ERROR_EMBED_COLOR)
        };
    }

    let embed = match eval_text(expression.as_bytes()) {
        Ok(result) => CreateEmbed::new()
            .description(format!("{expression} = **{result}**"))
            .color(DEFAULT_EMBED_COLOR),

        Err(MathError::ExprExpected(Some(at)))
            => error_embed!("Expected expression at `{at}`."),

        Err(MathError::ExprExpected(None))
            => error_embed!("Unexpected empty expression."),

        Err(MathError::InvalidNumber(num))
            => error_embed!("`{num}` is not a valid number."),

        Err(MathError::InvalidUnaryOperator(op))
            => error_embed!("`{op}` is not a unary operator."),

        Err(MathError::InvalidBinaryOperator(op))
            => error_embed!("`{op}` is not a binary operator."),

        Err(MathError::InvalidFunction(function))
            => error_embed!("The function `{function}` is unknown."),

        Err(MathError::InvalidParameterCount { function, count: 1 })
            => error_embed!("The function `{function}` takes 1 parameter."),

        Err(MathError::InvalidParameterCount { function, count })
            => error_embed!("The function `{function}` takes {count} parameters."),

        Err(r) => Err(r)?,
    };

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

/// A result for math evaluation.
type Result<T> = std::result::Result<T, MathError>;

/// The kinds of errors that may occur when evaluating a mathematical expression.
#[derive(Debug)]
enum MathError {
    /// Some internal error. Usually not returned.
    Internal,

    /// A sub-expression was expected but not found.
    /// Holds the last token before the error.
    ExprExpected(Option<String>),

    /// Found a token that seemed to be a number but couldn't be parsed as one.
    /// Holds the token in question.
    InvalidNumber(String),

    /// Found a token that should be a unary operator but wasn't valid.
    /// Holds the token in question.
    InvalidUnaryOperator(String),

    /// Found a token in a binary operator position that wasn't valid.
    /// Holds the token in question.
    InvalidBinaryOperator(String),

    /// Encountered a call with an invalid function name.
    /// Holds the function name in question.
    InvalidFunction(String),

    /// The parameter count for a function was incorrect.
    InvalidParameterCount { function: String, count: usize },
}

utils::define_simple_error!(
    @main
    MathError:
    e => "math expression evaluation failed: {e:?}"
);

/// Fully evaluates an equation text.
fn eval_text(text: &[u8]) -> Result<f64> {
    let mut tokens = parse::tokenize(text);
    let expr = parse::read_expr(&mut tokens)?;
    eval::eval(expr)
}

#[cfg(test)]
mod test {
    use super::eval_text;

    macro_rules! is_correct {
        ($math:literal, $result:literal) => {{
            const MIN: f64 = $result - 0.001;
            const MAX: f64 = $result + 0.001;
            assert!(matches!(
                eval_text($math),
                Ok(MIN..=MAX)
            ));
        }};
    }

    #[test]
    fn success() {
        is_correct!(b"-4.5", -4.5);
        is_correct!(b"1 + 2 * 3", 7.0);
        is_correct!(b"1 + min(2) * 3", 7.0);
        is_correct!(b"sin(pi)", 0.0);
        is_correct!(b"min(2, max(-3, +5, 2), 21) * log(100, 10)", 4.0);
    }
}
