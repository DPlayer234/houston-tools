use std::path::{Path, PathBuf};
use std::{env, fs, io};

use anyhow::{Context as _, Result};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use toml::map::Entry;
use toml::{Table, Value};

/// Provides a layered builder for deserializing configuration files.
#[must_use]
pub struct Builder {
    table: Result<Table>,
}

impl Builder {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self {
            table: Ok(Table::new()),
        }
    }

    /// Adds a layer of configuration.
    ///
    /// Layers added later take precedence over earlier ones.
    pub fn add_layer<L: Layer>(mut self, source: L) -> Self {
        self.table = self.table.and_then(|mut t| {
            source.extend_table(&mut t)?;
            Ok(t)
        });
        self
    }

    /// Deserializes the configuration from the provided layers.
    pub fn build<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.table.and_then(deserialize_table)
    }
}

/// A configuration layer.
pub trait Layer {
    /// Extends a TOML table by this layer.
    fn extend_table(&self, table: &mut Table) -> Result<()>;
}

/// A TOML file configuration layer.
#[must_use]
pub struct File {
    path: PathBuf,
    required: bool,
}

impl File {
    /// Creates a new layer, loading TOML from the file at the given path.
    ///
    /// The file is required by default.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        Self {
            path: PathBuf::from(path),
            required: true,
        }
    }

    /// Sets whether the file is required.
    ///
    /// If it is not required and does not exist, this layer is treated as
    /// empty. If it is required and does not exist, an error is raised.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

/// A TOML text configuration layer.
#[must_use]
pub struct TomlText<'a> {
    text: &'a str,
}

impl<'a> TomlText<'a> {
    /// Creates a new layer, parsing the text as TOML.
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }
}

/// An environment variable configuration layer.
///
/// This loads every environment variable. All their names are lowercased.
/// Nested values can be specified by using `__` (two underscores) as a
/// separator, (i.e. `DISCORD__TOKEN` will refer to `discord.token`).
///
/// Currently, all values are treated as strings. Values that are not fully
/// valid UTF-8 may be converted to UTF-8 with a lossy conversion.
#[must_use]
pub struct Env(());

impl Env {
    /// Creates a new layer.
    pub fn new() -> Self {
        Self(())
    }
}

impl Layer for File {
    fn extend_table(&self, table: &mut Table) -> Result<()> {
        let file = match fs::read_to_string(&self.path) {
            Ok(content) => deserialize_str_to_table(&content)
                .with_context(|| format!("failed to load config {:?}", self.path))?,
            Err(why) => {
                // on error, we definitely return and don't merge tables
                if !self.required && why.kind() == io::ErrorKind::NotFound {
                    return Ok(());
                }

                return Err(why).context(format!("cannot read required config {:?}", self.path));
            },
        };

        merge_tables(table, file);
        Ok(())
    }
}

impl Layer for TomlText<'_> {
    fn extend_table(&self, table: &mut Table) -> Result<()> {
        let toml = deserialize_str_to_table(self.text).context("toml str literal invalid")?;
        merge_tables(table, toml);
        Ok(())
    }
}

impl Layer for Env {
    fn extend_table(&self, table: &mut Table) -> Result<()> {
        for (key, value) in env::vars_os() {
            // non-utf8 keys cannot possibly refer to anything that serde or toml allows as
            // keys so they can just be excluded
            let Ok(mut key) = key.into_string() else {
                continue;
            };

            key.make_ascii_lowercase();

            // excluding values based on them not being utf8 isn't super great for error
            // reporting later, so just use lossy conversion so that at least gets seen.
            // also doesn't matter if the env var isn't used by this program
            let value = value
                .into_string()
                .unwrap_or_else(|o| o.to_string_lossy().into_owned());

            let segments = key.split("__").collect::<SmallVec<[&str; 8]>>();
            insert_at(table, &segments, Value::String(value));
        }

        Ok(())
    }
}

fn deserialize_str_to_table(text: &str) -> Result<Table> {
    toml::from_str(text).context("config toml is invalid")
}

fn deserialize_table<T>(table: Table) -> Result<T>
where
    T: DeserializeOwned,
{
    T::deserialize(table).context("cannot deserialize config")
}

fn merge_tables(target: &mut Table, consume: Table) {
    for (key, value) in consume {
        match target.entry(key) {
            Entry::Vacant(entry) => _ = entry.insert(value),
            Entry::Occupied(mut entry) => match (entry.get_mut(), value) {
                (Value::Table(a), Value::Table(b)) => merge_tables(a, b),
                (a, b) => *a = b,
            },
        }
    }
}

fn insert_at(table: &mut Table, path: &[&str], value: Value) {
    let [first, path @ ..] = path else {
        panic!("path must have at least one segment");
    };

    match table.entry(first.to_owned()) {
        Entry::Vacant(entry) => _ = entry.insert(nested_value(path, value)),
        Entry::Occupied(mut entry) => match entry.get_mut() {
            Value::Table(table) if !path.is_empty() => insert_at(table, path, value),
            entry => *entry = nested_value(path, value),
        },
    }
}

fn nested_value(path: &[&str], value: Value) -> Value {
    let [path @ .., last] = path else {
        return value;
    };

    let mut table = Table::new();
    let mut cur = &mut table;
    for &segment in path {
        cur = cur
            .entry(segment.to_owned())
            .or_insert(Value::Table(Table::new()))
            .as_table_mut()
            .expect("just inserted as a table");
    }

    cur.insert((*last).to_owned(), value);
    Value::Table(table)
}
