use arrayvec::ArrayVec;
use serde::Serialize;
use serde_steph::de::{Deserializer, SliceRead};
use utils::str_as_data::b20bit;

use super::ButtonValue;
use crate::prelude::*;

const STACK: usize = b20bit::max_byte_len(100);

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, STACK>;

/// Provides a button buffer decoder.
pub type Decoder<'de> = Deserializer<SliceRead<'de>>;

/// Encodes a button value as a custom ID.
pub fn to_custom_id<T: ButtonValue + Serialize>(action: &T) -> String {
    let mut buf = StackBuf::new();
    write_inner_data(&mut buf, action);
    encode_custom_id(&buf)
}

/// Encodes a button buffer as a custom ID.
pub fn encode_custom_id(slice: &[u8]) -> String {
    b20bit::to_string(slice)
}

/// Decodes a custom ID into a button buffer.
pub fn decode_custom_id<'de>(buf: &'de mut StackBuf, id: &str) -> Result<Decoder<'de>> {
    b20bit::decode(&mut *buf, id)?;
    Ok(serde_steph::Deserializer::from_slice(buf))
}

/// Writes the inner data for a button value.
pub fn write_inner_data<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) {
    fn inner<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) -> Result {
        serde_steph::to_writer(&mut *buf, &T::ACTION_KEY)?;
        Ok(serde_steph::to_writer(buf, action)?)
    }

    if let Err(why) = inner(buf, action) {
        log::error!("Error serializing `{}`: [{why:?}]", T::ACTION_KEY);
    }
}
