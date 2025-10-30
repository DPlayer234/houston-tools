pub use houston_utils_discord_components as components;
use serenity::model::prelude::*;
use serenity::small_fixed_array::FixedString;

pub mod events;
pub mod fmt;
mod serde;

pub use serde::IdBytes;

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
