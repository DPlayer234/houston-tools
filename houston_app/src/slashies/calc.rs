use std::str::FromStr;

use crate::prelude::*;

/// Evaluates a mathematical equation.
#[poise::command(slash_command)]
pub async fn calc(
    ctx: HContext<'_>,
    mut equation: String,
) -> HResult {
    equation.make_ascii_lowercase();

    let mut tokens = tokenize(equation.as_bytes());
    let expr = read_expr(&mut tokens, &|t| t.is_none())?.expr;
    let result = eval(expr)?;

    let embed = CreateEmbed::new()
        .description(format!(
            "{equation} = **{result}**"
        ))
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Token<'a> {
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

#[derive(Debug, Clone)]
struct ExprSuccess<'a> {
    expr: Expr<'a>,
    terminator: Option<Token<'a>>,
}

#[derive(Debug, Clone)]
enum Expr<'a> {
    Number(f64),
    BinaryOp(Box<BinaryOpExpr<'a>>),
    UnaryOp(Box<UnaryOpExpr<'a>>),
    Call(Box<CallExpr<'a>>),
}

impl Default for Expr<'_> {
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
struct BinaryOpExpr<'a> {
    kind: BinaryOp,
    lhs: Expr<'a>,
    rhs: Expr<'a>,
}

impl<'a> BinaryOpExpr<'a> {
    fn expr(self) -> Expr<'a> {
        Expr::BinaryOp(Box::new(self))
    }
}

#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Minus,
}

#[derive(Debug, Clone)]
struct UnaryOpExpr<'a> {
    kind: UnaryOp,
    operand: Expr<'a>,
}

impl<'a> UnaryOpExpr<'a> {
    fn expr(self) -> Expr<'a> {
        Expr::UnaryOp(Box::new(self))
    }
}

#[derive(Debug, Clone)]
struct CallExpr<'a> {
    method: Token<'a>,
    parameters: Vec<Expr<'a>>,
}

impl<'a> CallExpr<'a> {
    fn expr(self) -> Expr<'a> {
        Expr::Call(Box::new(self))
    }
}

#[derive(Debug)]
enum MathErrorKind {
    ParseFailed,
    ExprExpected,
    InvalidBinaryOperand,
    InvalidMethod,
    InvalidParameterCount,
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
        .map(Token::new)
        .fuse()
}

fn read_expr<'a>(tokens: &mut dyn Iterator<Item = Token<'a>>, terminate_on: &dyn Fn(Option<Token<'a>>) -> bool) -> Result<ExprSuccess<'a>> {
    struct ExprPair<'a> {
        value: Expr<'a>,
        operator: Option<BinaryOp>,
    }

    enum SubExpr<'a> {
        Expr(Expr<'a>),
        Name(Token<'a>),
    }

    fn read_sub_expr<'a>(tokens: &mut dyn Iterator<Item = Token<'a>>) -> Result<Option<SubExpr<'a>>> {
        let Some(token) = tokens.next() else {
            return Ok(None);
        };

        fn number(f: f64) -> Option<SubExpr<'static>> {
            Some(SubExpr::Expr(Expr::Number(f)))
        }

        // reads a sub expression, like `5`, `-2`, or parenthised expressions
        Ok(match token.text {
            b"(" => Some(SubExpr::Expr(read_expr(tokens, &|t| t == Some(Token::CLOSE))?.expr)),
            b"+" => Some(SubExpr::Expr(read_required_sub_expr(tokens)?)),
            b"-" => Some(SubExpr::Expr(UnaryOpExpr { kind: UnaryOp::Minus, operand: read_required_sub_expr(tokens)? }.expr())),
            b"pi" => number(std::f64::consts::PI),
            b"e" => number(std::f64::consts::E),
            b"tau" => number(std::f64::consts::TAU),
            [b'0'..=b'9', ..] => number(
                std::str::from_utf8(token.text).ok()
                    .and_then(|s| f64::from_str(s).ok())
                    .ok_or_else(|| MathError(MathErrorKind::ParseFailed))?
            ),
            _ => Some(SubExpr::Name(token)),
        })
    }

    fn read_required_sub_expr<'a>(tokens: &mut dyn Iterator<Item = Token<'a>>) -> Result<Expr<'a>> {
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
                (SubExpr::Name(method), Some(Token::OPEN)) => {
                    let mut parameters = Vec::new();
                    loop {
                        let parameter = read_expr(tokens, &|t| matches!(t, Some(Token::CLOSE | Token::COMMA)))?;
                        parameters.push(parameter.expr);
                        if parameter.terminator == Some(Token::CLOSE) {
                            break;
                        }
                    }

                    value = SubExpr::Expr(CallExpr {
                        method,
                        parameters,
                    }.expr());
                },

                // followed by terminator means this expression ends
                (SubExpr::Expr(expr), token) if terminate_on(token) => {
                    let expr = if !pairs.is_empty() {
                        pairs.push(ExprPair { value: expr, operator: None });
                        finish(pairs)?
                    } else {
                        expr
                    };

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
                    break 'sub;
                },

                // anything else is an error
                _ => break 'main
            }
        }
    }

    Err(MathError(MathErrorKind::ParseFailed))
}

fn eval(expr: Expr) -> Result<f64> {
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
            match expr.method.text {
                b"abs" => eval_one(expr.parameters)?.abs(),
                b"sqrt" => eval_one(expr.parameters)?.sqrt(),
                b"sin" => eval_one(expr.parameters)?.sin(),
                b"cos" => eval_one(expr.parameters)?.cos(),
                b"tan" => eval_one(expr.parameters)?.tan(),
                b"asin" => eval_one(expr.parameters)?.asin(),
                b"acos" => eval_one(expr.parameters)?.acos(),
                b"atan" => eval_one(expr.parameters)?.atan(),
                b"log" => {
                    let [a, b] = eval_many::<2>(expr.parameters)?;
                    a.log(b)
                },
                _ => Err(MathError(MathErrorKind::InvalidMethod))?,
            }
        }
    })
}

fn eval_one(exprs: Vec<Expr>) -> Result<f64> {
    match <[Expr; 1]>::try_from(exprs) {
        Err(_) => Err(MathError(MathErrorKind::InvalidParameterCount)),
        Ok([expr]) => {
            eval(expr)
        }
    }
}

fn eval_many<const N: usize>(exprs: Vec<Expr>) -> Result<[f64; N]> {
    match <[Expr; N]>::try_from(exprs) {
        Err(_) => Err(MathError(MathErrorKind::InvalidParameterCount)),
        Ok(exprs) => {
            let mut result = [0f64; N];
            for (index, expr) in exprs.into_iter().enumerate() {
                result[index] = eval(expr)?;
            }

            Ok(result)
        }
    }
}
