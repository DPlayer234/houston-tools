use bson_model::Filter;
use houston_utils::Join;

use super::DayOfYear;
use super::effects::Effect;
use super::items::Item;
use crate::data::HArgError;
use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ModelDocument)]
pub struct Wallet {
    #[serde(rename = "_id")]
    #[model(filter = false, partial = false)]
    pub id: ObjectId,
    #[serde(with = "As::<IdBson>")]
    pub guild: GuildId,
    #[serde(with = "As::<IdBson>")]
    pub user: UserId,
    #[serde(default)]
    pub cash: i64,
    #[serde(default)]
    pub pushpin: i64,
    #[serde(default)]
    pub role_edit: i64,
    #[serde(default)]
    pub crab: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct ActivePerk {
    #[serde(rename = "_id")]
    #[model(filter = false, partial = false)]
    pub id: ObjectId,
    #[serde(with = "As::<IdBson>")]
    pub guild: GuildId,
    #[serde(with = "As::<IdBson>")]
    pub user: UserId,
    pub effect: Effect,
    #[serde(with = "As::<FromChrono04DateTime>")]
    pub until: DateTime<Utc>,
    #[model(filter = false)]
    pub state: Option<Bson>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct UniqueRole {
    #[serde(rename = "_id")]
    #[model(filter = false, partial = false)]
    pub id: ObjectId,
    #[serde(with = "As::<IdBson>")]
    pub guild: GuildId,
    #[serde(with = "As::<IdBson>")]
    pub user: UserId,
    #[serde(with = "As::<IdBson>")]
    #[model(filter = false)]
    pub role: RoleId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct Birthday {
    #[serde(rename = "_id")]
    #[model(filter = false, partial = false)]
    pub id: ObjectId,
    #[serde(with = "As::<IdBson>")]
    pub user: UserId,
    pub region: u16,
    pub day_of_year: DayOfYear,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder().name(name.to_owned()).build()
}

fn name_unique(name: &str) -> IndexOptions {
    IndexOptions::builder()
        .name(name.to_owned())
        .unique(true)
        .build()
}

impl ModelCollection for Wallet {
    const COLLECTION_NAME: &str = "perks.wallet";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name_unique("guild-user"))
                .keys(Self::sort().guild(Asc).user(Asc))
                .build(),
        ]
    }
}

impl ActivePerk {
    pub fn self_filter(&self) -> Document {
        doc! {
            "_id": self.id,
        }
    }
}

impl ModelCollection for ActivePerk {
    const COLLECTION_NAME: &str = "perks.active_perks";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name_unique("guild-user-effect"))
                .keys(Self::sort().guild(Asc).user(Asc).effect(Asc))
                .build(),
            IndexModel::builder()
                .options(name("guild-effect"))
                .keys(Self::sort().guild(Asc).effect(Asc))
                .build(),
            IndexModel::builder()
                .options(name("until"))
                .keys(Self::sort().until(Asc))
                .build(),
        ]
    }
}

impl ModelCollection for UniqueRole {
    const COLLECTION_NAME: &str = "perks.unique_role";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name_unique("guild-user"))
                .keys(Self::sort().guild(Asc).user(Asc))
                .build(),
        ]
    }
}

impl ModelCollection for Birthday {
    const COLLECTION_NAME: &str = "perks.birthday";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name_unique("user"))
                .keys(Self::sort().user(Asc))
                .build(),
            IndexModel::builder()
                .options(name("region-day_of_year"))
                .keys(Self::sort().region(Asc).day_of_year(Asc))
                .build(),
        ]
    }
}

pub trait WalletExt {
    async fn add_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        items: &[(Item, i64)],
    ) -> Result<Wallet>;

    async fn take_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        items: &[(Item, i64)],
        perks: &super::config::Config,
    ) -> Result<Wallet>;
}

macro_rules! make_item_accessors {
    ($($item:ident => $field:ident,)*) => {
        impl Wallet {
            pub fn item(&self, item: Item) -> i64 {
                match item {
                    $( Item::$item => self.$field, )*
                }
            }
        }

        impl WalletPartial {
            pub fn item(self, item: Item, amount: i64) -> Self {
                match item {
                    $( Item::$item => self.$field(amount), )*
                }
            }

            pub fn item_mut(&mut self, item: Item) -> &mut i64 {
                match item {
                    $( Item::$item => self.$field.get_or_insert_default(), )*
                }
            }
        }

        impl WalletFilter {
            pub fn item(self, item: Item, amount: impl Into<Filter<i64>>) -> Self {
                match item {
                    $( Item::$item => self.$field(amount), )*
                }
            }

            pub fn item_mut(&mut self, item: Item) -> &mut Option<Filter<i64>> {
                match item {
                    $( Item::$item => &mut self.$field, )*
                }
            }
        }
    };
}

make_item_accessors!(
    Cash => cash,
    Pushpin => pushpin,
    RoleEdit => role_edit,
    Collectible => crab,
);

impl WalletExt for Collection<Wallet> {
    async fn add_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        items: &[(Item, i64)],
    ) -> Result<Wallet> {
        let filter = Wallet::filter()
            .guild(guild_id)
            .user(user_id)
            .into_document()?;

        let mut update = Wallet::update();
        let inc = update.inc.get_or_insert_default();

        for &(item, amount) in items {
            *inc.item_mut(item) += amount;
        }

        let update = update.into_document()?;

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .upsert(true)
            .await
            .context("failed to add items to wallet")?
            .context("cannot return none after upsert")?;

        Ok(doc)
    }

    async fn take_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        items: &[(Item, i64)],
        perks: &super::config::Config,
    ) -> Result<Wallet> {
        let mut update = Wallet::update();
        let mut filter = Wallet::filter().guild(guild_id).user(user_id);
        let inc = update.inc.get_or_insert_default();

        for &(item, amount) in items {
            let filter = filter.item_mut(item);
            anyhow::ensure!(filter.is_none(), "duplicate Item in `Wallet::take_items`");

            *filter = Some(Filter::Gte(amount));
            *inc.item_mut(item) = -amount;
        }

        let filter = filter.into_document()?;
        let update = update.into_document()?;

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::Before)
            .await
            .context("failed to try to take items from wallet")?
            .ok_or_else(|| item_take_error(items, perks))?;

        Ok(doc)
    }
}

fn item_take_error(items: &[(Item, i64)], perks: &super::config::Config) -> HArgError {
    let fmt = Join::AND.display_with(items, |(item, amount), f| {
        write!(f, "{amount} {}", item.info(perks).name)
    });

    HArgError::new(format!("You need {fmt} to do this."))
}

pub trait ActivePerkExt {
    async fn set_enabled(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
        until: DateTime<Utc>,
    ) -> Result;

    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result;

    async fn find_enabled(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
    ) -> Result<Option<ActivePerk>>;
}

fn active_perk_filter(guild_id: GuildId, user_id: UserId, effect: Effect) -> Result<Document> {
    Ok(ActivePerk::filter()
        .guild(guild_id)
        .user(user_id)
        .effect(effect)
        .into_document()?)
}

impl ActivePerkExt for Collection<ActivePerk> {
    async fn set_enabled(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
        until: DateTime<Utc>,
    ) -> Result {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        let update = ActivePerk::update()
            .set(|a| a.until(until))
            .into_document()?;

        self.update_one(filter, update)
            .upsert(true)
            .await
            .context("failed to set perk enabled in db")?;
        Ok(())
    }

    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        self.delete_one(filter)
            .await
            .context("failed to set perk disabled in db")?;
        Ok(())
    }

    async fn find_enabled(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
    ) -> Result<Option<ActivePerk>> {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        let doc = self
            .find_one(filter)
            .await
            .context("failed to check enabled perk")?;
        Ok(doc)
    }
}
