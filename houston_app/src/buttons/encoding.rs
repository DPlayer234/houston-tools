use std::io;

use arrayvec::ArrayVec;
use smallvec::SmallVec;
use utils::str_as_data::b20bit;

use super::{ButtonArgs, ButtonArgsRef};
use crate::prelude::*;

const STACK: usize = b20bit::max_byte_len(100);

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, STACK>;

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
    b20bit::decode(&mut data, id)?;
    read_button_args(&data)
}

/// Encodes a [`super::CustomData`] buffer as a custom ID.
pub fn encode_custom_id(slice: &[u8]) -> String {
    b20bit::to_string(slice)
}

/// Reads a [`super::CustomData`] buffer as a [`ButtonArgs`].
pub fn read_button_args(slice: &[u8]) -> Result<ButtonArgs> {
    Ok(serde_steph::from_slice(slice)?)
}

/// Encodes a [`ButtonArgsRef`] into a buffer.
///
/// This logs errors instead of returning them.
pub fn write_button_args<W: io::Write>(mut writer: W, args: ButtonArgsRef<'_>) {
    if let Err(why) = serde_steph::to_writer(&mut writer, &args) {
        log::error!("Error [{why:?}] serializing: {args:?}");
    }
}
