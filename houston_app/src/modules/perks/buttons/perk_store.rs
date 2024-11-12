use anyhow::Context as _;
use bson::doc;

use chrono::TimeDelta;
use chrono::Utc;
use serenity::futures::TryStreamExt;
use serenity::gateway::client::Context;
use utils::text::write_str::*;
use utils::time::TimeMentionable;

use crate::buttons::prelude::*;
use crate::helper::bson_id;
use crate::modules::perks::effects::Args;
use crate::modules::perks::effects::Kind;
use crate::modules::perks::model::*;
use crate::modules::perks::StoreConfig;

// View the store.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    action: Action,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Action {
    Nothing,
    Buy(Kind, StoreConfig),
}

impl View {
    pub fn new() -> Self {
        Self {
            action: Action::Nothing,
        }
    }
}

#[cfg(feature = "db")]
impl View {
    pub async fn create_reply<'new>(mut self, ctx: &Context, guild_id: GuildId, user_id: UserId) -> anyhow::Result<CreateReply<'new>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        if let Action::Buy(effect, conf) = self.action {
            let success = Wallet::collection(db)
                .try_take_cash(guild_id, user_id, conf.cost.into())
                .await?;

            if success {
                let args = Args {
                    ctx,
                    guild_id,
                    user_id,
                };

                let duration = TimeDelta::try_hours(conf.duration.into())
                    .context("invalid time configuration")?;

                let until = Utc::now()
                    .checked_add_signed(duration)
                    .context("duration beyond the end of time")?;

                ActivePerk::collection(db)
                    .set_enabled(guild_id, user_id, effect, until)
                    .await?;

                effect.enable(args).await?;
            }
        }

        self.action = Action::Nothing;
        let filter = doc! {
            "guild": bson_id!(guild_id),
            "user": bson_id!(user_id),
        };

        let active = ActivePerk::collection(db)
            .find(filter.clone())
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        let wallet = Wallet::collection(db)
            .find_one(filter)
            .await?
            .map(|w| w.cash)
            .unwrap_or_default();

        fn find(active: &[ActivePerk], effect: Kind) -> Option<&ActivePerk> {
            active.iter().find(|p| p.effect == effect)
        }

        let mut description = String::new();
        let mut buttons = Vec::new();

        let mut emit_item = |config: StoreConfig, effect: Kind| {
            if let Some(active) = find(&active, effect) {
                writeln_str!(
                    description,
                    "- **{}:** ~~${}~~ ✅ until {}",
                    effect.name(), config.cost, active.until.short_date_time(),
                );
            } else {
                let custom_id = self.to_custom_id_with(utils::field_mut!(Self: action), Action::Buy(effect, config));
                let button = CreateButton::new(custom_id)
                    .label(format!("{}: ${}", effect.name(), config.cost))
                    .disabled(wallet < i64::from(config.cost));

                buttons.push(button);

                writeln_str!(
                    description,
                    "- **{}:** ${} for {}h",
                    effect.name(), config.cost, config.duration,
                );
            }
        };

        if let Some(rainbow) = &perks.rainbow {
            emit_item(rainbow.store, Kind::RainbowRole);
        }

        let embed = CreateEmbed::new()
            .title("Perk Store")
            .color(DEFAULT_EMBED_COLOR)
            .description(description)
            .footer(CreateEmbedFooter::new(format!("Wallet: ${wallet}")));

        let components: Vec<_> = buttons
            .chunks(5)
            .map(|c| CreateActionRow::buttons(c.to_vec()))
            .collect();

        let reply = CreateReply::new()
            .embed(embed)
            .components(components);

        Ok(reply)
    }
}

#[cfg(not(feature = "db"))]
impl ButtonArgsReply for View {
    async fn reply(self, _ctx: ButtonContext<'_>) -> HResult {
        anyhow::bail!("perks not supported without db");
    }
}

#[cfg(feature = "db")]
impl ButtonArgsReply for View {
    async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
        let guild_id = ctx.interaction.guild_id.context("requires guild")?;
        let user_id = ctx.interaction.user.id;

        ctx.reply(CreateInteractionResponse::Acknowledge).await?;

        let reply = self.create_reply(ctx.serenity, guild_id, user_id).await?;
        let edit = reply.to_slash_initial_response_edit(Default::default());

        ctx.edit_reply(edit).await?;
        Ok(())
    }
}
