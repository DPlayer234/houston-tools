//! Data model used while parsing game data.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::LazyLock;

use azur_lane::skill::*;
use mlua::prelude::*;

/// The config model.
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Overrides for ship names based on their group ID.
    pub name_overrides: HashMap<u32, String>,
    /// Names for the special secretary kinds.
    pub special_secretary_kinds: Vec<String>,
    /// Overrides for skills based on their buff ID.
    pub predefined_skills: HashMap<u32, Skill>,
}

/// The app config. Statically embed as JSON.
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/config.json")).expect("config must be valid")
});

/// A group of ships.
#[derive(Debug)]
pub struct ShipGroup {
    /// The ID of the group, aka `group_type`.
    pub id: u32,
    /// The IDs of the members.
    pub members: Vec<u32>,
}

/// A set of data from which [`ShipData`] can be constructed.
///
/// [`ShipData`]: azur_lane::ship::ShipData
#[derive(Debug)]
pub struct ShipSet<'a> {
    /// The ship ID. Not the group's.
    pub id: u32,
    /// The `ship_data_template` entry.
    pub template: LuaTable,
    /// The `ship_data_statistics` entry.
    pub statistics: LuaTable,
    /// The associated strengthen data.
    pub strengthen: Strengthen<'a>,
    /// The associated retrofit data.
    pub retrofit_data: Option<Retrofit<'a>>,
}

/// A set of data from which [`ShipSkin`] can be constructed.
///
/// [`ShipSkin`]: azur_lane::ship::ShipSkin
#[derive(Debug)]
pub struct SkinSet {
    /// The skin ID.
    pub skin_id: u32,
    /// The `ship_skin_template` entry.
    pub template: LuaTable,
    /// The `ship_skin_words` entry.
    pub words: LuaTable,
    /// The `ship_skin_words_extra` entry.
    pub words_extra: Option<LuaTable>,
}

/// The strengthen data.
#[derive(Debug)]
pub enum Strengthen<'a> {
    /// Normal. Holds the `ship_data_strengthen` entry.
    Normal(LuaTable),
    /// Research.
    Blueprint(BlueprintStrengthen<'a>),
    // META.
    Meta(MetaStrengthen<'a>),
}

/// Strengthen data for a research ship.
#[derive(Debug)]
pub struct BlueprintStrengthen<'a> {
    /// The `ship_data_blueprint` entry.
    pub data: LuaTable,
    /// A reference to `ship_strengthen_blueprint`.
    pub effect_lookup: &'a LuaTable,
}

/// Strengthen data for a META ship.
#[derive(Debug)]
pub struct MetaStrengthen<'a> {
    /// The `ship_strengthen_meta` entry.
    pub data: LuaTable,
    /// A reference to `ship_meta_repair`.
    pub repair_lookup: &'a LuaTable,
    /// A reference to `ship_meta_repair_effect`.
    pub repair_effect_lookup: &'a LuaTable,
}

/// Retrofit data some ship.
#[derive(Debug)]
pub struct Retrofit<'a> {
    /// The `ship_data_trans` entry.
    pub data: LuaTable,
    /// A reference to `transform_data_template`.
    pub list_lookup: &'a LuaTable,
}

/// A set of data from which [`Augment`] can be constructed.
///
/// [`Augment`]: azur_lane::equip::Augment
#[derive(Debug)]
pub struct AugmentSet {
    /// The augment's ID.
    pub id: u32,
    /// The `spweapon_data_statistics` entry.
    pub statistics: LuaTable,
}

/// An error when loading the data.
#[derive(Debug)]
pub enum DataError {
    /// There is no state that appears to be the max limit break.
    NoMlb,
    /// There is no strengthen data of any kind.
    NoStrengthen,
}

impl Error for DataError {}
impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMlb => f.write_str("no mlb state found"),
            Self::NoStrengthen => f.write_str("no strengthen info found"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    #[test]
    fn static_config() {
        LazyLock::force(&super::CONFIG);
    }
}
