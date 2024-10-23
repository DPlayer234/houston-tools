use super::{MathError, Result};
use super::parse::Token;

/// Helper macro to deduplicate code between different and within operator kinds.
macro_rules! define_op_kind {
    {
        $(#[$attr:meta])*
        enum $Op:ident $([$($g:tt)*])? ($($par:tt)*) -> $Res:ty {
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
            pub fn apply $(<$($g)*>)? (self, $($par)*) -> $Res {
                match self {
                    $( Self::$name => $fn, )*
                }
            }

            /// Tries to get an operator from a token.
            pub fn from_token(t: Token<'_>) -> Option<Self> {
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
    enum BinaryOp(lhs: f64, rhs: f64) -> f64 {
        Add "+" => lhs + rhs,
        Sub "-" => lhs - rhs,
        Mul "*" => lhs * rhs,
        Div "/" => lhs / rhs,
        Mod "%" | "mod" => lhs % rhs,
        Pow "^" | "pow" => lhs.powf(rhs),
    }
}

impl BinaryOp {
    /// The priority for the operator.
    /// Relevant for order-of-operations.
    pub const fn priority(self) -> isize {
        match self {
            Self::Add | Self::Sub => 1,
            Self::Mul | Self::Div | Self::Mod => 2,
            Self::Pow => 3,
        }
    }
}

define_op_kind! {
    /// A unary operator kind.
    enum UnaryOp(value: f64) -> f64 {
        Plus "+" => value,
        Minus "-" => -value,
        Abs "abs" => value.abs(),
        Sqrt "sqrt" => value.sqrt(),
        Sin "sin" => value.sin(),
        Cos "cos" => value.cos(),
        Tan "tan" => value.tan(),
        Asin "asin" => value.asin(),
        Acos "acos" => value.acos(),
        Atan "atan" => value.atan(),
        Ln "ln" => value.ln(),
        Exp "exp" => value.exp(),
    }
}

define_op_kind! {
    /// A function to call.
    enum CallOp['a](fn_name: Token<'a>, values: &[f64]) -> Result<'a, f64> {
        Log "log" => {
            let &[a, b] = read_args(values, fn_name)?;
            Ok(a.log(b))
        },
        Min "min" => Ok(fold_values(values, f64::min)),
        Max "max" => Ok(fold_values(values, f64::max)),
    }
}

fn read_args<'v, 'n, const N: usize>(
    values: &'v [f64],
    fn_name: Token<'n>,
) -> Result<'n, &'v [f64; N]> {
    <&[f64; N]>::try_from(values)
        .map_err(|_| MathError::InvalidParameterCount {
            function: fn_name,
            count: N
        })
}

fn fold_values(
    values: &[f64],
    f: impl FnMut(f64, f64) -> f64,
) -> f64 {
    values.iter()
        .copied()
        .reduce(f)
        .unwrap_or(0.0)
}
