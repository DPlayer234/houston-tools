use super::{MathError, Result};
use super::parse::Token;

/// Helper macro to deduplicate code between different and within operator kinds.
macro_rules! define_op_kind {
    {
        $(#[$attr:meta])*
        enum $Op:ident ($($par:ident: $Par:ty),*) -> $Res:ty {
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
            pub fn apply(self, $($par: $Par),*) -> $Res {
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
    enum BinaryOp(lhs: f64, rhs: f64) -> f64 {
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
    pub const fn priority(self) -> isize {
        match self {
            BinaryOp::Add | BinaryOp::Sub => 1,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 2,
            BinaryOp::Pow => 3,
        }
    }
}

define_op_kind! {
    /// A unary operator kind.
    enum UnaryOp(value: f64) -> f64 {
        Plus b"+" => value,
        Minus b"-" => -value,
        Abs b"abs" => value.abs(),
        Sqrt b"sqrt" => value.sqrt(),
        Sin b"sin" => value.sin(),
        Cos b"cos" => value.cos(),
        Tan b"tan" => value.tan(),
        Asin b"asin" => value.asin(),
        Acos b"acos" => value.acos(),
        Atan b"atan" => value.atan(),
        Ln b"ln" => value.ln(),
        Exp b"exp" => value.exp(),
    }
}

define_op_kind! {
    /// A function to call.
    enum CallOp(values: &[f64]) -> Result<'static, f64> {
        Log b"log" => {
            let [a, b] = read_args(values, b"log")?;
            Ok(a.log(b))
        },
        Min b"min" => Ok(fold_values(values, f64::min)),
        Max b"max" => Ok(fold_values(values, f64::max)),
    }
}

fn read_args<'a, const N: usize>(
    values: &[f64],
    fn_name: &'a [u8],
) -> Result<'a, [f64; N]> {
    match <&[f64; N]>::try_from(values) {
        Ok(slice) => Ok(*slice),
        _ => Err(MathError::InvalidParameterCount { function: Token::new(fn_name), count: N })
    }
}

fn fold_values(
    values: &[f64],
    f: impl FnMut(f64, f64) -> f64,
) -> f64 {
    values.into_iter()
        .copied()
        .reduce(f)
        .unwrap_or(0.0)
}
