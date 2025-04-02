use std::io;

use arrayvec::ArrayVec;
use utils::str_as_data::b20bit;

use super::{ButtonArgs, ButtonArgsRef};
use crate::prelude::*;

const STACK: usize = b20bit::max_byte_len(100);

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, STACK>;

/// Encodes a [`ButtonArgsRef`] as a custom ID.
pub fn to_custom_id(args: ButtonArgsRef<'_>) -> String {
    let mut buf = StackBuf::new();
    write_button_args(&mut buf, args);
    encode_custom_id(&buf)
}

/// Encodes a [`super::CustomData`] buffer as a custom ID.
pub fn encode_custom_id(slice: &[u8]) -> String {
    b20bit::to_string(slice)
}

/// Decodes a custom ID into a [`ButtonArgs`] with a buffer.
pub fn decode_custom_id<'v>(buf: &'v mut StackBuf, id: &str) -> Result<ButtonArgs<'v>> {
    b20bit::decode(&mut *buf, id)?;
    Ok(serde_steph::from_slice(buf)?)
}

/// Encodes a [`ButtonArgsRef`] into a buffer.
///
/// This logs errors instead of returning them.
pub fn write_button_args<W: io::Write>(writer: W, args: ButtonArgsRef<'_>) {
    if let Err(why) = serde_steph::to_writer(writer, &args) {
        log::error!("Error [{why:?}] serializing: {args:?}");
    }
}
