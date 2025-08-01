use serenity::small_fixed_array::FixedString;

use crate::prelude::*;

pub mod components;

/// Creates a unicode [`ReactionType`] from a string with just the corresponding
/// unicode code symbol without allocating any memory.
///
/// No validation. I wish this could be const.
pub fn unicode_emoji(text: &'static str) -> ReactionType {
    // it is worth noting that `ReactionType::from` unconditionally allocates only
    // to throw the allocation away. it seems the compiler isn't quite smart enough
    // to eliminate it.
    // but this is useful even if it was smart enough to optimize that better since
    // some unicode emojis take up more than 1 char anyways.
    let text = FixedString::from_static_trunc(text);
    ReactionType::Unicode(text)
}

/// Checks whether two emojis are equivalent.
///
/// That is either:
/// - Both are custom emojis with the same ID.
/// - Both are identical unicode emoji.
pub fn emoji_equivalent(a: &ReactionType, b: &ReactionType) -> bool {
    use ReactionType as R;

    match (a, b) {
        (R::Custom { id: a_id, .. }, R::Custom { id: b_id, .. }) => a_id == b_id,
        (R::Unicode(a_name), R::Unicode(b_name)) => a_name == b_name,
        _ => false,
    }
}

pub fn guild_avatar_url(user_id: UserId, guild_id: GuildId, hash: &ImageHash) -> String {
    let ext = if hash.is_animated() { "gif" } else { "webp" };
    format!(
        "https://cdn.discordapp.com/guilds/{guild_id}/users/{user_id}/avatars/{hash}.{ext}?size=1024"
    )
}

pub fn is_normal_message(kind: MessageType) -> bool {
    matches!(kind, MessageType::Regular | MessageType::InlineReply)
}

pub fn is_user_message(message: &Message) -> bool {
    is_normal_message(message.kind) && !message.author.bot() && !message.author.system()
}

pub trait WithPartial {
    type Partial;
}

impl<'a, T: WithPartial> WithPartial for &'a T {
    type Partial = &'a T::Partial;
}

#[derive(Debug, Clone, Copy)]
pub enum Partial<T: WithPartial> {
    Full(T),
    Partial(T::Partial),
}

impl WithPartial for Member {
    type Partial = PartialMember;
}

/// Serializes a Discord ID as an [`u64`].
pub mod id_as_u64 {
    // LEB128 isn't really efficient for Discord IDs so circumvent that by encoding
    // them as byte arrays. we also need an override anyways because serenity tries
    // to deserialize them as any and that's no good.
    use serde::de::Error;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    pub(super) fn unpack<T, E>(int: [u8; 8]) -> Result<T, E>
    where
        T: From<u64>,
        E: Error,
    {
        let int = u64::from_le_bytes(int);
        if int != u64::MAX {
            Ok(T::from(int))
        } else {
            Err(E::custom("invalid discord id"))
        }
    }

    pub(super) fn pack<T>(val: T) -> [u8; 8]
    where
        T: Into<u64> + Copy,
    {
        let int: u64 = val.into();
        int.to_le_bytes()
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: From<u64>,
    {
        unpack(<[u8; 8]>::deserialize(deserializer)?)
    }

    pub fn serialize<S, T>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Into<u64> + Copy,
    {
        pack(*val).serialize(serializer)
    }
}

/// Serializes a Discord ID as an [`u64`].
pub mod option_id_as_u64 {
    // LEB128 isn't really efficient for Discord IDs so circumvent that by encoding
    // them as byte arrays. we also need an override anyways because serenity tries
    // to deserialize them as any and that's no good.
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: From<u64>,
    {
        match <Option<[u8; 8]>>::deserialize(deserializer)? {
            Some(value) => super::id_as_u64::unpack(value).map(Some),
            None => Ok(None),
        }
    }

    #[expect(clippy::ref_option)]
    pub fn serialize<S, T>(val: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Into<u64> + Copy,
    {
        val.map(super::id_as_u64::pack).serialize(serializer)
    }
}
