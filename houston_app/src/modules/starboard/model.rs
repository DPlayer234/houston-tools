use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub _id: ObjectId,
    pub board: ChannelId,
    pub channel: ChannelId,
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
                    "score": 1,
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
                    "guild": 1,
                    "board": 1,
                    "user": 1,
                })
                .build(),
            IndexModel::builder()
                .options(name("top-sort"))
                .keys(doc! {
                    "guild": 1,
                    "board": 1,
                    "score": 1,
                })
                .build()
        ]
    }
}
