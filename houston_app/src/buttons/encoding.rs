use arrayvec::ArrayVec;
use serde::{Deserialize, Serialize};
use serde_steph::de::{Deserializer, SliceRead};
use utils::str_as_data::b20bit;

use super::ButtonValue;
use crate::prelude::*;

const STACK: usize = b20bit::max_byte_len(100);

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, STACK>;

/// Allows decoding a button action value.
pub struct Decoder<'de>(Deserializer<SliceRead<'de>>);

impl<'de> Decoder<'de> {
    /// Read the button action key.
    pub(super) fn read_key(&mut self) -> Result<usize> {
        Ok(usize::deserialize(&mut self.0)?)
    }

    /// Deserializes the button value.
    pub fn into_button_value<T: Deserialize<'de>>(mut self) -> Result<T> {
        let value = T::deserialize(&mut self.0)?;
        self.0.end()?;
        Ok(value)
    }
}

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
    Ok(Decoder(Deserializer::from_slice(buf)))
}

/// Writes the inner data for a button value.
pub fn write_inner_data<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) {
    use serde_steph::{Error, Result, to_writer};

    #[inline]
    fn inner<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) -> Result<()> {
        to_writer(&mut *buf, &T::ACTION_KEY)?;
        to_writer(buf, action)
    }

    #[cold]
    fn log_error(why: Error, key: usize) {
        log::error!("Error serializing `{key}`: {why}");
    }

    if let Err(why) = inner(buf, action) {
        log_error(why, T::ACTION_KEY);
    }
}
