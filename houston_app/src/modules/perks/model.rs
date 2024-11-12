use anyhow::Context;
use bson::{doc, Bson, Document};
use bson::oid::ObjectId;
use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use chrono::{DateTime, Utc};
use mongodb::options::{IndexOptions, ReturnDocument};
use mongodb::{Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serenity::model::id::{GuildId, UserId};

use crate::helper::bson_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub _id: ObjectId,
    pub guild: GuildId,
    pub user: UserId,
    pub cash: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePerk {
    pub _id: ObjectId,
    pub guild: GuildId,
    pub user: UserId,
    pub effect: super::effects::Kind,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub until: DateTime<Utc>,
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
                .options(name("until"))
                .keys(doc! {
                    "until": 1,
                })
                .build(),
        ]
    }
}

pub trait WalletExt {
    async fn add_cash(&self, guild_id: GuildId, user_id: UserId, amount: i64) -> anyhow::Result<i64>;
    async fn try_take_cash(&self, guild_id: GuildId, user_id: UserId, amount: i64) -> anyhow::Result<bool>;
}

impl WalletExt for Collection<Wallet> {
    async fn add_cash(&self, guild_id: GuildId, user_id: UserId, amount: i64) -> anyhow::Result<i64> {
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
                "cash": amount,
            },
        };

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .upsert(true)
            .await?
            .context("cannot return none after upsert")?;

        Ok(doc.cash)
    }

    async fn try_take_cash(&self, guild_id: GuildId, user_id: UserId, amount: i64) -> anyhow::Result<bool> {
        let filter = doc! {
            "guild": bson_id!(guild_id),
            "user": bson_id!(user_id),
            "cash": {
                "$gt": amount,
            }
        };

        let update = doc! {
            "$inc": {
                "cash": -amount,
            },
        };

        let doc = self
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::Before)
            .await?;

        Ok(doc.is_some())
    }
}

pub trait ActivePerkExt {
    async fn set_enabled(&self, guild_id: GuildId, user_id: UserId, effect: super::effects::Kind, until: DateTime<Utc>) -> anyhow::Result<()>;
    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: super::effects::Kind) -> anyhow::Result<()>;
}

fn active_perk_filter(guild_id: GuildId, user_id: UserId, effect: super::effects::Kind) -> anyhow::Result<Document> {
    Ok(doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(user_id),
        "effect": bson::ser::to_bson(&effect)?,
    })
}

impl ActivePerkExt for Collection<ActivePerk> {
    async fn set_enabled(&self, guild_id: GuildId, user_id: UserId, effect: super::effects::Kind, until: DateTime<Utc>) -> anyhow::Result<()> {
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

    async fn set_disabled(&self, guild_id: GuildId, user_id: UserId, effect: super::effects::Kind) -> anyhow::Result<()> {
        let filter = active_perk_filter(guild_id, user_id, effect)?;

        self.delete_one(filter)
            .await?;
        Ok(())
    }
}
