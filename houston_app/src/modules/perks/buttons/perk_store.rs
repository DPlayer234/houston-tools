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
use crate::modules::perks::config::{EffectPrice, ItemPrice};
use crate::modules::perks::effects::Args;
use crate::modules::perks::effects::Effect;
use crate::modules::perks::items::Item;
use crate::modules::perks::model::*;

// View the store.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    action: Action,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Action {
    Nothing,
    BuyEffect(Effect),
    BuyItem(Item),
}

impl View {
    pub fn new() -> Self {
        Self {
            action: Action::Nothing,
        }
    }

    pub async fn create_reply(mut self, ctx: &Context, guild_id: GuildId, user_id: UserId) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        // used for wallet and active perks
        let filter = doc! {
            "guild": bson_id!(guild_id),
            "user": bson_id!(user_id),
        };

        let wallet = match self.action {
            Action::Nothing => {
                Wallet::collection(db)
                    .find_one(filter.clone())
                    .await?
                    .unwrap_or_default()
            }
            Action::BuyEffect(effect) => {
                let st = effect.price(perks)
                    .context("effect cannot be bought")?;

                let wallet = Wallet::collection(db)
                    .take_items(guild_id, user_id, Item::Cash, st.cost.into())
                    .await?;

                let args = Args {
                    ctx,
                    guild_id,
                    user_id,
                };

                let duration = TimeDelta::try_hours(st.duration.into())
                    .context("invalid time configuration")?;

                let until = Utc::now()
                    .checked_add_signed(duration)
                    .context("duration beyond the end of time")?;

                ActivePerk::collection(db)
                    .set_enabled(guild_id, user_id, effect, until)
                    .await?;

                effect.enable(args).await?;
                wallet
            }
            Action::BuyItem(item) => {
                let st = item.price(perks)
                    .context("effect cannot be bought")?;

                Wallet::collection(db)
                    .take_items(guild_id, user_id, Item::Cash, st.cost.into())
                    .await?;

                Wallet::collection(db)
                    .add_items(guild_id, user_id, item, st.amount.into())
                    .await?
            }
        };

        self.action = Action::Nothing;

        let active = ActivePerk::collection(db)
            .find(filter)
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        fn find(active: &[ActivePerk], effect: Effect) -> Option<&ActivePerk> {
            active.iter().find(|p| p.effect == effect)
        }

        let mut description = String::new();
        let mut buttons = Vec::new();

        let mut add_effect = |st: EffectPrice, effect: Effect| {
            if let Some(active) = find(&active, effect) {
                writeln_str!(
                    description,
                    "- **{}:** ~~{}{}~~ ✅ until {}",
                    effect.name(), perks.cash_name, st.cost, active.until.short_date_time(),
                );
            } else {
                let custom_id = self.to_custom_id_with(utils::field_mut!(Self: action), Action::BuyEffect(effect));
                let button = CreateButton::new(custom_id)
                    .label(utils::text::truncate(effect.name(), 25))
                    .disabled(wallet.cash < i64::from(st.cost));

                buttons.push(button);

                writeln_str!(
                    description,
                    "- **{}:** {}{} for {}h",
                    effect.name(), perks.cash_name, st.cost, st.duration,
                );
            }
        };

        if let Some(rainbow) = &perks.rainbow {
            add_effect(rainbow.price, Effect::RainbowRole);
        }

        let mut add_item = |st: ItemPrice, item: Item| {
            let custom_id = self.to_custom_id_with(utils::field_mut!(Self: action), Action::BuyItem(item));
            let button = CreateButton::new(custom_id)
                .label(utils::text::truncate(item.name(perks), 25))
                .disabled(wallet.cash < i64::from(st.cost));

            buttons.push(button);

            write_str!(
                description,
                "- **{}:** {}{}",
                item.name(perks), perks.cash_name, st.cost,
            );

            if st.amount != 1 {
                write_str!(description, " for {}", st.amount);
            }

            let owned = wallet.item(item);
            if owned != 0 {
                write_str!(description, " [Held: {owned}]");
            }

            description.push('\n');
        };

        if let Some(collectible) = &perks.collectible {
            add_item(collectible.price, Item::Collectible);
        }

        let embed = CreateEmbed::new()
            .title("Perk Store")
            .color(DEFAULT_EMBED_COLOR)
            .description(description)
            .footer(CreateEmbedFooter::new(format!("Wallet: {}{}", perks.cash_name, wallet.cash)));

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
