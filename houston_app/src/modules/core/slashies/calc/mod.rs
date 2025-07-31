use parse::Token;

use crate::slashies::prelude::*;

mod ops;
mod parse;

/// Evaluates a mathematical equation. Warning: Floating point math.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn calc(
    ctx: Context<'_>,
    /// The expression to evaluate.
    #[max_length = 3000]
    expression: &str,
    /// Whether to show the response only to yourself.
    mut ephemeral: Option<bool>,
) -> anyhow::Result<()> {
    let expression = expression.to_ascii_lowercase();

    macro_rules! calc_error {
        ($($t:tt)*) => {{
            ephemeral = Some(true);
            (format!($($t)*), ERROR_EMBED_COLOR)
        }};
    }

    let (content, color) = match eval_text(&expression) {
        Ok(result) => (
            format!("{expression} = **{result}**"),
            ctx.data_ref().config().embed_color,
        ),

        Err(MathError::ExprExpected(Some(at))) => {
            calc_error!("Expected expression at `{at}`.{}", at.error_fmt())
        },

        Err(MathError::ExprExpected(None)) => calc_error!("Unexpected empty expression."),

        Err(MathError::InvalidNumber(num)) => {
            calc_error!("`{num}` is not a valid number.{}", num.error_fmt())
        },

        Err(MathError::InvalidUnaryOperator(op)) => {
            calc_error!("`{op}` is not a unary operator.{}", op.error_fmt())
        },

        Err(MathError::InvalidBinaryOperator(op)) => {
            calc_error!("`{op}` is not a binary operator.{}", op.error_fmt())
        },

        Err(MathError::InvalidFunction(function)) => {
            calc_error!(
                "The function `{function}` is unknown.{}",
                function.error_fmt(),
            )
        },

        Err(MathError::InvalidParameterCount { function, count: 1 }) => {
            calc_error!(
                "The function `{function}` takes 1 parameter.{}",
                function.error_fmt(),
            )
        },

        Err(MathError::InvalidParameterCount { function, count }) => calc_error!(
            "The function `{function}` takes {count} parameters.{}",
            function.error_fmt(),
        ),

        Err(MathError::FunctionCallExpected(function)) => calc_error!(
            "`{function}` is a function and requires `(...)` after it.{}",
            function.error_fmt(),
        ),
    };

    let components = components_array![CreateTextDisplay::new(content)];
    let components = components_array![CreateContainer::new(&components).accent_color(color)];

    let reply = create_reply(ephemeral)
        .components_v2(&components)
        .allowed_mentions(CreateAllowedMentions::new());

    ctx.send(reply).await?;
    Ok(())
}

/// A result for math evaluation.
type Result<'a, T> = std::result::Result<T, MathError<'a>>;

/// The kinds of errors that may occur when evaluating a mathematical
/// expression.
#[derive(Debug)]
enum MathError<'a> {
    /// A sub-expression was expected but not found.
    /// Holds the last token before the error.
    ExprExpected(Option<Token<'a>>),

    /// Found a token that seemed to be a number but couldn't be parsed as one.
    /// Holds the token in question.
    InvalidNumber(Token<'a>),

    /// Found a token that should be a unary operator but wasn't valid.
    /// Holds the token in question.
    InvalidUnaryOperator(Token<'a>),

    /// Found a token in a binary operator position that wasn't valid.
    /// Holds the token in question.
    InvalidBinaryOperator(Token<'a>),

    /// Encountered a call with an invalid function name.
    /// Holds the function name in question.
    InvalidFunction(Token<'a>),

    /// The parameter count for a function was incorrect.
    InvalidParameterCount { function: Token<'a>, count: usize },

    /// Expected a function call.
    FunctionCallExpected(Token<'a>),
}

/// Fully evaluates an equation text.
fn eval_text(text: &str) -> Result<'_, f64> {
    let mut tokens = parse::tokenize(text);
    parse::read_expr(&mut tokens)
}

#[cfg(test)]
mod tests {
    use super::eval_text;

    macro_rules! is_correct {
        ($math:literal, $result:literal) => {{
            const MIN: f64 = $result - 0.001;
            const MAX: f64 = $result + 0.001;
            let text = $math;
            let res = eval_text(text);
            assert!(
                matches!(res, Ok(MIN..=MAX)),
                "`{text:?}` not in `{MIN}..={MAX}`, was {res:?}"
            );
        }};
    }

    #[test]
    fn success() {
        is_correct!("-4.5", -4.5);
        is_correct!("1 + 2 * 3", 7.0);
        is_correct!("1 + min(2) * 3", 7.0);
        is_correct!("sin(pi)", 0.0);
        is_correct!("min(2, max(-3, +5, 2), 21) * log(10, 100)", 4.0);
        is_correct!("min()", 0.0);
        is_correct!("1--2", 3.0);
    }
}
