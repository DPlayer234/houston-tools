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

fn name_unique(name: &str) -> IndexOptions {
    IndexOptions::builder()
        .name(name.to_owned())
        .unique(true)
        .build()
}

impl ModelCollection for Record {
    const COLLECTION_NAME: &str = "rep.record";

    fn indices() -> Vec<IndexModel> {
        vec![
            IndexModel::builder()
                .options(name_unique("user-guild"))
                .keys(Self::sort().user(Asc).guild(Asc))
                .build(),
        ]
    }
}
