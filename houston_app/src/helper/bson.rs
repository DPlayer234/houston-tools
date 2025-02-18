use anyhow::Context as _;
use bson::Document;
use mongodb::options::IndexOptions;
use mongodb::{Collection, IndexModel};

/// Creates the specified indices.
///
/// If there is a spec mismatch, drop and recreates the affected indices.
pub async fn update_indices<T>(
    collection: Collection<T>,
    indices: Vec<IndexModel>,
) -> anyhow::Result<()>
where
    T: Send + Sync,
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

    // attempt to create all indices in bulk first
    // this will usually succeed, so we can save some round-trips
    // if we can attempt a recreate, try the indices individually
    match collection.create_indexes(indices.iter().cloned()).await {
        Ok(_) => Ok(()),
        Err(err) if is_recreate(&err) => {
            update_indices_inner(collection.clone_with_type(), indices).await
        },
        Err(err) => Err(err).context("could not create indices"),
    }
}

/// Serializes a Discord ID as an [`i64`].
pub mod id_as_i64 {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
