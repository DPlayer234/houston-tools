use super::BoardId;
use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct Message {
    #[serde(rename = "_id")]
    pub id: ObjectId,
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

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct Score {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub board: BoardId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub post_count: i64,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder().name(name.to_owned()).build()
}

impl Message {
    pub fn self_filter(&self) -> Document {
        doc! {
            "_id": self.id,
        }
    }
}

impl ModelCollection for Message {
    const COLLECTION_NAME: &str = "starboard.messages";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name("board-message"))
                .keys(Self::sort().board(Asc).message(Asc))
                .build(),
            IndexModel::builder()
                .options(name("top-posts-sort"))
                .keys(Self::sort().board(Asc).max_reacts(Asc).message(Asc))
                .build(),
            IndexModel::builder()
                .options(name("top-posts-user-sort"))
                .keys(
                    Self::sort()
                        .board(Asc)
                        .user(Asc)
                        .max_reacts(Asc)
                        .message(Asc),
                )
                .build(),
        ]
    }
}

impl ModelCollection for Score {
    const COLLECTION_NAME: &str = "starboard.scores";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name("board-user"))
                .keys(Self::sort().board(Asc).user(Asc))
                .build(),
            IndexModel::builder()
                .options(name("top-sort"))
                .keys(Self::sort().board(Asc).score(Asc).post_count(Asc))
                .build(),
        ]
    }
}
