use anyhow::Context;
use mongodb::{Client, Collection};

use model::starboard;

pub mod model;

#[derive(Debug)]
#[non_exhaustive]
pub struct Database {
    pub starboard_messages: Collection<starboard::Message>,
    pub starboard_scores: Collection<starboard::Score>,
}

impl Database {
    pub async fn connect(uri: &str) -> anyhow::Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let database = client.default_database().context("no default database specified")?;

        let starboard_messages = database.collection("starboard.messages");
        starboard_messages.create_indexes(starboard::Message::indices()).await?;

        let starboard_scores = database.collection("starboard.scores");
        starboard_scores.create_indexes(starboard::Score::indices()).await?;

        Ok(Self {
            starboard_messages,
            starboard_scores,
        })
    }
}
