use std::fmt;

pub use const_builder::ConstBuilder;

use crate::{FluentInt, FluentStr};

pub struct Unset;

impl fmt::Display for Unset {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl FluentInt for Unset {
    fn to_switch_value(&self) -> i8 {
        i8::MAX
    }
}

impl FluentStr for Unset {
    fn to_switch_value(&self) -> &str {
        ""
    }
}
