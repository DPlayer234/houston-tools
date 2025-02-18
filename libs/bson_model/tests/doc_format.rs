use bson::oid::ObjectId;
use bson::{doc, Document};
use bson_model::{Filter, ModelDocument, Sort};
use bson_model_macros as _;
use serde::{Deserialize, Serialize, Serializer};

fn serialize_inverse<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
    (!value).serialize(serializer)
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
struct Example {
    #[serde(rename = "_id")]
    id: ObjectId,
    // ensure that `serialize_with` works for additional types
    #[serde(serialize_with = "serialize_inverse")]
    user: i64,
    // ensure that renaming works
    #[serde(rename = "game_score")]
    score: u32,
}

#[test]
fn partial_full() {
    let oid = ObjectId::new();
    let user = 123456i64;
    let score = 42u32;

    let partial = Example::partial()
        .id(oid)
        .user(user)
        .score(score)
        .into_document()
        .unwrap();

    assert_eq!(
        partial,
        doc! {
            "_id": oid,
            "user": (!user),
            "game_score": i64::from(score),
        }
    );
}

#[test]
fn partial_just_id() {
    let oid = ObjectId::new();

    let partial = Example::partial().id(oid).into_document().unwrap();

    assert_eq!(
        partial,
        doc! {
            "_id": oid,
        }
    );
}

#[test]
fn filter_full() {
    let oid = ObjectId::new();
    let user = Filter::in_([12345i64, 23456i64]);
    let score = Filter::Gt(42u32);

    let filter = Example::filter()
        .id(oid)
        .user(user)
        .score(score)
        .into_document()
        .unwrap();

    assert_eq!(
        filter,
        doc! {
            "_id": oid,
            "user": {
                "$in": [!12345i64, !23456i64]
            },
            "game_score": {
                "$gt": 42i64
            },
        }
    );
}

#[test]
fn sort() {
    let sort = Example::sort()
        .score(Sort::Desc)
        .user(Sort::Asc)
        .into_document();

    assert_eq!(
        sort,
        doc! {
            "game_score": -1,
            "user": 1,
        }
    );
}

#[test]
fn update() {
    let user = 123456i64;

    let update = Example::update()
        .set(|s| s.user(user))
        .set_on_insert(|s| s.user(user))
        .inc(|s| s.score(5))
        .max(|s| s.score(10000))
        .min(|s| s.score(0))
        .into_document()
        .unwrap();

    assert_eq!(
        update,
        doc! {
            "$set": {
                "user": (!user),
            },
            "$setOnInsert": {
                "user": (!user),
            },
            "$inc": {
                "game_score": 5i64,
            },
            "$max": {
                "game_score": 10000i64,
            },
            "$min": {
                "game_score": 0i64,
            },
        }
    )
}
