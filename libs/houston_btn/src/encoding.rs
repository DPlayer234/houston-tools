//! Defines and implements the encoding format for button custom IDs.

// abstracting this away into a trait is pretty hard, unless we carry a generic
// parameter for the event handler, nav, and everything else for that matter.
// also would make generating navs and custom ids harder since they are now tied
// to state provided by the infrastructure of this crate.
// additionally, the current code expects that you can read/write a usize
// followed by the actual data, which is not necessarily possible with all
// serialization formats (i.e. JSON) so even that approach would need to be
// abstracted away.

use arrayvec::ArrayVec;
use serde::{Deserialize, Serialize};
use serde_steph::de::{Deserializer, SliceRead};
use utils::str_as_data::b20bit;

use crate::{ButtonValue, Result};

const STACK: usize = b20bit::max_byte_len(100);

/// Buffer used for on-stack coding.
pub type StackBuf = ArrayVec<u8, STACK>;

/// Allows decoding a button action value.
#[derive(Debug)]
pub struct Decoder<'de>(Deserializer<SliceRead<'de>>);

impl<'de> Decoder<'de> {
    /// Read the button action key.
    ///
    /// # Errors
    ///
    /// Returns `Err` if deserializing the key failed.
    pub fn read_key(&mut self) -> Result<usize> {
        Ok(usize::deserialize(&mut self.0)?)
    }

    /// Deserializes the button value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if deserializing the value failed.
    pub fn into_button_value<T: Deserialize<'de>>(mut self) -> Result<T> {
        let value = T::deserialize(&mut self.0)?;
        self.0.end()?;
        Ok(value)
    }
}

/// Encodes a button buffer as a custom ID.
pub fn encode_custom_id(slice: &[u8]) -> String {
    b20bit::to_string(slice)
}

/// Decodes a custom ID into a button buffer.
///
/// # Errors
///
/// Returns `Err` if decoding `id` failed.
pub fn decode_custom_id<'de>(buf: &'de mut StackBuf, id: &str) -> Result<Decoder<'de>> {
    b20bit::decode(&mut *buf, id)?;
    Ok(Decoder(Deserializer::from_slice(buf)))
}

/// Writes the inner data for a button value.
pub fn write_inner_data<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) {
    use serde_steph::{Error, Result, to_writer};

    #[inline]
    fn inner<T: ButtonValue + Serialize>(buf: &mut StackBuf, action: &T) -> Result<()> {
        to_writer(&mut *buf, const { &T::ACTION.key })?;
        to_writer(buf, action)
    }

    #[cold]
    #[inline(never)]
    fn log_error(why: Error, key: usize) {
        log::error!("Error serializing `{key}`: {why}");
    }

    if let Err(why) = inner(buf, action) {
        log_error(why, const { T::ACTION.key });
    }
}
