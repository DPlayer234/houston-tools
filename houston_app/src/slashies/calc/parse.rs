use std::str::FromStr;

use super::{MathError, Result};
use super::ast::*;

/// A singular equation token, as returned by the tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    /// The token's actual text.
    pub text: &'a [u8],
}

impl Token<'static> {
    const OPEN: Token<'static> = Self::new(b"(");
    const CLOSE: Token<'static> = Self::new(b")");
    const COMMA: Token<'static> = Self::new(b",");
}

impl<'a> Token<'a> {
    pub const fn new(text: &'a [u8]) -> Self {
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

/// A kind-of iterator for tokenizing.
///
/// This doesn't extend [`Iterator`] to reduce implementation code.
pub trait Tokenizer<'a> {
    /// Reads the next token, or [`None`] if exhausted.
    fn next(&mut self) -> Option<Token<'a>>;

    /// Returns the last token returned by [`Tokenizer::next`].
    fn last_token(&self) -> Option<Token<'a>>;

    /// Returns a [`MathError::ExprExpected`] matching the last token.
    fn expr_expected(&self) -> MathError {
        MathError::ExprExpected(self.last_token().map(|t| t.to_string()))
    }
}

/// Returns an kind-of iterator to the tokens.
pub fn tokenize<'a>(text: &'a [u8]) -> impl Tokenizer<'a> {
    // - split by whitespace
    // - split each fragment by special characters, including them at the end of the new fragments
    // - split away the special characters also

    fn is_special_char(c: u8) -> bool {
        matches!(c, b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'(' | b')' | b',')
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

/// Reads an expression. This will consume `tokens` until the end.
pub fn read_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<Expr<'a>> {
    read_expr_with_terminator(tokens, &|t| t.is_none()).map(|e| e.expr)
}

/// Reads an expression. This will consume `tokens` until it matches `terminate_on`
/// in a top-level binary-operator position.
///
/// If no more tokens are available before it finds the terminator, returns an error.
fn read_expr_with_terminator<'a>(
    tokens: &mut impl Tokenizer<'a>,
    terminate_on: &dyn Fn(Option<Token<'a>>) -> bool,
) -> Result<ExprSuccess<'a>> {
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
                    let terminate_on = &|t| matches!(t, Some(Token::CLOSE | Token::COMMA));

                    let mut parameters = Vec::new();
                    'params: loop {
                        let parameter = read_expr_with_terminator(tokens, terminate_on)?;
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
                        merge_expr_pairs(tokens, pairs)?
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
                (SubExpr::Expr(value), Some(operator)) => {
                    let operator = BinaryOp::from_token(operator)
                        .ok_or_else(|| MathError::InvalidBinaryOperator(operator.to_string()))?;

                    pairs.push(ExprPair {
                        value,
                        operator: Some(operator),
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
        // start of parenthesis around child-expression
        b"(" => read_expr_with_terminator(tokens, &|t| t == Some(Token::CLOSE))?.expr,

        // constants
        b"pi" => Expr::Number(std::f64::consts::PI),
        b"e" => Expr::Number(std::f64::consts::E),
        b"tau" => Expr::Number(std::f64::consts::TAU),

        // anything starting with a digit is assumed to be a number
        [b'0'..=b'9', ..] => Expr::Number({
            let s = std::str::from_utf8(token.text).map_err(|_| MathError::Internal)?;
            f64::from_str(s).map_err(|_| MathError::InvalidNumber(s.to_owned()))?
        }),

        // these shouldn't show up here
        b"," | b")" => return Err(MathError::ExprExpected(Some(token.to_string()))),

        // lastly, also check for unary operators
        _ => match UnaryOp::from_token(token) {
            // not matched, return an `Ident`.
            None => return Ok(SubExpr::Ident(token)),

            // otherwise, read the following expression as the operand
            // also fold it directly into numbers to reduce allocations
            Some(kind) => match read_required_sub_expr(tokens)? {
                Expr::Number(n) => Expr::Number(kind.apply(n)),
                operand => UnaryOpExpr { kind, operand }.expr()
            }
        }
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

/// Merges a list of expression pairs into a singular expression.
///
/// The `tokens` are only used for error reporting.
fn merge_expr_pairs<'a>(tokens: &mut impl Tokenizer<'a>, mut pairs: Vec<ExprPair<'a>>) -> Result<Expr<'a>> {
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
                        value: into_binary_expr(kind, lhs_value, rhs_value),
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
                // this cannot lead to an infinite loop: if nothing else merges, the last 2 pairs get merged
                _ => (),
            }
        }
    }

    pairs.into_iter().next()
        .map(|p| p.value)
        .ok_or_else(|| tokens.expr_expected())
}

/// Creates a binary operator expression.
///
/// If both operands are numbers, simplifies it to just a number expression.
fn into_binary_expr<'a>(kind: BinaryOp, lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a> {
    match (lhs, rhs) {
        (Expr::Number(lhs), Expr::Number(rhs)) => Expr::Number(kind.apply(lhs, rhs)),
        (lhs, rhs) => BinaryOpExpr { kind, lhs, rhs }.expr(),
    }
}
