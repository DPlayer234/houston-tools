use bitflags::Flags;
use utils::text::WriteStr as _;
use utils::titlecase;

use crate::fmt::discord::{TimeMentionable as _, get_unique_username};
use crate::helper::discord::guild_avatar_url;
use crate::slashies::prelude::*;

/// Returns basic information about the provided user.
#[context_command(
    user,
    name = "User Info",
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn who_context(ctx: Context<'_>, user: SlashUser<'_>) -> Result {
    who_core(ctx, user, None).await
}

/// Returns basic information about the provided user.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn who(
    ctx: Context<'_>,
    /// The user to get info about.
    user: SlashUser<'_>,
    /// Whether to show the response only to yourself.
    ephemeral: Option<bool>,
) -> Result {
    who_core(ctx, user, ephemeral).await
}

async fn who_core(ctx: Context<'_>, user: SlashUser<'_>, ephemeral: Option<bool>) -> Result {
    let mut embed = who_user_embed(user.user)
        .color(ctx.data_ref().config().embed_color)
        .thumbnail(user.face());

    if let Some(member) = user.member {
        embed = embed.field(
            "Server Member Info",
            who_member_info(user.user, member, ctx.guild_id().unwrap_or_default()),
            false,
        );
    }

    ctx.send(create_reply(ephemeral).embed(embed)).await?;
    Ok(())
}

/* Format the embeds */

fn who_user_embed(user: &User) -> CreateEmbed<'_> {
    CreateEmbed::new()
        .author(CreateEmbedAuthor::new(get_unique_username(user)))
        .description(who_user_info(user))
}

fn who_user_info(user: &User) -> String {
    let mut f = String::new();

    if let Some(global_name) = &user.global_name {
        writeln!(f, "**Display Name:** {global_name}");
    }

    write!(
        f,
        "**Snowflake:** `{}`\n\
        **Created At:** {}\n",
        user.id,
        user.id.created_at().short_date_time(),
    );

    if let Some(avatar_url) = user.avatar_url() {
        writeln!(f, "**Avatar:** [Click]({avatar_url})");
    }

    // Bots don't get banners.

    if let Some(public_flags) = user.public_flags.filter(|p| !p.is_empty()) {
        write_public_flags(&mut f, public_flags);
    }

    let label = if user.bot() {
        "Bot Account"
    } else if user.system() {
        "System Account"
    } else {
        "User Account"
    };

    writeln!(f, "**{label}**");

    f
}

/* Additional server member info */

fn who_member_info(user: &User, member: &PartialMember, guild_id: GuildId) -> String {
    // role ids are also present, but not useful since there is no guild info.

    let mut f = String::new();

    if let Some(nick) = &member.nick {
        writeln!(f, "**Nickname:** `{nick}`");
    }

    if let Some(joined_at) = member.joined_at {
        writeln!(f, "**Joined At:** {}", joined_at.short_date_time());
    }

    if let Some(hash) = &member.avatar {
        let avatar_url = guild_avatar_url(user.id, guild_id, hash);
        writeln!(f, "**Guild Avatar:** [Click]({avatar_url})");
    }

    if let Some(premium_since) = member.premium_since {
        writeln!(f, "**Boosting Since:** {}", premium_since.short_date_time());
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

    write!(f, "**Public Flags:** `{:#x}`\n> -# ", public_flags.bits());

    write_flags(f, public_flags, &FLAGS);
    f.push('\n');
}

fn write_permissions(f: &mut String, permissions: Permissions) {
    macro_rules! flag {
        ($flag:ident) => {
            (Permissions::$flag, titlecase!(stringify!($flag)))
        };
        ($flag:ident, $name:literal) => {
            (Permissions::$flag, $name)
        };
    }

    // use const size to catch when new flags are added
    const FLAG_COUNT: usize = Permissions::FLAGS.len();
    const FLAGS: [(Permissions, &str); FLAG_COUNT] = [
        flag!(ADMINISTRATOR),
        flag!(VIEW_CHANNEL),
        flag!(MANAGE_CHANNELS),
        flag!(MANAGE_ROLES),
        flag!(CREATE_GUILD_EXPRESSIONS, "Create Expressions"),
        flag!(MANAGE_GUILD_EXPRESSIONS, "Manage Expressions"),
        flag!(VIEW_AUDIT_LOG),
        flag!(VIEW_GUILD_INSIGHTS, "View Server Insights"),
        flag!(MANAGE_WEBHOOKS),
        flag!(MANAGE_GUILD, "Manage Server"),
        flag!(CREATE_INSTANT_INVITE, "Create Invite"),
        flag!(CHANGE_NICKNAME),
        flag!(MANAGE_NICKNAMES),
        flag!(KICK_MEMBERS),
        flag!(BAN_MEMBERS),
        flag!(MODERATE_MEMBERS, "Timeout Members"),
        flag!(SEND_MESSAGES),
        flag!(SEND_MESSAGES_IN_THREADS, "Send Messages in Threads"),
        flag!(CREATE_PUBLIC_THREADS),
        flag!(CREATE_PRIVATE_THREADS),
        flag!(EMBED_LINKS),
        flag!(ATTACH_FILES),
        flag!(ADD_REACTIONS),
        flag!(USE_EXTERNAL_EMOJIS),
        flag!(USE_EXTERNAL_STICKERS),
        flag!(MENTION_EVERYONE, "Mention @\u{200D}everyone"),
        flag!(MANAGE_MESSAGES),
        flag!(MANAGE_THREADS),
        flag!(READ_MESSAGE_HISTORY),
        flag!(SEND_TTS_MESSAGES),
        flag!(SEND_VOICE_MESSAGES),
        flag!(SEND_POLLS, "Create Polls"),
        flag!(CONNECT),
        flag!(SPEAK),
        flag!(STREAM, "Video"),
        flag!(USE_SOUNDBOARD),
        flag!(USE_EXTERNAL_SOUNDS),
        flag!(USE_VAD, "Use Voice Activity"),
        flag!(PRIORITY_SPEAKER),
        flag!(MUTE_MEMBERS),
        flag!(DEAFEN_MEMBERS),
        flag!(MOVE_MEMBERS),
        flag!(REQUEST_TO_SPEAK, "Request to Speak"),
        flag!(SET_VOICE_CHANNEL_STATUS),
        flag!(USE_APPLICATION_COMMANDS),
        flag!(USE_EMBEDDED_ACTIVITIES, "Use Activities"),
        flag!(USE_EXTERNAL_APPS),
        flag!(CREATE_EVENTS),
        flag!(MANAGE_EVENTS),
        flag!(VIEW_CREATOR_MONETIZATION_ANALYTICS),
    ];

    write!(f, "**Permissions:** `{:#x}`\n> -# ", permissions.bits());

    if permissions.administrator() {
        f.push_str("Administrator, *");
    } else if !permissions.is_empty() {
        write_flags(f, permissions, &FLAGS);
    }

    f.push('\n');
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
