use std::time::Instant;

use utils::text::write_str::*;

use crate::fmt::discord::{TimeMentionable as _, get_unique_username};
use crate::helper::time::get_startup_time;
use crate::modules::Module as _;
use crate::slashies::prelude::*;

/// Shows information about the current bot session.
#[chat_command(
    name = "bot-stats",
    contexts = "BotDm",
    integration_types = "Guild | User"
)]
pub async fn bot_stats(ctx: Context<'_>) -> Result {
    use crate::build::{GIT_HASH, VERSION};

    let now = Instant::now();

    ctx.defer(false).await?;

    let elapsed = now.elapsed().as_millis();

    let data = ctx.data_ref();
    let startup = get_startup_time().short_date_time();

    let description = format!(
        "**Started:** {startup}\n\
         **Version:** `{VERSION}`\n\
         **Git Rev:** `{GIT_HASH}`\n\
         **Ping:** {elapsed} ms"
    );

    let current_user = data.cache.current_user()?;
    let author = get_unique_username(current_user);
    let author_icon = current_user.face();

    let author = CreateEmbedAuthor::new(author).icon_url(author_icon);
    let footer = CreateEmbedFooter::new("Houston Tools");

    let config = data.config();
    let mut modules = String::new();

    // core must be enabled -- this command is part of it
    writeln_str!(modules, "**core**");

    if crate::modules::azur::Module.enabled(config) {
        let azur = config.azur_raw()?;
        let load_label = if azur.loaded() { "loaded" } else { "unloaded" };
        writeln_str!(modules, "**azur:** {load_label}");
    }

    if crate::modules::media_react::Module.enabled(config) {
        let channels = config.media_react.len();
        writeln_str!(modules, "**media_react:** channels: {channels}");
    }

    if crate::modules::minigame::Module.enabled(config) {
        writeln_str!(modules, "**minigame**");
    }

    if crate::modules::perks::Module.enabled(config) {
        let perks = config.perks()?;
        modules.push_str("**perks:**");

        if perks.rainbow.is_some() {
            modules.push_str(" rainbow");
        }
        if perks.pushpin.is_some() {
            modules.push_str(" pushpin");
        }
        if perks.role_edit.is_some() {
            modules.push_str(" role_edit");
        }
        if perks.collectible.is_some() {
            modules.push_str(" collectible");
        }
        if perks.birthday.is_some() {
            modules.push_str(" birthday");
        }

        modules.push('\n');
    }

    if crate::modules::profile::Module.enabled(config) {
        writeln_str!(modules, "**profile**");
    }

    if crate::modules::starboard::Module.enabled(config) {
        let guilds = config.starboard.len();
        let boards = config
            .starboard
            .values()
            .map(|v| v.boards.len())
            .sum::<usize>();

        writeln_str!(modules, "**starboard:** guilds: {guilds}, boards: {boards}");
    }

    let mut embed = CreateEmbed::new()
        .author(author)
        .footer(footer)
        .color(config.embed_color)
        .description(description)
        .field("Modules", modules, false);

    if let Some(stats) = data.cache.stats() {
        embed = embed.field("Cache", stats, false);
    }

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
