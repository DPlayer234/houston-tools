use crate::internal::prelude::*;
use crate::time::*;

/// Returns basic information about the provided user.
#[poise::command(slash_command)]
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
	let mut builder = MessageBuilder::new();

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

	if let Some(public_flags) = user.public_flags.and_then(to_string_public_flags) {
		builder.push_bold("Public Flags:")
			.push(' ')
			.push_mono_line(public_flags);
	}

	builder.push_bold(if user.bot { "Bot Account:" } else { "User Account:" })
		.push(' ')
		.user(user);

    CreateEmbed::new()
	    .author(CreateEmbedAuthor::new(user.name.clone()))
		.thumbnail(user.face())
		.description(builder.build())
        .color(DEFAULT_EMBED_COLOR)
}

/* Local utilities */

macro_rules! append_flag {
	($type:ident, $buffer:expr, $value:expr, $flag:ident) => {
		if $value.contains($type::$flag) {
			if !$buffer.is_empty() {
				$buffer.push_str(", ");
			}

			$buffer.push_str(stringify!($flag));
		}
	};
}

fn to_string_public_flags(public_flags: UserPublicFlags) -> Option<String> {
	let mut buffer = String::new();

	append_flag!(UserPublicFlags, buffer, public_flags, DISCORD_EMPLOYEE);
	append_flag!(UserPublicFlags, buffer, public_flags, PARTNERED_SERVER_OWNER);
	append_flag!(UserPublicFlags, buffer, public_flags, HYPESQUAD_EVENTS);
	append_flag!(UserPublicFlags, buffer, public_flags, BUG_HUNTER_LEVEL_1);
	append_flag!(UserPublicFlags, buffer, public_flags, HOUSE_BRAVERY);
	append_flag!(UserPublicFlags, buffer, public_flags, HOUSE_BRILLIANCE);
	append_flag!(UserPublicFlags, buffer, public_flags, HOUSE_BALANCE);
	append_flag!(UserPublicFlags, buffer, public_flags, EARLY_SUPPORTER);
	append_flag!(UserPublicFlags, buffer, public_flags, TEAM_USER);
	append_flag!(UserPublicFlags, buffer, public_flags, SYSTEM);
	append_flag!(UserPublicFlags, buffer, public_flags, BUG_HUNTER_LEVEL_2);
	append_flag!(UserPublicFlags, buffer, public_flags, VERIFIED_BOT);
	append_flag!(UserPublicFlags, buffer, public_flags, EARLY_VERIFIED_BOT_DEVELOPER);
	append_flag!(UserPublicFlags, buffer, public_flags, DISCORD_CERTIFIED_MODERATOR);
	append_flag!(UserPublicFlags, buffer, public_flags, BOT_HTTP_INTERACTIONS);
	append_flag!(UserPublicFlags, buffer, public_flags, ACTIVE_DEVELOPER);

	if buffer.is_empty() {
		None
	} else {
		Some(buffer)
	}
}