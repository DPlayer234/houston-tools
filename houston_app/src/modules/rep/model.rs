use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
pub struct Record {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(with = "id_as_i64")]
    pub user: UserId,
    #[serde(with = "id_as_i64")]
    pub guild: GuildId,
    #[serde(default)]
    pub received: i64,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub cooldown_ends: DateTime<Utc>,
}

fn name(name: &str) -> IndexOptions {
    IndexOptions::builder().name(name.to_owned()).build()
}

impl RecordPartial {
    pub fn init(self, user: UserId, guild: GuildId) -> Self {
        self.user(user)
            .guild(guild)
            .cooldown_ends(DateTime::UNIX_EPOCH)
    }
}

impl ModelCollection for Record {
    const COLLECTION_NAME: &str = "rep.record";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name("user-guild"))
                .keys(Self::sort().user(Asc).guild(Asc))
                .build(),
        ]
    }
}
