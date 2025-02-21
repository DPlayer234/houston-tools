use bson::Document;
use chrono::Utc;
use serenity::prelude::*;
use utils::iter::IteratorExt as _;
use utils::text::truncate;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::fmt::discord::TimeMentionable as _;
use crate::fmt::time::HumanDuration;
use crate::modules::perks::config::{Config, ItemPrice};
use crate::modules::perks::effects::{Args, Effect};
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
    BuyItem(Item, u16),
}

// 20 EM dashes.
const BREAK: &str = "\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}\
    \u{2014}\u{2014}\u{2014}\u{2014}\u{2014}";

// used for wallet and active perks
fn filter(guild_id: GuildId, user_id: UserId) -> Result<Document> {
    Ok(Wallet::filter()
        .guild(guild_id)
        .user(user_id)
        .into_document()?)
}

fn base_shop_embed<'new>(perks: &Config, wallet: &Wallet) -> CreateEmbed<'new> {
    CreateEmbed::new().footer(CreateEmbedFooter::new(format!(
        "Wallet: {}{}",
        perks.cash_name, wallet.cash
    )))
}

impl View {
    pub fn new() -> Self {
        Self::with_action(Action::Main)
    }

    fn with_action(action: Action) -> Self {
        Self { action }
    }

    async fn view_main(
        self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HContextData>();
        let perks = data.config().perks()?;
        let db = data.database()?;

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id)?)
            .await?
            .unwrap_or_default();

        let active = ActivePerk::collection(db)
            .find(filter(guild_id, user_id)?)
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

            let info = effect.info(perks);

            let custom_id = Self::with_action(Action::ViewEffect(effect)).to_custom_id();
            let button = CreateButton::new(custom_id).label(truncate(info.name, 25));

            buttons.push(button);

            if let Some(active) = find(&active, effect) {
                writeln_str!(
                    description,
                    "- **{}:** ✅ until {}",
                    info.name,
                    active.until.short_date_time(),
                );
            } else {
                writeln_str!(
                    description,
                    "- **{}:** {}{} for {}",
                    info.name,
                    perks.cash_name,
                    st.cost,
                    HumanDuration::new(st.duration),
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
            let button = CreateButton::new(custom_id).label(truncate(info.name, 25));

            buttons.push(button);

            write_str!(
                description,
                "- **{}:** {}{}",
                info.name,
                perks.cash_name,
                st.cost,
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
            .into_iter()
            .vec_chunks(5)
            .map(CreateActionRow::buttons)
            .collect();

        let reply = CreateReply::new().embed(embed).components(components);
        Ok(reply)
    }

    async fn view_effect(
        self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
    ) -> Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HContextData>();
        let perks = data.config().perks()?;
        let db = data.database()?;

        let st = effect.price(perks).context("effect cannot be bought")?;

        let info = effect.info(perks);

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id)?)
            .await?
            .unwrap_or_default();

        let active = ActivePerk::collection(db)
            .find_enabled(guild_id, user_id, effect)
            .await?;

        let mut description = format!("> {}\n-# {BREAK}\n", info.description,);

        if let Some(active) = &active {
            write_str!(
                description,
                "~~Cost: {}{} for {}~~\n✅ until {}",
                perks.cash_name,
                st.cost,
                HumanDuration::new(st.duration),
                active.until.short_date_time(),
            );
        } else {
            write_str!(
                description,
                "Cost: {}{} for {}",
                perks.cash_name,
                st.cost,
                HumanDuration::new(st.duration),
            );
        }

        let embed = base_shop_embed(perks, &wallet)
            .title(truncate(info.name, 100))
            .description(description)
            .color(data.config().embed_color);

        let back = Self::new().to_custom_id();
        let back = CreateButton::new(back).emoji('⏪').label("Back");

        let buy = Self::with_action(Action::BuyEffect(effect)).to_custom_id();
        let buy = CreateButton::new(buy)
            .label("Buy")
            .style(ButtonStyle::Success)
            .disabled(wallet.cash < st.cost.into() || active.is_some());

        let components = vec![CreateActionRow::buttons(vec![back, buy])];
        let reply = CreateReply::new().embed(embed).components(components);
        Ok(reply)
    }

    async fn view_item(
        self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        item: Item,
    ) -> Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HContextData>();
        let perks = data.config().perks()?;
        let db = data.database()?;

        let st = item.price(perks).context("effect cannot be bought")?;

        let info = item.info(perks);

        let wallet = Wallet::collection(db)
            .find_one(filter(guild_id, user_id)?)
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
            .title(truncate(info.name, 100))
            .description(description)
            .color(data.config().embed_color);

        let back = Self::new().to_custom_id();
        let back = CreateButton::new(back).emoji('⏪').label("Back");

        let buy = Self::with_action(Action::BuyItem(item, 1)).to_custom_id();
        let buy = CreateButton::new(buy)
            .label("Buy")
            .style(ButtonStyle::Success)
            .disabled(wallet.cash < st.cost.into());

        let mut buttons = vec![back, buy];

        fn buy_button<'new>(
            wallet: &Wallet,
            st: ItemPrice,
            item: Item,
            mult: u16,
        ) -> CreateButton<'new> {
            let cost = i64::from(st.cost) * i64::from(mult);
            let buy = View::with_action(Action::BuyItem(item, mult)).to_custom_id();
            CreateButton::new(buy)
                .label(format!("x{mult}"))
                .style(ButtonStyle::Success)
                .disabled(wallet.cash < cost)
        }

        if owned >= 10 {
            buttons.push(buy_button(&wallet, st, item, 10));
        }
        if owned >= 50 {
            buttons.push(buy_button(&wallet, st, item, 50));
        }
        if owned >= 250 {
            buttons.push(buy_button(&wallet, st, item, 250));
        }

        let components = vec![CreateActionRow::buttons(buttons)];
        let reply = CreateReply::new().embed(embed).components(components);
        Ok(reply)
    }

    async fn buy_effect(
        mut self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
    ) -> Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HContextData>();
        let perks = data.config().perks()?;
        let db = data.database()?;

        let args = Args::new(ctx, guild_id, user_id);
        if !(effect.supported(args).await?) {
            anyhow::bail!("effect is not supported in this server");
        }

        let st = effect.price(perks).context("effect cannot be bought")?;

        Wallet::collection(db)
            .take_items(guild_id, user_id, Item::Cash, st.cost.into(), perks)
            .await?;

        let until = Utc::now()
            .checked_add_signed(st.duration)
            .context("duration beyond the end of time")?;

        ActivePerk::collection(db)
            .set_enabled(guild_id, user_id, effect, until)
            .await?;

        effect.enable(args, None).await?;

        self.action = Action::ViewEffect(effect);
        self.view_effect(ctx, guild_id, user_id, effect).await
    }

    async fn buy_item(
        mut self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
        item: Item,
        mult: u16,
    ) -> Result<CreateReply<'_>> {
        let data = ctx.data_ref::<HContextData>();
        let perks = data.config().perks()?;
        let db = data.database()?;

        let st = item.price(perks).context("effect cannot be bought")?;

        let cost = i64::from(st.cost) * i64::from(mult);
        Wallet::collection(db)
            .take_items(guild_id, user_id, Item::Cash, cost, perks)
            .await?;

        let amount = i64::from(st.amount) * i64::from(mult);
        let wallet = Wallet::collection(db)
            .add_items(guild_id, user_id, item, amount)
            .await?;

        let owned = wallet.item(item);
        let args = Args::new(ctx, guild_id, user_id);
        item.on_buy(args, owned).await?;

        self.action = Action::ViewItem(item);
        self.view_item(ctx, guild_id, user_id, item).await
    }

    pub async fn create_reply(
        self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<CreateReply<'_>> {
        match self.action {
            Action::Main => self.view_main(ctx, guild_id, user_id).await,
            Action::ViewEffect(effect) => self.view_effect(ctx, guild_id, user_id, effect).await,
            Action::ViewItem(item) => self.view_item(ctx, guild_id, user_id, item).await,
            Action::BuyEffect(effect) => self.buy_effect(ctx, guild_id, user_id, effect).await,
            Action::BuyItem(item, mult) => self.buy_item(ctx, guild_id, user_id, item, mult).await,
        }
    }
}

impl ButtonArgsReply for View {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let guild_id = ctx.interaction.guild_id.context("requires guild")?;
        let user_id = ctx.interaction.user.id;

        ctx.acknowledge().await?;

        let reply = self.create_reply(ctx.serenity, guild_id, user_id).await?;
        ctx.edit(reply.into()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_eq() {
        let guild_id = GuildId::new(1);
        let user_id = UserId::new(2);

        let filter_active_perk = ActivePerk::filter()
            .guild(guild_id)
            .user(user_id)
            .into_document()
            .unwrap();

        let filter_wallet = Wallet::filter()
            .guild(guild_id)
            .user(user_id)
            .into_document()
            .unwrap();

        assert_eq!(filter_active_perk, filter_wallet);
    }
}
