use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Not;

use bson::doc;
use bson::oid::ObjectId;
use bson_model::{Filter, ModelDocument, Sort};
use serde::{Deserialize, Serialize, Serializer};
use {bson_model_macros as _, serde_with as _, small_fixed_array as _};

fn serialize_inverse<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Not + Copy,
    S: Serializer,
    T::Output: Serialize,
{
    (!*value).serialize(serializer)
}

fn serialize_normal<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    value.serialize(serializer)
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

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
struct ForFilter {
    basic: i32,
    #[serde(serialize_with = "serialize_inverse")]
    with: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
struct Generic<T: ModelDocument> {
    #[serde(rename = "_id")]
    id: ObjectId,
    data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
struct Borrowing<'a> {
    #[serde(rename = "_id")]
    id: ObjectId,
    data: Cow<'a, str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
#[serde(bound(deserialize = "T::Owned: for<'d> Deserialize<'d>"))]
struct BorrowingGeneric<'a, T>
where
    T: ToOwned,
    T::Owned: std::fmt::Debug,
{
    #[serde(rename = "_id")]
    id: ObjectId,
    data: Cow<'a, T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ModelDocument)]
struct All<'a, A, B, const N: usize>
where
    A: Serialize,
{
    cow: Cow<'a, str>,
    #[serde(serialize_with = "serialize_normal")]
    r#gen: A,
    #[serde(skip)]
    _ignore: PhantomData<&'a (A, B, [(); N])>,
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
        .expect("example partial should serialize");

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

    let partial = Example::partial()
        .id(oid)
        .into_document()
        .expect("example partial should serialize");

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
        .expect("example filter should serialize");

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
        .expect("example update should serialize");

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

#[test]
fn serialize_filter() {
    macro_rules! sub {
        ($fn:ident, $name:literal, $input:tt, $output:tt) => {{
            let f = ForFilter::filter()
                .basic(Filter::$fn($input))
                .with(Filter::$fn($input))
                .into_document()
                .unwrap();

            assert_eq!(
                f,
                doc! {
                    "basic": {
                        $name: $input,
                    },
                    "with": {
                        $name: $output,
                    },
                }
            );
        }};
    }

    let f_eq = ForFilter::filter()
        .basic(64)
        .with(64)
        .into_document()
        .expect("for filter should serialize");

    assert_eq!(
        f_eq,
        doc! {
            "basic": 64i32,
            "with": !64i32,
        }
    );

    sub!(Ne, "$ne", 64i32, (!64i32));
    sub!(Gt, "$gt", 64i32, (!64i32));
    sub!(Gte, "$gte", 64i32, (!64i32));
    sub!(Lt, "$lt", 64i32, (!64i32));
    sub!(Lte, "$lte", 64i32, (!64i32));
    sub!(in_, "$in", [64i32, 42i32], [!64i32, !42i32]);
    sub!(not_in, "$nin", [64i32, 42i32], [!64i32, !42i32]);
}

#[test]
fn supports_generic_types() {
    type X<'a> = All<'a, i64, (), 0>;

    let _update = X::update()
        .set(|u| u.cow("hello".into()))
        .set_on_insert(|u| u.r#gen(0))
        .into_document()
        .expect("must support update");

    let _filter = X::filter()
        .cow(Cow::from("hello"))
        .r#gen(Filter::Lt(4))
        .into_document()
        .expect("must support filter");

    let _sort = X::sort().cow(Sort::Asc).r#gen(Sort::Desc).into_document();
}
