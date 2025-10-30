//! Traits and functions for defining or using the database model.

use anyhow::Context as _;
use bson::Document;
use mongodb::error::{CommandError, Error, ErrorKind, WriteError, WriteFailure};
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

    /// Creates the indices for this model.
    ///
    /// If there is a spec mismatch, drop and recreates the affected indices.
    fn update_indices(db: &Database) -> impl Future<Output = anyhow::Result<()>> {
        update_indices(Self::collection_raw(db), Self::indices)
    }
}

/// Determines whether the error code is `11000 (DuplicateKey)`.
///
/// This can be used to upsert a document with a filter that includes more than
/// just unique-index fields. If this error is encountered, it means that the
/// document already exists, but doesn't match the non-unique-index predicates.
pub fn is_upsert_duplicate_key(err: &Error) -> bool {
    // can show up for both command and write errors, for some reason
    matches!(
        *err.kind,
        ErrorKind::Command(CommandError { code: 11000, .. })
            | ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. }))
    )
}

/// Shared non-generic logic for [`ModelCollection::update_indices`].
///
/// This attempts to create all indices in bulk. If there is a conflict, falls
/// back to creating the indices one-by-one, dropping and recreating any
/// individual ones that run into a conflict. This function includes logging for
/// this case.
///
/// The indices are provided as a function pointer so they can be created a
/// second time as needed rather than having to be cloned every time.
async fn update_indices(
    collection: Collection<Document>,
    indices_fn: fn() -> Vec<IndexModel>,
) -> anyhow::Result<()> {
    /// Whether to try to drop the index and recreate it. This just checks
    /// whether the command error kind is `86 (IndexKeySpecsConflict)`.
    fn is_recreate(err: &Error) -> bool {
        matches!(*err.kind, ErrorKind::Command(CommandError { code: 86, .. }))
    }

    /// Slow path for [`update_indices`]. Indices are created one-by-one and, if
    /// a conflict arises, are dropped and then created again.
    async fn recreate_indices(
        collection: Collection<Document>,
        indices_fn: fn() -> Vec<IndexModel>,
    ) -> anyhow::Result<()> {
        for index in indices_fn() {
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

    let indices = indices_fn();
    if indices.is_empty() {
        return Ok(());
    }

    // attempt to create all indices in bulk first
    // this will usually succeed, so we can save some round-trips
    // if we can attempt a recreate, try the indices individually
    match collection.create_indexes(indices).await {
        Ok(_) => Ok(()),
        Err(err) if is_recreate(&err) => recreate_indices(collection, indices_fn).await,
        Err(err) => Err(err).context("could not create indices"),
    }
}
