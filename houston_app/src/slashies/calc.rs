use std::str::FromStr;

use crate::prelude::*;

/// Evaluates a mathematical equation.
#[poise::command(slash_command)]
pub async fn calc(
    ctx: HContext<'_>,
    equation: String,
) -> HResult {
    let mut tokens = tokenize(equation.as_bytes());
    let expr = read_expr(&mut tokens)?;
    let result = eval(expr);

    let embed = CreateEmbed::new()
        .description(format!(
            "{equation} = **{result}**"
        ))
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Token<'a> {
    text: &'a [u8],
}

#[derive(Debug, Clone)]
enum Expr {
    Number(f64),
    BinaryOp(Box<BinaryOpExpr>),
    UnaryOp(Box<UnaryOpExpr>),
    Call(Box<CallExpr>),
}

impl Default for Expr {
    fn default() -> Self {
        Self::Number(0.0)
    }
}

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
    fn priority(self) -> isize {
        match self {
            BinaryOp::Add | BinaryOp::Sub => 1,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 2,
            BinaryOp::Pow => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct BinaryOpExpr {
    kind: BinaryOp,
    lhs: Expr,
    rhs: Expr,
}

impl BinaryOpExpr {
    fn expr(self) -> Expr {
        Expr::BinaryOp(Box::new(self))
    }
}

#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Minus,
}

#[derive(Debug, Clone)]
struct UnaryOpExpr {
    kind: UnaryOp,
    operand: Expr,
}

impl UnaryOpExpr {
    fn expr(self) -> Expr {
        Expr::UnaryOp(Box::new(self))
    }
}

#[derive(Debug, Clone, Copy)]
enum CallMethod {
    Abs,
    Sqrt,
}

#[derive(Debug, Clone)]
struct CallExpr {
    method: CallMethod,
    parameter: Expr,
}

impl CallExpr {
    fn expr(self) -> Expr {
        Expr::Call(Box::new(self))
    }
}

#[derive(Debug)]
enum MathErrorKind {
    ParseFailed,
    ExprExpected,
    InvalidBinaryOperand,
    InvalidMethod,
}

utils::define_simple_error!(
    #[allow(dead_code)]
    MathError(MathErrorKind):
    e => "cannot do math: {e:?}"
);

type Result<T> = std::result::Result<T, MathError>;

fn tokenize<'a>(text: &'a [u8]) -> impl Iterator<Item = Token<'a>> {
    // - split by whitespace
    // - split each fragment by special characters, including them at the end of the new fragments
    // - split away the special characters also

    fn is_special_char(c: u8) -> bool {
        matches!(c, b'+' | b'-' | b'*' | b'/' | b'(' | b')' | b',')
    }

    text.split(|c| c.is_ascii_whitespace())
        .flat_map(|s| s.split_inclusive(|c| is_special_char(*c)))
        .flat_map(|s| match s.split_last() {
            Some((last, rest)) if is_special_char(*last) => std::iter::once(rest).chain(Some(std::slice::from_ref(last))),
            _ => std::iter::once(s).chain(None),
        })
        .filter(|s| !s.is_empty())
        .map(|s| Token { text: s })
        .fuse()
}

fn read_expr<'a>(tokens: &mut impl Iterator<Item = Token<'a>>) -> Result<Expr> {
    struct ExprPair {
        value: Expr,
        operator: Option<BinaryOp>,
    }

    enum SubExpr<'a> {
        Expr(Expr),
        Name(Token<'a>),
    }

    fn read_sub_expr<'a>(tokens: &mut impl Iterator<Item = Token<'a>>) -> Result<Option<SubExpr<'a>>> {
        let Some(token) = tokens.next() else {
            return Ok(None);
        };

        // reads a sub expression, like `5`, `-2`, or parenthised expressions
        Ok(match token.text {
            b"(" => Some(SubExpr::Expr(read_expr(tokens)?)),
            b"+" => Some(SubExpr::Expr(read_required_sub_expr(tokens)?)),
            b"-" => Some(SubExpr::Expr(UnaryOpExpr { kind: UnaryOp::Minus, operand: read_required_sub_expr(tokens)? }.expr())),
            [b'0'..=b'9', ..] => Some(SubExpr::Expr(Expr::Number(
                std::str::from_utf8(token.text).ok()
                    .and_then(|s| f64::from_str(s).ok())
                    .ok_or_else(|| MathError(MathErrorKind::ParseFailed))?
            ))),
            _ => Some(SubExpr::Name(token)),
        })
    }

    fn read_required_sub_expr<'a>(tokens: &mut impl Iterator<Item = Token<'a>>) -> Result<Expr> {
        read_sub_expr(tokens).and_then(|f| match f {
            Some(SubExpr::Expr(expr)) => Ok(expr),
            _ => Err(MathError(MathErrorKind::ExprExpected)),
        })
    }

    fn binary_op_kind(token: Token) -> Result<BinaryOp> {
        match token.text {
            b"+" => Ok(BinaryOp::Add),
            b"-" => Ok(BinaryOp::Sub),
            b"*" => Ok(BinaryOp::Mul),
            b"/" => Ok(BinaryOp::Div),
            b"%" | b"mod" => Ok(BinaryOp::Mod),
            b"^" | b"pow" => Ok(BinaryOp::Pow),
            _ => Err(MathError(MathErrorKind::InvalidBinaryOperand)),
        }
    }

    fn finish(mut pairs: Vec<ExprPair>) -> Result<Expr> {
        while pairs.len() > 1 {
            'merge_once: for index in 1..pairs.len() {
                let ([.., lhs], [rhs, ..]) = pairs.split_at_mut(index) else { unreachable!() };

                match lhs.operator {
                    // None should only be set for the last element
                    None => Err(MathError(MathErrorKind::InvalidBinaryOperand))?,

                    // merge cells if the left-hand priority is greater or equal than the right
                    // or if the right hand operator is None
                    Some(kind) if rhs.operator.map(|r| kind.priority() >= r.priority()).unwrap_or(true) => {
                        use std::mem::take;
                        let lhs_value = take(&mut lhs.value);
                        let rhs_value = take(&mut rhs.value);

                        *lhs = ExprPair {
                            value: BinaryOpExpr {
                                kind,
                                lhs: lhs_value,
                                rhs: rhs_value,
                            }.expr(),
                            operator: rhs.operator,
                        };

                        pairs.remove(index);
                        break 'merge_once;
                    },

                    // other cases continue searching
                    _ => (),
                }
            }
        }

        pairs.into_iter().next()
            .map(|p| p.value)
            .ok_or_else(|| MathError(MathErrorKind::ExprExpected))
    }

    let mut pairs = Vec::new();

    // read sub expressions until out of tokens
    'main: while let Some(mut value) = read_sub_expr(tokens)? {
        'sub: loop {
            match (value, tokens.next()) {
                // a name is currently only valid for a call
                // in this case, the call becomes the sub expression
                (SubExpr::Name(name), Some(Token { text: b"(" })) => {
                    let method = match name.text {
                        b"abs" => CallMethod::Abs,
                        b"sqrt" => CallMethod::Sqrt,
                        _ => return Err(MathError(MathErrorKind::InvalidMethod)),
                    };

                    let parameter = read_expr(tokens)?;
                    value = SubExpr::Expr(CallExpr {
                        method,
                        parameter,
                    }.expr());
                },

                // followed by nothing or ")" means this is the end of the expression
                (SubExpr::Expr(expr), None | Some(Token { text: b")" })) => {
                    if pairs.is_empty() {
                        return Ok(expr);
                    }

                    pairs.push(ExprPair { value: expr, operator: None });
                    return finish(pairs);
                },

                // otherwise a binary operator follows
                (SubExpr::Expr(expr), Some(operator)) => {
                    pairs.push(ExprPair {
                        value: expr,
                        operator: Some(binary_op_kind(operator)?),
                    });
                    break 'sub;
                },

                // anything else is an error
                _ => break 'main
            }
        }
    }

    Err(MathError(MathErrorKind::ParseFailed))
}

fn eval(expr: Expr) -> f64 {
    match expr {
        Expr::Number(num) => num,
        Expr::BinaryOp(expr) => {
            let lhs = eval(expr.lhs);
            let rhs = eval(expr.rhs);
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
            let value = eval(expr.operand);
            match expr.kind {
                UnaryOp::Minus => -value,
            }
        },
        Expr::Call(expr) => {
            let value = eval(expr.parameter);
            match expr.method {
                CallMethod::Abs => value.abs(),
                CallMethod::Sqrt => value.sqrt(),
            }
        }
    }
}
