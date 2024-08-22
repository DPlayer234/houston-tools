use std::str::FromStr;

use super::{MathError, Result};
use super::ops::*;

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
    value: f64,
    /// The token after the expression that terminated it.
    terminator: Option<Token<'a>>,
}

/// Pair of a value and the following binary operator.
struct ValuePair {
    value: f64,
    operator: Option<BinaryOp>,
}

/// A kind-of iterator for tokenizing.
///
/// This doesn't extend [`Iterator`] to reduce implementation code.
pub trait Tokenizer<'a> {
    /// Reads the next token, or [`None`] if exhausted.
    fn next(&mut self) -> Option<Token<'a>>;

    fn peek(&mut self) -> Option<Token<'a>>;

    /// Returns the last token returned by [`Tokenizer::next`].
    fn last_token(&self) -> Option<Token<'a>>;

    /// Returns a [`MathError::ExprExpected`] matching the last token.
    fn expr_expected(&self) -> MathError<'a> {
        MathError::ExprExpected(self.last_token())
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
        peeked: Option<Token<'a>>,
        iter: I,
    }

    impl<'a, I> Tokenizer<'a> for TokenizerImpl<'a, I>
    where
        I: Iterator<Item = Token<'a>> + ?Sized,
    {
        fn next(&mut self) -> Option<Token<'a>> {
            let value = self.peeked.take().or_else(|| self.iter.next());
            if value.is_some() {
                self.most_recent = value;
            }

            value
        }

        fn peek(&mut self) -> Option<Token<'a>> {
            if self.peeked.is_none() {
                self.peeked = self.iter.next();
            }

            self.peeked
        }

        fn last_token(&self) -> Option<Token<'a>> {
            self.most_recent
        }
    }

    TokenizerImpl {
        most_recent: None,
        peeked: None,
        iter,
    }
}

/// Reads an expression. This will consume `tokens` until the end.
pub fn read_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<'a, f64> {
    read_expr_with_terminator(tokens, |t| t.is_none()).map(|e| e.value)
}

/// Reads an expression. This will consume `tokens` until it matches `terminate_on`
/// in a top-level binary-operator position.
///
/// If no more tokens are available before it finds the terminator, returns an error.
fn read_expr_with_terminator<'a>(
    tokens: &mut impl Tokenizer<'a>,
    terminate_on: fn(Option<Token<'a>>) -> bool,
) -> Result<'a, ExprSuccess<'a>> {
    // this is basically the only place where this allocates now
    let mut pairs = Vec::new();
    loop {
        // read sub expressions until out of tokens
        let value = read_sub_expr(tokens)?;
        let token = tokens.next();

        // if this a terminator, finish the expression and return it
        if terminate_on(token) {
            let value = if !pairs.is_empty() {
                pairs.push(ValuePair { value, operator: None });
                merge_expr_pairs(tokens, pairs)?
            } else {
                value
            };

            // we're done!
            return Ok(ExprSuccess {
                value,
                terminator: token,
            });
        }

        let Some(operator) = token else {
            // a non-terminating None is an error
            return Err(tokens.expr_expected());
        };

        // expecting a binary operator here
        let operator = BinaryOp::from_token(operator)
            .ok_or_else(|| MathError::InvalidBinaryOperator(operator))?;

        pairs.push(ValuePair {
            value,
            operator: Some(operator),
        });
    }
}

/// Reads a "sub-expression", i.e. it gets the first expression without binary operators
/// like `5` in `5 + 2`, an expression within parenthesis, unary operators with their operand,
/// or an identifier.
///
/// If no more tokens are available, returns an error.
fn read_sub_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<'a, f64> {
    let Some(token) = tokens.next() else {
        return Err(tokens.expr_expected());
    };

    // this match *returns* for non-Expr branches
    let expr = match token.text {
        // start of parenthesis around child-expression
        b"(" => read_expr_with_terminator(tokens, |t| t == Some(Token::CLOSE))?.value,

        // constants
        b"pi" => std::f64::consts::PI,
        b"e" => std::f64::consts::E,
        b"tau" => std::f64::consts::TAU,

        // anything starting with a digit is assumed to be a number
        [b'0'..=b'9', ..] => {
            let s = std::str::from_utf8(token.text).map_err(|_| MathError::Internal)?;
            f64::from_str(s).map_err(|_| MathError::InvalidNumber(token))?
        },

        // these shouldn't show up here
        b"," | b")" => return Err(MathError::ExprExpected(Some(token))),

        // lastly, also check for unary operators and functions
        _ => if let Some(op) = UnaryOp::from_token(token) {
            op.apply(read_sub_expr(tokens)?)
        } else if let Some(call) = CallOp::from_token(token) {
            read_call(tokens, call, token)?
        } else if matches!(tokens.peek(), Some(Token::OPEN)) {
            return Err(MathError::InvalidFunction(token));
        } else if matches!(tokens.peek(), Some(_)) {
            return Err(MathError::InvalidUnaryOperator(token));
        } else {
            return Err(MathError::ExprExpected(tokens.last_token()));
        }
    };

    Ok(expr)
}

/// Reads the parameters for a function call and evaluates it.
///
/// This also checks that the next token is `(`.
fn read_call<'a>(tokens: &mut impl Tokenizer<'a>, call_fn: CallOp, call_fn_token: Token<'a>) -> Result<'a, f64> {
    if !matches!(tokens.next(), Some(Token::OPEN)) {
        return Err(MathError::FunctionCallExpected(call_fn_token));
    }

    let terminate_on = |t| matches!(t, Some(Token::CLOSE | Token::COMMA));
    let mut params = Vec::new();

    if tokens.peek() == Some(Token::CLOSE) {
        // empty argument list. pop `)`
        tokens.next();
    } else {
        // otherwise terminate when we hit a close in a terminator position
        loop {
            let res = read_expr_with_terminator(tokens, terminate_on)?;
            params.push(res.value);
            if res.terminator == Some(Token::CLOSE) {
                break;
            }
        }
    }

    call_fn.apply(&params)
}

/// Merges a list of expression pairs into a singular expression.
///
/// The `tokens` are only used for error reporting.
fn merge_expr_pairs<'a>(tokens: &mut impl Tokenizer<'a>, mut pairs: Vec<ValuePair>) -> Result<'a, f64> {
    while pairs.len() > 1 {
        // iterate over adjacent pairs (e.g. basically `pairs.windows(2)` but mutable).
        // the cell trick documented for `windows` could work, but it's harder to deal with and not any less code.
        'merge_once: for index in 1..pairs.len() {
            let ([.., lhs], [rhs, ..]) = pairs.split_at_mut(index) else { unreachable!() };

            match lhs.operator {
                // None should only be set for the last element
                None => Err(MathError::InvalidBinaryOperator(Token::new(b"<eol>")))?,

                // merge cells if the left-hand priority is greater or equal than the right
                // or if the right hand operator is None
                Some(kind) if rhs.operator.map_or(true, |r| kind.priority() >= r.priority()) => {
                    // copy the values out since we'll need to put them elsewhere
                    let lhs_value = lhs.value;
                    let rhs_value = rhs.value;

                    // replace `lhs` with the new pair
                    *lhs = ValuePair {
                        value: kind.apply(lhs_value, rhs_value),
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
