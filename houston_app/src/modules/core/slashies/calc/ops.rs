use super::parse::Token;
use super::{MathError, Result};

/// Helper macro to deduplicate code between different and within operator
/// kinds.
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
        SinH "sinh" => value.sinh(),
        CosH "cosh" => value.cosh(),
        TanH "tanh" => value.tanh(),
        Asin "asin" => value.asin(),
        Acos "acos" => value.acos(),
        Atan "atan" => value.atan(),
        AsinH "asinh" => value.asinh(),
        AcosH "acosh" => value.acosh(),
        AtanH "atanh" => value.atanh(),
        Ln "ln" => value.ln(),
        Log10 "log10" => value.log10(),
        Exp "exp" => value.exp(),
        Floor "floor" => value.floor(),
        Ceil "ceil" => value.ceil(),
        Round "round" => value.round_ties_even(),
        Trunc "trunc" => value.trunc(),
    }
}

define_op_kind! {
    /// A post-fix unary operator.
    enum PostUnaryOp(value: f64) -> f64 {
        Factorial "!" => factorial(value),
    }
}

fn factorial(mut value: f64) -> f64 {
    if value == f64::INFINITY {
        value
    } else if value <= 0.0 || value % 1.0 != 0.0 {
        f64::NAN
    } else {
        let mut acc = value;
        while value > 1.0 {
            value -= 1.0;
            acc *= value;

            if !acc.is_finite() {
                break;
            }
        }

        acc
    }
}

define_op_kind! {
    /// A function to call.
    enum CallOp['a](fn_name: Token<'a>, values: &[f64]) -> Result<'a, f64> {
        Log "log" => {
            let &[a, b] = read_args(values, fn_name)?;
            Ok(b.log(a))
        },
        Min "min" => Ok(fold_values(values, f64::min)),
        Max "max" => Ok(fold_values(values, f64::max)),
        Atan2 "atan2" => atan2_checked(fn_name, values),
    }
}

fn read_args<'v, 'n, const N: usize>(
    values: &'v [f64],
    fn_name: Token<'n>,
) -> Result<'n, &'v [f64; N]> {
    <&[f64; N]>::try_from(values).map_err(|_| MathError::InvalidParameterCount {
        function: fn_name,
        count: N,
    })
}

fn fold_values(values: &[f64], f: impl FnMut(f64, f64) -> f64) -> f64 {
    values.iter().copied().reduce(f).unwrap_or(0.0)
}

fn atan2_checked<'a>(fn_name: Token<'a>, values: &[f64]) -> Result<'a, f64> {
    let &[a, b] = read_args(values, fn_name)?;
    Ok(if a == 0.0 && b == 0.0 {
        f64::NAN
    } else {
        a.atan2(b)
    })
}
