use std::str::FromStr;

use super::{MathError, Result};
use super::ops::*;

/// A singular equation token, as returned by the tokenizer.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    /// The token's actual text.
    pub text: &'a str,
    /// The index within the tokenized full text.
    pub token_index: usize,
}

impl<'a> Token<'a> {
    /// Gets an object that can be used to print error information.
    pub fn error_fmt(self) -> TokenErrorFmt<'a> {
        TokenErrorFmt { token: self }
    }
}

/// Checks whether an `Option<Token>` has the given text.
macro_rules! matches_token {
    ($e:expr, $p:pat) => {
        match $e {
            Some(Token { text: $p, .. }) => true,
            _ => false,
        }
    };
}

pub struct TokenErrorFmt<'a> {
    token: Token<'a>,
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.text)
    }
}

impl std::fmt::Display for TokenErrorFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n-# Position: {}", self.token.token_index + 1)
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
pub fn tokenize<'a>(text: &'a str) -> impl Tokenizer<'a> {
    // - split by whitespace
    // - split each fragment by special characters, including them at the end of the new fragments
    // - split away the special characters also

    fn is_special_char(c: u8) -> bool {
        // Note: each of these must be an ASCII character
        matches!(c, b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'(' | b')' | b',')
    }

    unsafe fn token_from_utf8<'a>(token_index: usize, bytes: &'a [u8]) -> Token<'a> {
        debug_assert!(std::str::from_utf8(bytes).is_ok());

        // SAFETY: only splitting on ASCII characters
        let text = unsafe { std::str::from_utf8_unchecked(bytes) };
        Token { text, token_index }
    }

    let iter = text.as_bytes()
        .split(|c| c.is_ascii_whitespace())
        .flat_map(|s| s.split_inclusive(|c| is_special_char(*c)))
        .flat_map(|s| match s.split_last() {
            Some((last, rest)) if is_special_char(*last) => std::iter::once(rest).chain(Some(std::slice::from_ref(last))),
            _ => std::iter::once(s).chain(None),
        })
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, s)| unsafe { token_from_utf8(i, s) });

    // this is only generic over `I` because we can't spell out the iterator name
    // and i don't want to box the iterator to be able to return the value
    struct TokenizerImpl<'a, I> {
        most_recent: Option<Token<'a>>,
        peeked: Option<Token<'a>>,
        iter: I,
    }

    impl<'a, I> Tokenizer<'a> for TokenizerImpl<'a, I>
    where
        I: Iterator<Item = Token<'a>>,
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
    // this is the main place where this allocates. the other is function parameters
    // the arrayvec crate could prevent that, but it might also need a lot of stack space
    // do note that this function may be called recursively!
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
    let expr = match token.text.as_bytes() {
        // start of parenthesis around child-expression
        b"(" => read_expr_with_terminator(tokens, |t| matches_token!(t, ")"))?.value,

        // constants
        b"pi" => std::f64::consts::PI,
        b"e" => std::f64::consts::E,
        b"tau" => std::f64::consts::TAU,

        // anything starting with a digit is assumed to be a number
        [b'0'..=b'9', ..] => f64::from_str(token.text).map_err(|_| MathError::InvalidNumber(token))?,

        // these shouldn't show up here
        b"," | b")" => return Err(MathError::ExprExpected(Some(token))),

        // lastly, also check for unary operators and functions
        _ => if let Some(op) = UnaryOp::from_token(token) {
            op.apply(read_sub_expr(tokens)?)
        } else if let Some(call) = CallOp::from_token(token) {
            read_call(tokens, call, token)?
        } else if matches_token!(tokens.peek(), "(") {
            return Err(MathError::InvalidFunction(token));
        } else if tokens.peek().is_some() {
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
    if !matches_token!(tokens.next(), "(") {
        return Err(MathError::FunctionCallExpected(call_fn_token));
    }

    let terminate_on = |t| matches_token!(t, ")" | ",");
    let mut params = Vec::new();

    if matches_token!(tokens.peek(), ")") {
        // empty argument list. pop `)`
        tokens.next();
    } else {
        // otherwise terminate when we hit a close in a terminator position
        loop {
            let res = read_expr_with_terminator(tokens, terminate_on)?;
            params.push(res.value);
            if matches_token!(res.terminator, ")") {
                break;
            }
        }
    }

    call_fn.apply(call_fn_token, &params)
}

/// Merges a list of expression pairs into a singular expression.
///
/// The `tokens` are only used for error reporting.
fn merge_expr_pairs<'a>(tokens: &mut impl Tokenizer<'a>, mut pairs: Vec<ValuePair>) -> Result<'a, f64> {
    while pairs.len() > 1 {
        // iterate over adjacent pairs (e.g. basically `pairs.windows(2)` but mutable).
        // the cell trick documented for `windows` could work, but it's harder to deal with and not any less code.
        'merge_once: for index in 0..(pairs.len() - 1) {
            let [lhs, rhs, ..] = &mut pairs[index..] else { unreachable!() };

            match lhs.operator {
                // None should only be set for the last element
                None => Err(MathError::Internal)?,

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
                    pairs.remove(index + 1);

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
