use std::str::FromStr;

use crate::prelude::*;

/// Evaluates a mathematical equation.
#[poise::command(slash_command)]
pub async fn calc(
    ctx: HContext<'_>,
    mut equation: String,
) -> HResult {
    equation.make_ascii_lowercase();

    macro_rules! error_embed {
        ($($t:tt)*) => {
            CreateEmbed::new()
                .description(format!($($t)*))
                .color(ERROR_EMBED_COLOR)
        };
    }

    let embed = match eval_text(equation.as_bytes()) {
        Ok(result) => CreateEmbed::new()
            .description(format!("{equation} = **{result}**"))
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

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.text))
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
    function: Token<'a>,
    parameters: Vec<Expr<'a>>,
}

impl<'a> CallExpr<'a> {
    fn expr(self) -> Expr<'a> {
        Expr::Call(Box::new(self))
    }
}

#[derive(Debug)]
enum MathError {
    Internal,
    ExprExpected(Option<String>),
    InvalidNumber(String),
    InvalidUnaryOperator(String),
    InvalidBinaryOperator(String),
    InvalidFunction(String),
    InvalidParameterCount { function: String, count: usize },
}

utils::define_simple_error!(
    @main
    MathError:
    e => "math expression evaluation failed: {e:?}"
);

type Result<T> = std::result::Result<T, MathError>;

trait Tokenizer<'a> {
    fn next(&mut self) -> Option<Token<'a>>;
    fn last(&self) -> Option<Token<'a>>;

    fn expr_expected(&self) -> MathError {
        MathError::ExprExpected(self.last().map(|t| t.to_string()))
    }
}

fn eval_text(text: &[u8]) -> Result<f64> {
    let mut tokens = tokenize(text);
    let expr = read_expr(&mut tokens, &|t| t.is_none())?.expr;
    eval(expr)
}

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
        last: Option<Token<'a>>,
        iter: I,
    }

    impl<'a, I> Tokenizer<'a> for TokenizerImpl<'a, I>
    where
        I: Iterator<Item = Token<'a>> + ?Sized + 'a,
    {
        fn next(&mut self) -> Option<Token<'a>> {
            let value = self.iter.next();
            if value.is_some() {
                self.last = value;
            }
            value
        }

        fn last(&self) -> Option<Token<'a>> {
            self.last
        }
    }

    TokenizerImpl {
        last: None,
        iter,
    }
}

fn read_expr<'a>(tokens: &mut impl Tokenizer<'a>, terminate_on: &dyn Fn(Option<Token<'a>>) -> bool) -> Result<ExprSuccess<'a>> {
    struct ExprPair<'a> {
        value: Expr<'a>,
        operator: Option<BinaryOp>,
    }

    enum SubExpr<'a> {
        Expr(Expr<'a>),
        Name(Token<'a>),
    }

    fn read_sub_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<SubExpr<'a>> {
        let Some(token) = tokens.next() else {
            return Err(tokens.expr_expected());
        };

        fn number(f: f64) -> SubExpr<'static> {
            SubExpr::Expr(Expr::Number(f))
        }

        // reads a sub expression, like `5`, `-2`, or parenthised expressions
        Ok(match token.text {
            b"," | b")" => Err(MathError::ExprExpected(Some(token.to_string())))?,
            b"(" => SubExpr::Expr(read_expr(tokens, &|t| t == Some(Token::CLOSE))?.expr),
            b"+" => SubExpr::Expr(read_required_sub_expr(tokens)?),
            b"-" => SubExpr::Expr(UnaryOpExpr { kind: UnaryOp::Minus, operand: read_required_sub_expr(tokens)? }.expr()),
            b"pi" => number(std::f64::consts::PI),
            b"e" => number(std::f64::consts::E),
            b"tau" => number(std::f64::consts::TAU),
            [b'0'..=b'9', ..] => number({
                let s = std::str::from_utf8(token.text).map_err(|_| MathError::Internal)?;
                f64::from_str(s).map_err(|_| MathError::InvalidNumber(s.to_owned()))?
            }),
            _ => SubExpr::Name(token),
        })
    }

    fn read_required_sub_expr<'a>(tokens: &mut impl Tokenizer<'a>) -> Result<Expr<'a>> {
        read_sub_expr(tokens).and_then(|f| match f {
            SubExpr::Expr(expr) => Ok(expr),
            _ => Err(tokens.expr_expected()),
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
            _ => Err(MathError::InvalidBinaryOperator(token.to_string())),
        }
    }

    fn finish<'a>(tokens: &mut impl Tokenizer<'a>, mut pairs: Vec<ExprPair<'a>>) -> Result<Expr<'a>> {
        while pairs.len() > 1 {
            'merge_once: for index in 1..pairs.len() {
                let ([.., lhs], [rhs, ..]) = pairs.split_at_mut(index) else { unreachable!() };

                match lhs.operator {
                    // None should only be set for the last element
                    None => Err(MathError::InvalidBinaryOperator("<eol>".to_owned()))?,

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
            .ok_or_else(|| tokens.expr_expected())
    }

    let mut pairs = Vec::new();
    loop {
        // read sub expressions until out of tokens
        let mut value = read_sub_expr(tokens)?;
        'sub: loop {
            match (value, tokens.next()) {
                // a name is currently only valid for a call
                // in this case, the call becomes the sub expression
                (SubExpr::Name(method), Some(Token::OPEN)) => {
                    let mut parameters = Vec::new();
                    'params: loop {
                        let parameter = read_expr(tokens, &|t| matches!(t, Some(Token::CLOSE | Token::COMMA)))?;
                        parameters.push(parameter.expr);
                        if parameter.terminator == Some(Token::CLOSE) {
                            break 'params;
                        }
                    }

                    value = SubExpr::Expr(CallExpr {
                        function: method,
                        parameters,
                    }.expr());
                    continue 'sub;
                },

                // a name followed by anything else is an error.
                // assume it is supposed to be a unary operator.
                (SubExpr::Name(name), _) => {
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
                _ => return Err(tokens.expr_expected()),
            }
        }
    }
}

fn eval(expr: Expr) -> Result<f64> {
    fn eval_one(expr: CallExpr) -> Result<f64> {
        eval_many::<1>(expr).map(|r| r[0])
    }

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

    fn fold(exprs: Vec<Expr>, mut f: impl FnMut(f64, f64) -> f64) -> Result<f64> {
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
