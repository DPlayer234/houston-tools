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

/// Marker type to use with [`serde_with`] in place of the ID type to
/// serialize Discord IDs as fixed-length 8-byte values.
pub enum IdBytes {}

trait CastU64: From<u64> + Into<u64> + Copy {}
impl<T: From<u64> + Into<u64> + Copy> CastU64 for T {}

mod impl_id_bytes {
    // LEB128 isn't really efficient for Discord IDs so circumvent that by encoding
    // them as byte arrays. we also need an override anyways because serenity tries
    // to deserialize them as any and that's no good.
    use serde::de::Error as _;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    use super::{CastU64, IdBytes};

    impl<T: CastU64> SerializeAs<T> for IdBytes {
        fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let val = *source;
            let int: u64 = val.into();
            int.to_le_bytes().serialize(serializer)
        }
    }

    impl<'de, T: CastU64> DeserializeAs<'de, T> for IdBytes {
        fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
        where
            D: Deserializer<'de>,
        {
            // serenity's ids panic if you try to construct them from `u64::MAX`
            // since they are backed by `NonMaxU64` inner values.
            let int = <[u8; 8]>::deserialize(deserializer)?;
            let int = u64::from_le_bytes(int);
            if int != u64::MAX {
                Ok(T::from(int))
            } else {
                Err(D::Error::custom("discord id cannot be u64::MAX"))
            }
        }
    }
}
