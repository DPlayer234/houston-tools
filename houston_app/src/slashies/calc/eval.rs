use super::{MathError, Result};
use super::ast::*;

/// Evaluates an expression recursively.
pub fn eval(expr: Expr) -> Result<f64> {
    Ok(match expr {
        Expr::Number(num) => num,
        Expr::BinaryOp(expr) => expr.kind.apply(
            eval(expr.lhs)?,
            eval(expr.rhs)?,
        ),
        Expr::UnaryOp(expr) => expr.kind.apply(
            eval(expr.operand)?
        ),
        Expr::Call(expr) => eval_call(expr)?,
    })
}

fn eval_call(expr: Box<CallExpr>) -> Result<f64> {
    Ok(match expr.function.text {
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
        b"min" => fold_exprs(expr.parameters, f64::min)?,
        b"max" => fold_exprs(expr.parameters, f64::max)?,
        _ => Err(MathError::InvalidFunction(expr.function.to_string()))?,
    })
}

/// Evaluates a call's parameters, expecting one parameter.
fn eval_one(expr: Box<CallExpr>) -> Result<f64> {
    eval_many::<1>(expr).map(|r| r[0])
}

/// Evaluates a call's parameters, expecting a certain amount.
fn eval_many<const N: usize>(expr: Box<CallExpr>) -> Result<[f64; N]> {
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
fn fold_exprs<'a>(
    exprs: impl IntoIterator<Item = Expr<'a>>,
    mut f: impl FnMut(f64, f64) -> f64,
) -> Result<f64> {
    exprs.into_iter()
        .map(eval)
        .reduce(|a, b| Ok(f(a?, b?)))
        .unwrap_or(Ok(0.0))
}
