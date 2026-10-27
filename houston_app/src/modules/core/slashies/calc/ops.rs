use winnow::combinator::{alt, cond, opt, terminated};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::literal;

fn separator(input: &mut &str) -> ModalResult<()> {
    if input
        .chars()
        .next()
        .is_none_or(|c| c.is_ascii_whitespace() || c.is_ascii_punctuation())
    {
        Ok(())
    } else {
        Err(ErrMode::Backtrack(ContextError::new()))
    }
}

/// Helper macro to deduplicate code between different and within operator
/// kinds.
macro_rules! define_op_kind {
    {
        $(#[$attr:meta])*
        enum $Op:ident ($($par:tt)*) -> $Res:ty {
            $($name:ident $($lit:literal)|* => $fn:expr,)*
        }
    } => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy)]
        pub enum $Op {
            $($name,)*
        }

        impl $Op {
            /// Applies the operator to the given values.
            pub fn apply(self, $($par)*) -> $Res {
                match self {
                    $( Self::$name => $fn, )*
                }
            }

            pub fn parser(input: &mut &str) -> ModalResult<Self> {
                let needs_sep = opt(separator).parse_next(input)?.is_none();

                terminated(
                    alt(&mut [
                        $( alt(&mut [$(literal($lit)),*][..]).value(Self::$name), )*
                    ][..]),
                    cond(needs_sep, separator)
                )
                .parse_next(input)
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

#[derive(Debug, thiserror::Error)]
pub enum CallApplyError {
    #[error("expected {0} arguments")]
    IncorrectArgumentCount(usize),
}

define_op_kind! {
    /// A function to call.
    enum CallOp(values: &[f64]) -> Result<f64, CallApplyError> {
        Log "log" => {
            let &[a, b] = read_args(values)?;
            Ok(b.log(a))
        },
        Min "min" => Ok(fold_values(values, f64::min)),
        Max "max" => Ok(fold_values(values, f64::max)),
        Atan2 "atan2" => atan2_checked(values),
    }
}

fn read_args<const N: usize>(values: &[f64]) -> Result<&[f64; N], CallApplyError> {
    <&[f64; N]>::try_from(values).map_err(|_| CallApplyError::IncorrectArgumentCount(N))
}

fn fold_values(values: &[f64], f: impl FnMut(f64, f64) -> f64) -> f64 {
    values.iter().copied().reduce(f).unwrap_or(0.0)
}

fn atan2_checked(values: &[f64]) -> Result<f64, CallApplyError> {
    let &[a, b] = read_args(values)?;
    Ok(if a == 0.0 && b == 0.0 {
        f64::NAN
    } else {
        a.atan2(b)
    })
}
