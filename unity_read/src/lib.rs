//! Allows reading in UnityFS archives, enumerating their files, and objects.
//!
//! Inspired and made by referencing https://github.com/gameltb/io_unity and https://github.com/yuanyan3060/unity-rs for file formats.

use std::{error::Error, fmt::{Debug, Display}};

pub mod classes;
pub mod object;
pub mod serialized_file;
mod unity_fs_common_str;
pub mod unity_fs;

#[derive(Debug, Clone)]
pub enum UnityError {
    UnexpectedEof,
    InvalidData(&'static str),
    Mismatch(UnityMismatch),
    Unsupported(&'static str)
}

#[derive(Debug, Clone)]
pub struct UnityMismatch {
    pub expected: String,
    pub received: String,
}

impl Display for UnityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for UnityError {}

#[macro_export]
macro_rules! read_endian {
    ($Type:ty, $endian:expr, $cursor:expr) => {
        if $endian {
            <$Type>::read_be($cursor)
        } else {
            <$Type>::read_le($cursor)
        }
    };
}
