use winnow::ModalResult;
use winnow::ascii::{alpha1, alphanumeric0, digit1, multispace0};
use winnow::combinator::{
    alt, cut_err, delimited, dispatch, empty, eof, fail, opt, peek, preceded, repeat,
    separated_pair, terminated,
};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::one_of;

use super::ops::*;

/// Parses and evaluates an expression.
pub fn evaluate(input: &str) -> Result<f64, ParseError<&str, ContextError>> {
    terminated(expr, eof.context(expected_desc("binary operator"))).parse(input)
}

/// Pair of a value and the following binary operator.
struct ValuePair {
    value: f64,
    operator: Option<BinaryOp>,
}

/// Parses the literal, providing expected context.
fn ch<'a>(literal: char) -> impl Parser<&'a str, char, ErrMode<ContextError>> {
    literal.context(StrContext::Expected(literal.into()))
}

/// Returns a [`StrContext::Expected`] with the given description.
fn expected_desc(desc: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(desc))
}

/// Parses a full expression.
///
/// `<sub_expr BinaryOp>* sub_expr`
fn expr(input: &mut &str) -> ModalResult<f64> {
    fn finalize_value((mut pairs, last): (Vec<ValuePair>, f64)) -> f64 {
        if pairs.is_empty() {
            last
        } else {
            pairs.push(ValuePair {
                value: last,
                operator: None,
            });
            merge_expr_pairs(pairs)
        }
    }

    fn value_pair(input: &mut &str) -> ModalResult<ValuePair> {
        (sub_expr, BinaryOp::parser)
            .map(|(value, op)| ValuePair {
                value,
                operator: Some(op),
            })
            .parse_next(input)
    }

    (repeat(0.., value_pair), sub_expr)
        .map(finalize_value)
        .parse_next(input)
}

/// Parses a unary operator expression.
///
/// `UnaryOp sub_expr`
fn unary_op_expr(input: &mut &str) -> ModalResult<f64> {
    (UnaryOp::parser, cut_err(sub_expr))
        .map(|(op, val)| op.apply(val))
        .parse_next(input)
}

/// Parses function call arguments.
///
/// `expr <, expr>*`
fn call_args(input: &mut &str) -> ModalResult<Vec<f64>> {
    let mut end_at = opt(peek(')'));
    if end_at.parse_next(input)?.is_some() {
        return Ok(Vec::new());
    }

    let mut args = Vec::new();
    loop {
        let item = expr.parse_next(input)?;

        args.push(item);
        if end_at.parse_next(input)?.is_some() {
            break Ok(args);
        }

        _ = ','
            .context(StrContext::Expected(','.into()))
            .context(StrContext::Expected(')'.into()))
            .parse_next(input)?;
    }
}

/// Parses a function call.
///
/// `CallOp "(" call_args ")"`
fn call_expr(input: &mut &str) -> ModalResult<f64> {
    let syntax_check = (alpha1, alphanumeric0, multispace0, '(');
    let call_parser = separated_pair(
        CallOp::parser.context(StrContext::Label("function name")),
        multispace0,
        delimited(ch('('), call_args, ch(')')),
    );

    let (op, args) = preceded(peek(syntax_check), cut_err(call_parser)).parse_next(input)?;
    cut_err(empty.try_map(|()| op.apply(&args))).parse_next(input)
}

/// Parses a floating-point number literal.
///
/// `digit1 [. digit1] [<"e" | "E"> [<"+" | "-">] digit1]`
fn float(input: &mut &str) -> ModalResult<f64> {
    (
        digit1,
        opt(('.', cut_err(digit1))),
        opt((one_of(('e', 'E')), opt(one_of(('+', '-'))), cut_err(digit1))),
    )
        .take()
        .parse_to()
        .parse_next(input)
}

/// Parses a constant.
///
/// `<"pi" | "e" | "tau">`
fn constant(input: &mut &str) -> ModalResult<f64> {
    use std::f64::consts::{E, PI, TAU};

    dispatch!((alpha1, alphanumeric0).take();
        "pi" => empty.value(PI),
        "e" => empty.value(E),
        "tau" => empty.value(TAU),
        _ => cut_err(fail),
    )
    .parse_next(input)
}

/// Parses a "sub-expression", i.e. it gets the first expression without binary
/// operators like `5` in `5 + 2`, an expression within parenthesis, unary
/// operators with their operand, function calls, or constants.
///
/// `< <"(" expr ")"> | unary_op_expr | call_expr | float | constant >`
fn sub_expr(input: &mut &str) -> ModalResult<f64> {
    (
        delimited(
            multispace0,
            // note: this is partially order-sensitive since most of the parsers here will cut_err
            // when they internally decide that other parsers shouldn't apply
            alt((
                preceded('(', cut_err(terminated(expr, ch(')')))),
                unary_op_expr,
                call_expr,
                float.context(StrContext::Label("incomplete number literal")),
                constant.context(StrContext::Label("constant")),
                fail.context(expected_desc("expression")),
            )),
            multispace0,
        ),
        terminated(opt(PostUnaryOp::parser), multispace0),
    )
        .map(|(val, op)| op.map(|op| op.apply(val)).unwrap_or(val))
        .parse_next(input)
}

/// Merges a list of expression pairs into a singular expression.
///
/// # Panics
///
/// Panics if `pairs` is empty or any operator other than the last is [`None`].
fn merge_expr_pairs(mut pairs: Vec<ValuePair>) -> f64 {
    while pairs.len() > 1 {
        // iterate over adjacent pairs (e.g. basically `pairs.windows(2)` but mutable).
        // the cell trick documented for `windows` could work, but it's harder to deal
        // with and both more and more complex code.
        let mut iter = 0usize..;
        'merge_once: while let Some(index) = iter.next()
            && let Some([lhs, rhs, ..]) = pairs.get_mut(index..)
        {
            // None is only set for the last element
            let kind = lhs.operator.expect("only last operator must be empty");

            // merge cells if the left-hand priority is greater or equal than the right
            // or if the right hand operator is None
            if rhs.operator.is_none_or(|r| kind.priority() >= r.priority()) {
                // copy the values out since we'll need to put them elsewhere
                let lhs_value = lhs.value;
                let rhs_value = rhs.value;

                // replace `lhs` with the new pair
                *lhs = ValuePair {
                    value: kind.apply(lhs_value, rhs_value),
                    operator: rhs.operator,
                };

                // remove `rhs` from the list entirely
                // we can't do that earlier to get `rhs_value` because that would also
                // invalidate `lhs`.
                pairs.remove(index + 1);

                // restart the inner loop.
                // this could start further in, but the logic for that is more difficult to get
                // right.
                break 'merge_once;
            }

            // other cases continue searching
            // this cannot lead to an infinite loop: if nothing else merges, the
            // last 2 pairs get merged
        }
    }

    // pairs must not be empty
    pairs.pop().expect("the pairs must not be empty").value
}
