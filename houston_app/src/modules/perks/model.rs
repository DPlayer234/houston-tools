use super::effects::Effect;
use super::items::Item;
use crate::data::HArgError;
use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wallet {
    pub _id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePerk {
    pub _id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    pub effect: Effect,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub until: DateTime<Utc>,
    pub state: Option<Bson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueRole {
    pub _id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(with = "id_as_i64")]
    pub role: RoleId,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder()
        .name(name.to_owned())
        .build()
}

impl Wallet {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.wallet")
    }

    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .options(name("guild-user"))
                .keys(doc! {
                    "guild": 1,
                    "user": 1,
                })
                .build(),
        ]
    }
}

impl ActivePerk {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.active_perks")
    }

    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .options(name("guild-user-effect"))
                .keys(doc! {
                    "guild": 1,
                    "user": 1,
                    "effect": 1,
                })
                .build(),
            IndexModel::builder()
                .options(name("guild-effect"))
                .keys(doc! {
                    "guild": 1,
                    "effect": 1,
                })
                .build(),
            IndexModel::builder()
                .options(name("until"))
                .keys(doc! {
                    "until": 1,
                })
                .build(),
        ]
    }
}

impl UniqueRole {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("perks.unique_role")
    }

    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .options(name("guild-user"))
                .keys(doc! {
                    "guild": 1,
                    "user": 1,
                })
                .build(),
        ]
    }
}

pub trait WalletExt {
    async fn add_items(&self, guild_id: GuildId, user_id: UserId, item: Item, amount: i64) -> Result<Wallet>;
    async fn take_items(&self, guild_id: GuildId, user_id: UserId, item: Item, amount: i64, perks: &super::config::Config) -> Result<Wallet>;
}

macro_rules! make_item_accessors {
    ($($item:ident => $field:ident,)*) => {
        fn item_to_key(item: Item) -> &'static str {
            match item {
                $( Item::$item => stringify!($field), )*
            }
        }

        impl Wallet {
            pub fn item(&self, item: Item) -> i64 {
                match item {
                    $( Item::$item => self.$field, )*
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
    async fn add_items(&self, guild_id: GuildId, user_id: UserId, item: Item, amount: i64) -> Result<Wallet> {
        let key = item_to_key(item);

        let filter = doc! {
            "guild": bson_id!(guild_id),
            "user": bson_id!(user_id),
        };

        let update = doc! {
            "$setOnInsert": {
                "guild": bson_id!(guild_id),
                "user": bson_id!(user_id),
            },
            "$inc": {
                key: amount,
            },
        };

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .upsert(true)
            .await?
            .context("cannot return none after upsert")?;

        Ok(doc)
    }

    async fn take_items(&self, guild_id: GuildId, user_id: UserId, item: Item, amount: i64, perks: &super::config::Config) -> Result<Wallet> {
        let key = item_to_key(item);

        let filter = doc! {
            "guild": bson_id!(guild_id),
            "user": bson_id!(user_id),
            key: {
                "$gte": amount,
            }
        };

        let update = doc! {
            "$inc": {
                key: -amount,
            },
        };

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::Before)
            .await?
            .ok_or_else(|| HArgError::new(format!(
                "You need {} {} to do this.",
                amount, item.info(perks).name,
            )))?;

        Ok(doc)
    }
}

pub trait ActivePerkExt {
    async fn set_enabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect, until: DateTime<Utc>) -> Result;
    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result;
    async fn find_enabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result<Option<ActivePerk>>;
}

fn active_perk_filter(guild_id: GuildId, user_id: UserId, effect: Effect) -> Result<Document> {
    Ok(doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(user_id),
        "effect": bson::ser::to_bson(&effect)?,
    })
}

impl ActivePerkExt for Collection<ActivePerk> {
    async fn set_enabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect, until: DateTime<Utc>) -> Result {
        let filter = active_perk_filter(guild_id, user_id, effect)?;
        let update = doc! {
            "$setOnInsert": filter.clone(),
            "$set": {
                "until": Bson::DateTime(until.into()),
            },
        };

        self.update_one(filter, update)
            .upsert(true)
            .await?;

        Ok(())
    }

    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        self.delete_one(filter)
            .await?;
        Ok(())
    }

    async fn find_enabled(&self, guild_id: GuildId, user_id: UserId, effect: Effect) -> Result<Option<ActivePerk>> {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        let doc = self.find_one(filter).await?;
        Ok(doc)
    }
}
