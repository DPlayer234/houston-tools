use std::str::FromStr;

use crate::prelude::*;

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
        Err(MathError::ExprExpected(Some(at))) => error_embed!("Expected expression at `{at}`."),
        Err(MathError::ExprExpected(None)) => error_embed!("Unexpected empty expression."),
        Err(MathError::InvalidNumber(num)) => error_embed!("`{num}` is not a valid number."),
        Err(MathError::InvalidUnaryOperator(op)) => error_embed!("`{op}` is not a unary operator."),
        Err(MathError::InvalidBinaryOperator(op)) => error_embed!("`{op}` is not a binary operator."),
        Err(MathError::InvalidFunction(function)) => error_embed!("The function `{function}` is unknown."),
        Err(MathError::InvalidParameterCount { function, count: 1 }) => error_embed!("The function `{function}` takes 1 parameter."),
        Err(MathError::InvalidParameterCount { function, count }) => error_embed!("The function `{function}` takes {count} parameters."),
        Err(r) => Err(r)?,
    };

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

/// A singular equation token, as returned by the tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Token<'a> {
    /// The token's actual text.
    text: &'a [u8],
}

impl Token<'static> {
    const OPEN: Token<'static> = Self::new(b"(");
    const CLOSE: Token<'static> = Self::new(b")");
    const COMMA: Token<'static> = Self::new(b",");
}

impl<'a> Token<'a> {
    const fn new(text: &'a [u8]) -> Self {
        Self { text }
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.text))
    }
}

/// Type of a successful [`read_expr`].
#[derive(Debug, Clone)]
struct ExprSuccess<'a> {
    /// The expression.
    expr: Expr<'a>,
    /// The token after the expression that terminated it.
    terminator: Option<Token<'a>>,
}

/// A tree representing a mathematical expression.
#[derive(Debug, Clone)]
enum Expr<'a> {
    /// A numeric constant.
    Number(f64),
    /// A binary operation.
    BinaryOp(Box<BinaryOpExpr<'a>>),
    /// A unary operation.
    UnaryOp(Box<UnaryOpExpr<'a>>),
    /// A function call.
    Call(Box<CallExpr<'a>>),
}

impl Default for Expr<'_> {
    fn default() -> Self {
        Self::Number(0.0)
    }
}

/// A binary operator kind.
#[derive(Debug, Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl BinaryOp {
    /// The priority for the operator.
    /// Relevant for order-of-operations.
    fn priority(self) -> isize {
        match self {
            BinaryOp::Add | BinaryOp::Sub => 1,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 2,
            BinaryOp::Pow => 3,
        }
    }
}

/// A binary operation expression.
#[derive(Debug, Clone)]
struct BinaryOpExpr<'a> {
    /// The operator to apply.
    kind: BinaryOp,
    /// The left-hand-side value.
    lhs: Expr<'a>,
    /// The right-hand-side value.
    rhs: Expr<'a>,
}

impl<'a> BinaryOpExpr<'a> {
    /// Wraps this value in an [`Expr`].
    fn expr(self) -> Expr<'a> {
        Expr::BinaryOp(Box::new(self))
    }
}

/// A unary operator kind.
#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Minus,
}

/// A unary operation expression.
#[derive(Debug, Clone)]
struct UnaryOpExpr<'a> {
    /// The operator to apply.
    kind: UnaryOp,
    /// The value.
    operand: Expr<'a>,
}

impl<'a> UnaryOpExpr<'a> {
    /// Wraps this value in an [`Expr`].
    fn expr(self) -> Expr<'a> {
        Expr::UnaryOp(Box::new(self))
    }
}

/// A function call expression.
#[derive(Debug, Clone)]
struct CallExpr<'a> {
    /// The function name token.
    function: Token<'a>,
    /// The provided parameters.
    parameters: Vec<Expr<'a>>,
}

impl<'a> CallExpr<'a> {
    /// Wraps this value in an [`Expr`].
    fn expr(self) -> Expr<'a> {
        Expr::Call(Box::new(self))
    }
}

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

/// A result for math evaluation.
type Result<T> = std::result::Result<T, MathError>;

/// A kind-of iterator for tokenizing.
///
/// This doesn't extend [`Iterator`] to reduce implementation code.
trait Tokenizer<'a> {
    /// Reads the next token, or [`None`] if exhausted.
    fn next(&mut self) -> Option<Token<'a>>;

    /// Returns the last token returned by [`Tokenizer::next`].
    fn last_token(&self) -> Option<Token<'a>>;

    /// Returns a [`MathError::ExprExpected`] matching the last token.
    fn expr_expected(&self) -> MathError {
        MathError::ExprExpected(self.last_token().map(|t| t.to_string()))
    }
}

/// Fully evaluates an equation text.
fn eval_text(text: &[u8]) -> Result<f64> {
    let mut tokens = tokenize(text);
    let expr = read_expr(&mut tokens, &|t| t.is_none())?.expr;
    eval(expr)
}

/// Returns an kind-of iterator to the tokens.
fn tokenize<'a>(text: &'a [u8]) -> impl Tokenizer<'a> {
    // - split by whitespace
    // - split each fragment by special characters, including them at the end of the new fragments
    // - split away the special characters also

    fn is_special_char(c: u8) -> bool {
        matches!(c, b'+' | b'-' | b'*' | b'/' | b'(' | b')' | b',')
    }

    let iter = text
        .split(|c| c.is_ascii_whitespace())
        .flat_map(|s| s.split_inclusive(|c| is_special_char(*c)))
        .flat_map(|s| match s.split_last() {
            Some((last, rest)) if is_special_char(*last) => std::iter::once(rest).chain(Some(std::slice::from_ref(last))),
            _ => std::iter::once(s).chain(None),
        })
        .filter(|s| !s.is_empty())
        .map(Token::new)
        .fuse();

    // this is only generic over `I` because we can't spell out the iterator name
    // and i don't want to box the iterator to be able to return the value
    struct TokenizerImpl<'a, I: ?Sized> {
        most_recent: Option<Token<'a>>,
        iter: I,
    }

    impl<'a, I> Tokenizer<'a> for TokenizerImpl<'a, I>
    where
        I: Iterator<Item = Token<'a>> + ?Sized + 'a,
    {
        fn next(&mut self) -> Option<Token<'a>> {
            let value = self.iter.next();
            if value.is_some() {
                self.most_recent = value;
            }
            value
        }

        fn last_token(&self) -> Option<Token<'a>> {
            self.most_recent
        }
    }

    TokenizerImpl {
        most_recent: None,
        iter,
    }
}

/// Reads an expression. This will consume `tokens` until it matches `terminate_on`
/// in a top-level binary-operator position.
///
/// If no more tokens are available before it finds the terminator, returns an error.
fn read_expr<'a>(
    tokens: &mut impl Tokenizer<'a>,
    terminate_on: &dyn Fn(Option<Token<'a>>) -> bool,
) -> Result<ExprSuccess<'a>> {
    /// Pair of an expression value and the following binary operator.
    struct ExprPair<'a> {
        value: Expr<'a>,
        operator: Option<BinaryOp>,
    }

    /// A sub-expression.
    enum SubExpr<'a> {
        Expr(Expr<'a>),
        Ident(Token<'a>),
    }

    /// Reads a "sub-expression", i.e. it gets the first expression without binary operators
    /// like `5` in `5 + 2`, an expression within parenthesis, unary operators with their operand,
    /// or an identifier.
    ///
    /// This function also resolves constants to their value.
    ///
    /// If no more tokens are available, returns an error.
    fn read_sub_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<SubExpr<'a>> {
        let Some(token) = tokens.next() else {
            return Err(tokens.expr_expected());
        };

        // this match *returns* for non-Expr branches
        let expr = match token.text {
            b"(" => read_expr(tokens, &|t| t == Some(Token::CLOSE))?.expr,
            b"+" => read_required_sub_expr(tokens)?,
            b"-" => match read_required_sub_expr(tokens)? {
                Expr::Number(n) => Expr::Number(-n),
                Expr::UnaryOp(op) if matches!(op.kind, UnaryOp::Minus) => op.operand,
                operand => UnaryOpExpr { kind: UnaryOp::Minus, operand }.expr()
            },
            b"pi" => Expr::Number(std::f64::consts::PI),
            b"e" => Expr::Number(std::f64::consts::E),
            b"tau" => Expr::Number(std::f64::consts::TAU),
            [b'0'..=b'9', ..] => Expr::Number({
                let s = std::str::from_utf8(token.text).map_err(|_| MathError::Internal)?;
                f64::from_str(s).map_err(|_| MathError::InvalidNumber(s.to_owned()))?
            }),
            b"," | b")" => return Err(MathError::ExprExpected(Some(token.to_string()))),
            _ => return Ok(SubExpr::Ident(token)),
        };

        Ok(SubExpr::Expr(expr))
    }

    /// Same as [`read_sub_expr`], but requires that it returns an expression and not anything else.
    fn read_required_sub_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<Expr<'a>> {
        match read_sub_expr(tokens)? {
            SubExpr::Expr(expr) => Ok(expr),
            _ => Err(tokens.expr_expected()),
        }
    }

    /// Maps a [`Token`] to a [`BinaryOp`].
    fn binary_op_kind(token: Token) -> Result<BinaryOp> {
        match token.text {
            b"+" => Ok(BinaryOp::Add),
            b"-" => Ok(BinaryOp::Sub),
            b"*" => Ok(BinaryOp::Mul),
            b"/" => Ok(BinaryOp::Div),
            b"%" | b"mod" => Ok(BinaryOp::Mod),
            b"^" | b"pow" => Ok(BinaryOp::Pow),
            _ => Err(MathError::InvalidBinaryOperator(token.to_string())),
        }
    }

    /// Finalizes a list of expression pairs into a singular expression.
    ///
    /// The `tokens` are only used for error reporting.
    fn finish<'a>(tokens: &mut impl Tokenizer<'a>, mut pairs: Vec<ExprPair<'a>>) -> Result<Expr<'a>> {
        while pairs.len() > 1 {
            // iterate over adjacent pairs (e.g. basically `pairs.windows(2)` but mutable).
            // the cell trick documented for `windows` could work, but it's harder to deal with and not any less code.
            'merge_once: for index in 1..pairs.len() {
                let ([.., lhs], [rhs, ..]) = pairs.split_at_mut(index) else { unreachable!() };

                match lhs.operator {
                    // None should only be set for the last element
                    None => Err(MathError::InvalidBinaryOperator("<eol>".to_owned()))?,

                    // merge cells if the left-hand priority is greater or equal than the right
                    // or if the right hand operator is None
                    Some(kind) if rhs.operator.map_or(true, |r| kind.priority() >= r.priority()) => {
                        use std::mem::take;

                        // move the values out since we'll need to put them elsewhere
                        let lhs_value = take(&mut lhs.value);
                        let rhs_value = take(&mut rhs.value);

                        // replace `lhs` with the new pair
                        *lhs = ExprPair {
                            value: BinaryOpExpr {
                                kind,
                                lhs: lhs_value,
                                rhs: rhs_value,
                            }.expr(),
                            operator: rhs.operator,
                        };

                        // remove `rhs` from the list entirely
                        // we can't do that earlier to get `rhs_value` because that would also invalidate `lhs`.
                        pairs.remove(index);

                        // restart the inner loop.
                        // this could start further in, but the logic for that is more difficult to get right.
                        break 'merge_once;
                    },

                    // other cases continue searching
                    _ => (),
                }
            }
        }

        pairs.into_iter().next()
            .map(|p| p.value)
            .ok_or_else(|| tokens.expr_expected())
    }

    let mut pairs = Vec::new();
    loop {
        // read sub expressions until out of tokens
        let mut value = read_sub_expr(tokens)?;

        // this loop is used as a pseudo-goto target
        'sub: loop {
            match (value, tokens.next()) {
                // a name is currently only valid for a call
                // in this case, the call becomes the sub expression
                (SubExpr::Ident(method), Some(Token::OPEN)) => {
                    let mut parameters = Vec::new();
                    'params: loop {
                        let parameter = read_expr(tokens, &|t| matches!(t, Some(Token::CLOSE | Token::COMMA)))?;
                        parameters.push(parameter.expr);

                        // we either read an expression followed by `,` or `)`.
                        // `,` would mean another parameter, `)` ends the parameter list.
                        if parameter.terminator == Some(Token::CLOSE) {
                            break 'params;
                        }
                    }

                    value = SubExpr::Expr(CallExpr {
                        function: method,
                        parameters,
                    }.expr());

                    // rerun the match with the new value
                    continue 'sub;
                },

                // a name followed by anything else is an error.
                // assume it is supposed to be a unary operator.
                (SubExpr::Ident(name), _) => {
                    return Err(MathError::InvalidUnaryOperator(name.to_string()));
                },

                // followed by terminator means this expression ends
                (SubExpr::Expr(expr), token) if terminate_on(token) => {
                    let expr = if !pairs.is_empty() {
                        pairs.push(ExprPair { value: expr, operator: None });
                        finish(tokens, pairs)?
                    } else {
                        expr
                    };

                    // we're done!
                    return Ok(ExprSuccess {
                        expr,
                        terminator: token,
                    });
                },

                // otherwise a binary operator follows
                (SubExpr::Expr(expr), Some(operator)) => {
                    pairs.push(ExprPair {
                        value: expr,
                        operator: Some(binary_op_kind(operator)?),
                    });

                    // read the next sub-expression
                    break 'sub;
                },

                // anything else is an error
                _ => return Err(tokens.expr_expected()),
            }
        }
    }
}

/// Evaluates an expression recursively.
fn eval(expr: Expr) -> Result<f64> {
    /// Evaluates a call's parameters, expecting one parameter.
    fn eval_one(expr: CallExpr) -> Result<f64> {
        eval_many::<1>(expr).map(|r| r[0])
    }

    /// Evaluates a call's parameters, expecting a certain amount.
    fn eval_many<const N: usize>(expr: CallExpr) -> Result<[f64; N]> {
        match <[Expr; N]>::try_from(expr.parameters) {
            Err(_) => Err(MathError::InvalidParameterCount {
                function: expr.function.to_string(),
                count: N,
            }),
            Ok(exprs) => {
                let mut result = [0f64; N];
                for (index, expr) in exprs.into_iter().enumerate() {
                    result[index] = eval(expr)?;
                }

                Ok(result)
            }
        }
    }

    /// Evaluates and folds an iterable of expressions.
    ///
    /// Returns `Ok(0.0)` if the iterable is empty.
    fn fold<'a>(
        exprs: impl IntoIterator<Item = Expr<'a>>,
        mut f: impl FnMut(f64, f64) -> f64,
    ) -> Result<f64> {
        exprs.into_iter()
            .map(eval)
            .reduce(|a, b| Ok(f(a?, b?)))
            .unwrap_or(Ok(0.0))
    }

    Ok(match expr {
        Expr::Number(num) => num,
        Expr::BinaryOp(expr) => {
            let lhs = eval(expr.lhs)?;
            let rhs = eval(expr.rhs)?;
            match expr.kind {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Div => lhs / rhs,
                BinaryOp::Mod => lhs % rhs,
                BinaryOp::Pow => lhs.powf(rhs),
            }
        },
        Expr::UnaryOp(expr) => {
            let value = eval(expr.operand)?;
            match expr.kind {
                UnaryOp::Minus => -value,
            }
        },
        Expr::Call(expr) => {
            let expr = *expr;
            match expr.function.text {
                b"abs" => eval_one(expr)?.abs(),
                b"sqrt" => eval_one(expr)?.sqrt(),
                b"sin" => eval_one(expr)?.sin(),
                b"cos" => eval_one(expr)?.cos(),
                b"tan" => eval_one(expr)?.tan(),
                b"asin" => eval_one(expr)?.asin(),
                b"acos" => eval_one(expr)?.acos(),
                b"atan" => eval_one(expr)?.atan(),
                b"log" => {
                    let [a, b] = eval_many::<2>(expr)?;
                    a.log(b)
                },
                b"min" => fold(expr.parameters, f64::min)?,
                b"max" => fold(expr.parameters, f64::max)?,
                _ => Err(MathError::InvalidFunction(expr.function.to_string()))?,
            }
        }
    })
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
        is_correct!(b"sin(pi)", 0.0);
        is_correct!(b"min(2, max(-3, +5, 2), 21) * log(100, 10)", 4.0);
    }
}
