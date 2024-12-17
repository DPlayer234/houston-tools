use std::io;

use arrayvec::ArrayVec;
use smallvec::SmallVec;

use super::{ButtonArgs, ButtonArgsRef};
use crate::prelude::*;

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, 200>;

/// Buffer used for coding when inline size is important.
pub type Buf = SmallVec<[u8; 16]>;

/// Encodes a [`ButtonArgsRef`] as a custom ID.
pub fn to_custom_id(args: ButtonArgsRef<'_>) -> String {
    let mut buf = StackBuf::new();
    write_button_args(&mut buf, args);
    encode_custom_id(&buf)
}

/// Decodes a [`ButtonArgs`] from a custom ID.
pub fn from_custom_id(id: &str) -> Result<ButtonArgs> {
    let mut data = StackBuf::new();
    utils::str_as_data::decode_b65536(&mut data, id)?;
    read_button_args(&data)
}

/// Encodes a [`super::CustomData`] buffer as a custom ID.
pub fn encode_custom_id(slice: &[u8]) -> String {
    utils::str_as_data::to_b65536(slice)
}

/// Reads a [`super::CustomData`] buffer as a [`ButtonArgs`].
pub fn read_button_args(slice: &[u8]) -> Result<ButtonArgs> {
    Ok(serde_bare::from_slice(slice)?)
}

/// Encodes a [`ButtonArgsRef`] into a buffer.
///
/// This logs errors instead of returning them.
pub fn write_button_args<W: io::Write>(mut writer: W, args: ButtonArgsRef<'_>) {
    if let Err(why) = serde_bare::to_writer(&mut writer, &args) {
        log::error!("Error [{why:?}] serializing: {args:?}");
    }
}
