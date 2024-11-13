use anyhow::Context as _;
use bson::doc;

use bson::Document;
use chrono::{Utc, TimeDelta};
use serenity::futures::TryStreamExt;
use serenity::gateway::client::Context;
use utils::text::write_str::*;
use utils::time::TimeMentionable;

use crate::buttons::prelude::*;
use crate::helper::bson_id;
use crate::modules::perks::config::Config;
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
    Main,
    ViewEffect(Effect),
    ViewItem(Item),
    BuyEffect(Effect),
    BuyItem(Item),
}

// 25 EM dashes.
const BREAK: &str = "\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}";

// used for wallet and active perks
fn filter(guild_id: GuildId, user_id: UserId) -> Document {
    doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(user_id),
    }
}

fn base_shop_embed<'new>(perks: &Config, wallet: &Wallet) -> CreateEmbed<'new> {
    CreateEmbed::new()
        .footer(CreateEmbedFooter::new(format!("Wallet: {}{}", perks.cash_name, wallet.cash)))
}

impl View {
    pub fn new() -> Self {
        Self::with_action(Action::Main)
    }

    fn with_action(action: Action) -> Self {
        Self { action }
    }

    async fn view_main(self, ctx: &Context, guild_id: GuildId, user_id: UserId) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id))
            .await?
            .unwrap_or_default();

        let active = ActivePerk::collection(db)
            .find(filter(guild_id, user_id))
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        fn find(active: &[ActivePerk], effect: Effect) -> Option<&ActivePerk> {
            active.iter().find(|p| p.effect == effect)
        }

        let mut description = String::new();
        let mut buttons = Vec::new();

        // add all effects first
        for &effect in Effect::all() {
            let Some(st) = effect.price(perks) else {
                continue;
            };

            let args = Args::new(ctx, guild_id, user_id);
            if !(effect.supported(args).await?) {
                continue;
            }

            let info = effect.info();

            let custom_id = Self::with_action(Action::ViewEffect(effect)).to_custom_id();
            let button = CreateButton::new(custom_id)
                .label(utils::text::truncate(info.name, 25));

            buttons.push(button);

            if let Some(active) = find(&active, effect) {
                writeln_str!(
                    description,
                    "- **{}:** ✅ until {}",
                    info.name, active.until.short_date_time(),
                );
            } else {
                writeln_str!(
                    description,
                    "- **{}:** {}{} for {}h",
                    info.name, perks.cash_name, st.cost, st.duration,
                );
            }
        }

        // add the items individually after
        for &item in Item::all() {
            let Some(st) = item.price(perks) else {
                continue;
            };

            let info = item.info(perks);

            let custom_id = Self::with_action(Action::ViewItem(item)).to_custom_id();
            let button = CreateButton::new(custom_id)
                .label(utils::text::truncate(info.name, 25));

            buttons.push(button);

            write_str!(
                description,
                "- **{}:** {}{}",
                info.name, perks.cash_name, st.cost,
            );

            if st.amount != 1 {
                write_str!(description, " for x{}", st.amount);
            }

            let owned = wallet.item(item);
            if owned != 0 {
                write_str!(description, " [Held: {owned}]");
            }

            description.push('\n');
        }

        let embed = base_shop_embed(perks, &wallet)
            .title("Server Shop")
            .description(description)
            .color(data.config().embed_color);

        let components: Vec<_> = buttons
            .chunks(5)
            .map(|c| CreateActionRow::buttons(c.to_vec()))
            .collect();

        let reply = CreateReply::new()
            .embed(embed)
            .components(components);

        Ok(reply)
    }

    async fn view_effect(self, ctx: &Context, guild_id: GuildId, user_id: UserId, effect: Effect) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        let st = effect.price(perks)
            .context("effect cannot be bought")?;

        let info = effect.info();

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id))
            .await?
            .unwrap_or_default();

        let active = ActivePerk::collection(db)
            .find_enabled(guild_id, user_id, effect)
            .await?;

        let mut description = format!(
            "> {}\n-# {BREAK}\n",
            info.description,
        );

        if let Some(active) = &active {
            write_str!(
                description,
                "~~Cost: {}{} for {}h~~\n✅ until {}",
                perks.cash_name, st.cost, st.duration, active.until.short_date_time(),
            );
        } else {
            write_str!(
                description,
                "Cost: {}{} for {}h",
                perks.cash_name, st.cost, st.duration,
            );
        }

        let embed = base_shop_embed(perks, &wallet)
            .title(utils::text::truncate(info.name, 100))
            .description(description)
            .color(data.config().embed_color);

        let back = Self::new().to_custom_id();
        let back = CreateButton::new(back).emoji('⏪').label("Back");

        let buy = Self::with_action(Action::BuyEffect(effect)).to_custom_id();
        let buy = CreateButton::new(buy)
            .label("Buy")
            .style(ButtonStyle::Success)
            .disabled(wallet.cash < st.cost.into() || active.is_some());

        let components = vec![
            CreateActionRow::buttons(vec![back, buy]),
        ];

        let reply = CreateReply::new()
            .embed(embed)
            .components(components);

        Ok(reply)
    }

    async fn view_item(self, ctx: &Context, guild_id: GuildId, user_id: UserId, item: Item) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        let st = item.price(perks)
            .context("effect cannot be bought")?;

        let info = item.info(perks);

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id))
            .await?
            .unwrap_or_default();

        let mut description = format!(
            "> {}\n-# {BREAK}\nCost: {}{}",
            info.description, perks.cash_name, st.cost,
        );

        if st.amount != 1 {
            write_str!(description, " for x{}", st.amount);
        }

        let owned = wallet.item(item);
        if owned != 0 {
            write_str!(description, "\nHeld: {owned}");
        }

        let embed = base_shop_embed(perks, &wallet)
            .title(utils::text::truncate(info.name, 100))
            .description(description)
            .color(data.config().embed_color);

        let back = Self::new().to_custom_id();
        let back = CreateButton::new(back).emoji('⏪').label("Back");

        let buy = Self::with_action(Action::BuyItem(item)).to_custom_id();
        let buy = CreateButton::new(buy)
            .label("Buy")
            .style(ButtonStyle::Success)
            .disabled(wallet.cash < st.cost.into());

        let components = vec![
            CreateActionRow::buttons(vec![back, buy]),
        ];

        let reply = CreateReply::new()
            .embed(embed)
            .components(components);

        Ok(reply)
    }

    async fn buy_effect(mut self, ctx: &Context, guild_id: GuildId, user_id: UserId, effect: Effect) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        let args = Args::new(ctx, guild_id, user_id);
        if !(effect.supported(args).await?) {
            anyhow::bail!("effect is not supported in this server");
        }

        let st = effect.price(perks)
            .context("effect cannot be bought")?;

        Wallet::collection(db)
            .take_items(guild_id, user_id, Item::Cash, st.cost.into())
            .await?;

        let duration = TimeDelta::try_hours(st.duration.into())
            .context("invalid time configuration")?;

        let until = Utc::now()
            .checked_add_signed(duration)
            .context("duration beyond the end of time")?;

        ActivePerk::collection(db)
            .set_enabled(guild_id, user_id, effect, until)
            .await?;

        effect.enable(args).await?;

        self.action = Action::ViewEffect(effect);
        self.view_effect(ctx, guild_id, user_id, effect).await
    }

    async fn buy_item(mut self, ctx: &Context, guild_id: GuildId, user_id: UserId, item: Item) -> anyhow::Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HBotData>();
        let perks = data.config().perks.as_ref().context("perks must be enabled")?;
        let db = data.database()?;

        let st = item.price(perks)
            .context("effect cannot be bought")?;

        Wallet::collection(db)
            .take_items(guild_id, user_id, Item::Cash, st.cost.into())
            .await?;

        Wallet::collection(db)
            .add_items(guild_id, user_id, item, st.amount.into())
            .await?;

        self.action = Action::ViewItem(item);
        self.view_item(ctx, guild_id, user_id, item).await
    }

    pub async fn create_reply(self, ctx: &Context, guild_id: GuildId, user_id: UserId) -> anyhow::Result<CreateReply<'_>> {
        match self.action {
            Action::Main => self.view_main(ctx, guild_id, user_id).await,
            Action::ViewEffect(effect) => self.view_effect(ctx, guild_id, user_id, effect).await,
            Action::ViewItem(item) => self.view_item(ctx, guild_id, user_id, item).await,
            Action::BuyEffect(effect) => self.buy_effect(ctx, guild_id, user_id, effect).await,
            Action::BuyItem(item) => self.buy_item(ctx, guild_id, user_id, item).await,
        }
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
