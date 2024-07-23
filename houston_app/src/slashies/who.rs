use std::fmt::Write;

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
    let embed = who_user_embed(&user);
    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

/* Format the embeds */

fn who_user_embed(user: &User) -> CreateEmbed {
    CreateEmbed::new()
    .author(CreateEmbedAuthor::new(get_unique_username(user)))
        .thumbnail(user.face())
        .description(who_user_info(user))
        .fields(who_user_public_flags(user))
        .color(DEFAULT_EMBED_COLOR)
}

fn who_user_info(user: &User) -> String {
    let mut builder = MessageBuilder::new();

    if let Some(ref global_name) = user.global_name {
        builder.push_bold("Display Name:")
            .push(' ')
            .push_mono_line(global_name);
    }

    builder.push_bold("Snowflake:")
        .push(' ')
        .push_mono_line(user.id.to_string());

    builder.push_bold("Created At:")
        .push(' ')
        .push_line(user.created_at().mention(SHORT_DATE_TIME));

    if let Some(avatar_url) = user.avatar_url() {
        builder.push_bold("Avatar:")
            .push(' ')
            .push_named_link_safe("Click", avatar_url)
            .push('\n');
    }

    // Bots don't actually get this as far as I can tell
    if let Some(banner_url) = user.banner_url() {
        builder.push_bold("Banner:")
            .push(' ')
            .push_named_link_safe("Click", banner_url)
            .push('\n');
    } else if let Some(accent_color) = user.accent_colour {
        builder.push_bold("Accent Color:")
            .push(" #")
            .push_line(accent_color.hex());
    }

    if user.bot {
        builder.push_bold("Bot Account");
    } else if user.system {
        builder.push_bold("System Account");
    } else {
        builder.push_bold("User Account");

        // Apparently bots don't get this either
        match user.premium_type {
            PremiumType::None => (),
            PremiumType::NitroClassic => { builder.push(" w/ Nitro Classic"); },
            PremiumType::Nitro => { builder.push(" w/ Nitro"); },
            PremiumType::NitroBasic => { builder.push(" w/ Nitro Basic"); },
            _ => { builder.push(format!(" w/ Premium Type {}", u8::from(user.premium_type))); },
        }
    }

    builder.0
}

fn who_user_public_flags(user: &User) -> Option<SimpleEmbedFieldCreate> {
    user.public_flags
        .filter(|s| !s.is_empty())
        .map(|f| ("Public Flags", to_string_public_flags(f), true))
}

/* Local utilities */

fn to_string_public_flags(public_flags: UserPublicFlags) -> String {
    let mut buffer = String::new();

    macro_rules! append_flag {
        ($flag:ident) => {
            if public_flags.contains(UserPublicFlags::$flag) {
                if !buffer.is_empty() {
                    buffer.push('\n');
                }

                buffer.push('`');
                buffer.push_str(titlecase!(stringify!($flag)));
                buffer.push('`');
            }
        };
    }

    append_flag!(DISCORD_EMPLOYEE);
    append_flag!(PARTNERED_SERVER_OWNER);
    append_flag!(HYPESQUAD_EVENTS);
    append_flag!(BUG_HUNTER_LEVEL_1);
    append_flag!(HOUSE_BRAVERY);
    append_flag!(HOUSE_BRILLIANCE);
    append_flag!(HOUSE_BALANCE);
    append_flag!(EARLY_SUPPORTER);
    append_flag!(TEAM_USER);
    append_flag!(SYSTEM);
    append_flag!(BUG_HUNTER_LEVEL_2);
    append_flag!(VERIFIED_BOT);
    append_flag!(EARLY_VERIFIED_BOT_DEVELOPER);
    append_flag!(DISCORD_CERTIFIED_MODERATOR);
    append_flag!(BOT_HTTP_INTERACTIONS);
    append_flag!(ACTIVE_DEVELOPER);

    if !buffer.is_empty() {
        buffer.push('\n');
    }

    write!(buffer, "Raw: `{:#x}`", public_flags.bits()).discard();

    buffer
}