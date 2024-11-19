use bson::doc;
use bson::oid::ObjectId;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, MessageId, UserId};

use super::BoardId;
use crate::helper::bson::id_as_i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub _id: ObjectId,
    pub board: BoardId,
    #[serde(with = "id_as_i64")]
    pub channel: ChannelId,
    #[serde(with = "id_as_i64")]
    pub message: MessageId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(default)]
    pub max_reacts: i64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub pin_messages: Vec<MessageId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub _id: ObjectId,
    pub board: BoardId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub post_count: i64,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder()
        .name(name.to_owned())
        .build()
}

impl Message {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("starboard.messages")
    }

    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .options(name("board-message"))
                .keys(doc! {
                    "board": 1,
                    "message": 1,
                })
                .build(),
            IndexModel::builder()
                .options(name("top-posts-sort"))
                .keys(doc! {
                    "board": 1,
                    "max_reacts": 1,
                    "message": 1,
                })
                .build(),
        ]
    }
}

impl Score {
    pub fn collection(db: &Database) -> Collection<Self> {
        db.collection("starboard.scores")
    }

    pub fn indices() -> impl IntoIterator<Item = IndexModel> {
        [
            IndexModel::builder()
                .options(name("board-user"))
                .keys(doc! {
                    "board": 1,
                    "user": 1,
                })
                .build(),
            IndexModel::builder()
                .options(name("top-sort"))
                .keys(doc! {
                    "board": 1,
                    "score": 1,
                })
                .build()
        ]
    }
}
