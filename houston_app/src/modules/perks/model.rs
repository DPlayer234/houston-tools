use bson_model::Filter;

use super::effects::Effect;
use super::items::Item;
use super::DayOfYear;
use crate::data::HArgError;
use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ModelDocument)]
pub struct Wallet {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    pub birthday: Option<DayOfYear>,
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
    pub id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    pub effect: Effect,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub until: DateTime<Utc>,
    pub state: Option<Bson>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct UniqueRole {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(with = "id_as_i64")]
    pub role: RoleId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct Birthday {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    pub region: u16,
    pub day_of_year: DayOfYear,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder().name(name.to_owned()).build()
}

impl Wallet {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.wallet")
    }

    pub fn indices() -> Vec<IndexModel> {
        vec![IndexModel::builder()
            .options(name("guild-user"))
            .keys(Self::sort().guild(Asc).user(Asc))
            .build()]
    }
}

impl ActivePerk {
    pub fn self_filter(&self) -> Document {
        doc! {
            "_id": self.id,
        }
    }

    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.active_perks")
    }

    pub fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name("guild-user-effect"))
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

impl UniqueRole {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.unique_role")
    }

    pub fn indices() -> Vec<IndexModel> {
        vec![IndexModel::builder()
            .options(name("guild-user"))
            .keys(Self::sort().guild(Asc).user(Asc))
            .build()]
    }
}

impl Birthday {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.birthday")
    }

    pub fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name("user"))
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
        item: Item,
        amount: i64,
    ) -> Result<Wallet>;

    async fn take_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        item: Item,
        amount: i64,
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
        }

        impl WalletFilter {
            pub fn item(self, item: Item, amount: impl Into<Filter<i64>>) -> Self {
                match item {
                    $( Item::$item => self.$field(amount), )*
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
        item: Item,
        amount: i64,
    ) -> Result<Wallet> {
        let filter = Wallet::filter()
            .guild(guild_id)
            .user(user_id)
            .into_document()?;

        let update = Wallet::update()
            .set_on_insert(|w| w.guild(guild_id).user(user_id))
            .inc(|w| w.item(item, amount))
            .into_document()?;

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .upsert(true)
            .await?
            .context("cannot return none after upsert")?;

        Ok(doc)
    }

    async fn take_items(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        item: Item,
        amount: i64,
        perks: &super::config::Config,
    ) -> Result<Wallet> {
        let filter = Wallet::filter()
            .guild(guild_id)
            .user(user_id)
            .item(item, Filter::Gte(amount))
            .into_document()?;

        let update = Wallet::update()
            .inc(|w| w.item(item, -amount))
            .into_document()?;

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::Before)
            .await?
            .ok_or_else(|| {
                HArgError::new(format!(
                    "You need {} {} to do this.",
                    amount,
                    item.info(perks).name,
                ))
            })?;

        Ok(doc)
    }
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
            .set_on_insert(|a| a.guild(guild_id).user(user_id).effect(effect))
            .set(|a| a.until(until))
            .into_document()?;

        self.update_one(filter, update).upsert(true).await?;
        Ok(())
    }

    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        self.delete_one(filter).await?;
        Ok(())
    }

    async fn find_enabled(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        effect: Effect,
    ) -> Result<Option<ActivePerk>> {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        let doc = self.find_one(filter).await?;
        Ok(doc)
    }
}
