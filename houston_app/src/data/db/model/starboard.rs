use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub _id: ObjectId,
    pub board: ChannelId,
    pub message: MessageId,
    pub user: UserId,
    #[serde(default)]
    pub max_reacts: i64,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub _id: ObjectId,
    pub guild: GuildId,
    pub board: ChannelId,
    pub user: UserId,
    pub score: i64,
}

impl Message {
    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .keys(doc! {
                    "board": 1,
                    "message": 1,
                })
                .build(),
        ]
    }
}

impl Score {
    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .keys(doc! {
                    "guild": 1,
                    "board": 1,
                    "user": 1,
                })
                .build(),
        ]
    }
}
