use std::fmt::Write;

use bitflags::Flags;

use utils::Discard;
use utils::time::*;
use utils::titlecase;

use crate::fmt::discord::get_unique_username;
use crate::prelude::*;

/// Returns basic information about the provided user.
#[poise::command(slash_command, context_menu_command = "User Info")]
pub async fn who(
    ctx: HContext<'_>,
    #[description = "The user to get info about."]
    user: User
) -> HResult {
    let mut embed = who_user_embed(&user);

    // while the resolved params would have the member, that's not available
    // in context menu commands. in the interest of still supporting that,
    // manually look up the member in the resolved collection here.
    // plus, it's more code to implement a custom parameter type that's User + Option<PartialMember>.
    if let HContext::Application(ctx) = &ctx {
        if let Some(member) = ctx.interaction.data.resolved.members.get(&user.id) {
            embed = embed.field("Server Member Info", who_member_info(member), false);
        }
    }

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

/* Format the embeds */

fn who_user_embed(user: &User) -> CreateEmbed {
    CreateEmbed::new()
        .author(CreateEmbedAuthor::new(get_unique_username(user)))
        .thumbnail(user.face())
        .description(who_user_info(user))
        .color(DEFAULT_EMBED_COLOR)
}

fn who_user_info(user: &User) -> String {
    let mut f = String::new();

    if let Some(global_name) = &user.global_name {
        writeln!(f, "**Display Name:** {global_name}").discard();
    }

    write!(
        f,
        "**Snowflake:** `{}`\n\
        **Created At:** {}\n",
        user.id,
        user.created_at().short_date_time(),
    ).discard();

    if let Some(avatar_url) = user.avatar_url() {
        writeln!(f, "**Avatar:** [Click]({avatar_url})").discard();
    }

    // Bots don't get banners.

    if let Some(public_flags) = user.public_flags.filter(|p| !p.is_empty()) {
        write_public_flags(&mut f, public_flags);
    }

    let label = if user.bot {
        "Bot Account"
    } else if user.system {
        "System Account"
    } else {
        "User Account"
    };

    writeln!(f, "**{label}**").discard();

    f
}

/* Additional server member info */

fn who_member_info(member: &PartialMember) -> String {
    // role ids are also present, but not useful since there is no guild info.

    let mut f = String::new();

    if let Some(nick) = &member.nick {
        writeln!(f, "**Nickname:** `{nick}`").discard();
    }

    if let Some(joined_at) = member.joined_at {
        writeln!(f, "**Joined At:** {}", joined_at.short_date_time()).discard();
    }

    if let Some(premium_since) = member.premium_since {
        writeln!(f, "**Boosting Since:** {}", premium_since.short_date_time()).discard();
    }

    if let Some(permissions) = member.permissions.filter(|p| !p.is_empty()) {
        // these are channel scoped.
        write_permissions(&mut f, permissions);
    }

    f
}

/* Local utilities */

fn write_public_flags(f: &mut String, public_flags: UserPublicFlags) {
    macro_rules! flag {
        ($flag:ident) => {
            (UserPublicFlags::$flag, titlecase!(stringify!($flag)))
        };
    }

    // use const size to catch when new flags are added
    const FLAG_COUNT: usize = UserPublicFlags::FLAGS.len();
    const FLAGS: [(UserPublicFlags, &str); FLAG_COUNT] = [
        flag!(DISCORD_EMPLOYEE),
        flag!(PARTNERED_SERVER_OWNER),
        flag!(HYPESQUAD_EVENTS),
        flag!(BUG_HUNTER_LEVEL_1),
        flag!(HOUSE_BRAVERY),
        flag!(HOUSE_BRILLIANCE),
        flag!(HOUSE_BALANCE),
        flag!(EARLY_SUPPORTER),
        flag!(TEAM_USER),
        flag!(SYSTEM),
        flag!(BUG_HUNTER_LEVEL_2),
        flag!(VERIFIED_BOT),
        flag!(EARLY_VERIFIED_BOT_DEVELOPER),
        flag!(DISCORD_CERTIFIED_MODERATOR),
        flag!(BOT_HTTP_INTERACTIONS),
        flag!(ACTIVE_DEVELOPER),
    ];

    write!(f, "**Public Flags:** `{:#x}`\n> -# ", public_flags.bits()).discard();

    write_flags(f, public_flags, &FLAGS);
    f.push_str("\n");
}

fn write_permissions(f: &mut String, permissions: Permissions) {
    macro_rules! flag {
        ($flag:ident) => {
            (Permissions::$flag, titlecase!(stringify!($flag)))
        };
    }

    // use const size to catch when new flags are added
    // `-1` because one flag is deprecated/duplicated
    const FLAG_COUNT: usize = Permissions::FLAGS.len() - 1;
    const FLAGS: [(Permissions, &str); FLAG_COUNT] = [
        flag!(CREATE_INSTANT_INVITE),
        flag!(KICK_MEMBERS),
        flag!(BAN_MEMBERS),
        flag!(ADMINISTRATOR),
        flag!(MANAGE_CHANNELS),
        flag!(MANAGE_GUILD),
        flag!(ADD_REACTIONS),
        flag!(VIEW_AUDIT_LOG),
        flag!(PRIORITY_SPEAKER),
        flag!(STREAM),
        flag!(VIEW_CHANNEL),
        flag!(SEND_MESSAGES),
        flag!(SEND_TTS_MESSAGES),
        flag!(MANAGE_MESSAGES),
        flag!(EMBED_LINKS),
        flag!(ATTACH_FILES),
        flag!(READ_MESSAGE_HISTORY),
        flag!(MENTION_EVERYONE),
        flag!(USE_EXTERNAL_EMOJIS),
        flag!(VIEW_GUILD_INSIGHTS),
        flag!(CONNECT),
        flag!(SPEAK),
        flag!(MUTE_MEMBERS),
        flag!(DEAFEN_MEMBERS),
        flag!(MOVE_MEMBERS),
        flag!(USE_VAD),
        flag!(CHANGE_NICKNAME),
        flag!(MANAGE_NICKNAMES),
        flag!(MANAGE_ROLES),
        flag!(MANAGE_WEBHOOKS),
        flag!(MANAGE_GUILD_EXPRESSIONS),
        flag!(USE_APPLICATION_COMMANDS),
        flag!(REQUEST_TO_SPEAK),
        flag!(MANAGE_EVENTS),
        flag!(MANAGE_THREADS),
        flag!(CREATE_PUBLIC_THREADS),
        flag!(CREATE_PRIVATE_THREADS),
        flag!(USE_EXTERNAL_STICKERS),
        flag!(SEND_MESSAGES_IN_THREADS),
        flag!(USE_EMBEDDED_ACTIVITIES),
        flag!(MODERATE_MEMBERS),
        flag!(VIEW_CREATOR_MONETIZATION_ANALYTICS),
        flag!(USE_SOUNDBOARD),
        flag!(CREATE_GUILD_EXPRESSIONS),
        flag!(CREATE_EVENTS),
        flag!(USE_EXTERNAL_SOUNDS),
        flag!(SEND_VOICE_MESSAGES),
        flag!(SET_VOICE_CHANNEL_STATUS),
        flag!(SEND_POLLS),
        flag!(USE_EXTERNAL_APPS),
    ];

    write!(f, "**Permissions:** `{:#x}`\n> -# ", permissions.bits()).discard();

    if permissions.administrator() {
        f.push_str("Administrator, *");
    } else if !permissions.is_empty() {
        write_flags(f, permissions, &FLAGS);
    }

    f.push_str("\n");
}

fn write_flags<T: Flags + Copy>(f: &mut String, flags: T, names: &[(T, &str)]) {
    let mut first = true;
    for (flag, label) in names {
        if flags.contains(*flag) {
            if !first {
                f.push_str(", ");
            }

            f.push_str(label);
            first = false;
        }
    }

    if first {
        f.push_str("<None?>");
    }
}
