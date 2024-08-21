use super::parse::Token;

/// A tree representing a mathematical expression.
#[derive(Debug, Clone)]
pub enum Expr<'a> {
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
    /// Returns the numeric value 0.
    fn default() -> Self {
        Self::Number(0.0)
    }
}

/// Helper macro to deduplicate code between different and within operator kinds.
macro_rules! define_op_kind {
    {
        $(#[$attr:meta])*
        enum $Op:ident ($($par:ident: $Par:ty),*) {
            $($name:ident $lit:pat => $fn:expr,)*
        }
    } => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy)]
        pub enum $Op {
            $($name,)*
        }

        impl $Op {
            /// Applies the operator to the given values.
            pub fn apply(self, $($par: $Par),*) -> f64 {
                match self {
                    $( Self::$name => $fn, )*
                }
            }

            /// Tries to get an operator from a token.
            pub fn from_token(t: Token) -> Option<Self> {
                match t.text {
                    $( $lit => Some(Self::$name), )*
                    _ => None,
                }
            }
        }
    };
}

define_op_kind! {
    /// A binary operator kind.
    enum BinaryOp(lhs: f64, rhs: f64) {
        Add b"+" => lhs + rhs,
        Sub b"- "=> lhs - rhs,
        Mul b"*" => lhs * rhs,
        Div b"/" => lhs / rhs,
        Mod b"%" | b"mod" => lhs % rhs,
        Pow b"^" | b"pow" => lhs.powf(rhs),
    }
}

impl BinaryOp {
    /// The priority for the operator.
    /// Relevant for order-of-operations.
    pub fn priority(self) -> isize {
        match self {
            BinaryOp::Add | BinaryOp::Sub => 1,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 2,
            BinaryOp::Pow => 3,
        }
    }
}

/// A binary operation expression.
#[derive(Debug, Clone)]
pub struct BinaryOpExpr<'a> {
    /// The operator to apply.
    pub kind: BinaryOp,
    /// The left-hand-side value.
    pub lhs: Expr<'a>,
    /// The right-hand-side value.
    pub rhs: Expr<'a>,
}

impl<'a> BinaryOpExpr<'a> {
    /// Wraps this value in an [`Expr`].
    pub fn expr(self) -> Expr<'a> {
        Expr::BinaryOp(Box::new(self))
    }
}

define_op_kind! {
    /// A unary operator kind.
    enum UnaryOp(value: f64) {
        Plus b"+" => value,
        Minus b"-" => -value,
    }
}

/// A unary operation expression.
#[derive(Debug, Clone)]
pub struct UnaryOpExpr<'a> {
    /// The operator to apply.
    pub kind: UnaryOp,
    /// The value.
    pub operand: Expr<'a>,
}

impl<'a> UnaryOpExpr<'a> {
    /// Wraps this value in an [`Expr`].
    pub fn expr(self) -> Expr<'a> {
        Expr::UnaryOp(Box::new(self))
    }
}

/// A function call expression.
#[derive(Debug, Clone)]
pub struct CallExpr<'a> {
    /// The function name token.
    pub function: Token<'a>,
    /// The provided parameters.
    pub parameters: Vec<Expr<'a>>,
}

impl<'a> CallExpr<'a> {
    /// Wraps this value in an [`Expr`].
    pub fn expr(self) -> Expr<'a> {
        Expr::Call(Box::new(self))
    }
}
