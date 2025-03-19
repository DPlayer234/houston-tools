use anyhow::Context as _;
use bson::Document;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database, IndexModel};

/// Declares a type as being a collection type in MongoDB.
pub trait ModelCollection {
    /// The name of the MongoDB collection.
    const COLLECTION_NAME: &str;

    /// Gets the collection for this type on the given database.
    fn collection(db: &Database) -> Collection<Self>
    where
        Self: Sized + Send + Sync,
    {
        db.collection(Self::COLLECTION_NAME)
    }

    /// Gets the collection for this type on the given database without
    /// ascribing the type, instead using a raw [`Document`] collection.
    fn collection_raw(db: &Database) -> Collection<Document> {
        db.collection(Self::COLLECTION_NAME)
    }

    /// Gets the indices to create for this collection. This must always return
    /// the same values.
    ///
    /// To apply the indices, call [`update_indices`].
    fn indices() -> Vec<IndexModel> {
        Vec::new()
    }
}

/// Creates the indices for model `M`.
///
/// If there is a spec mismatch, drop and recreates the affected indices.
pub async fn update_indices<M>(db: &Database) -> anyhow::Result<()>
where
    M: ModelCollection,
{
    use mongodb::error::{CommandError, Error, ErrorKind};

    // match for command error kind 86 `IndexKeySpecsConflict`
    // in this case, we can probably just drop the index and recreate it
    fn is_recreate(err: &Error) -> bool {
        matches!(*err.kind, ErrorKind::Command(CommandError { code: 86, .. }))
    }

    async fn update_indices_inner(
        collection: Collection<Document>,
        indices: Vec<IndexModel>,
    ) -> anyhow::Result<()> {
        for index in indices {
            match collection.create_index(index.clone()).await {
                Ok(_) => {},
                Err(err) if is_recreate(&err) => {
                    let name = match &index.options {
                        Some(IndexOptions {
                            name: Some(name), ..
                        }) => name.as_str(),
                        _ => return Err(err).context("must set index name to attempt re-create"),
                    };

                    log::trace!("Detected index {}/{} mismatch.", collection.name(), name);
                    collection.drop_index(name).await?;
                    let create = collection.create_index(index).await?;
                    log::info!(
                        "Replaced index {}/{}.",
                        collection.name(),
                        create.index_name
                    );
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }

    let indices = M::indices();
    if indices.is_empty() {
        return Ok(());
    }

    // attempt to create all indices in bulk first
    // this will usually succeed, so we can save some round-trips
    // if we can attempt a recreate, try the indices individually
    let collection = M::collection_raw(db);
    match collection.create_indexes(indices).await {
        Ok(_) => Ok(()),
        Err(err) if is_recreate(&err) => update_indices_inner(collection, M::indices()).await,
        Err(err) => Err(err).context("could not create indices"),
    }
}

/// Serializes a Discord ID as an [`i64`].
pub mod id_as_i64 {
    use serde::de::Error as _;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: From<u64>,
    {
        #[allow(clippy::cast_sign_loss)]
        let int = i64::deserialize(deserializer)? as u64;
        if int != u64::MAX {
            Ok(T::from(int))
        } else {
            Err(D::Error::custom("invalid discord id"))
        }
    }

    pub fn serialize<S, T>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Into<i64> + Copy,
    {
        let int: i64 = (*val).into();
        int.serialize(serializer)
    }
}
