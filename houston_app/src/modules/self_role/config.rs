use std::collections::HashMap;
use std::num::NonZero;

use serenity::small_fixed_array::{FixedArray, FixedString};

use crate::prelude::*;

pub type Config = HashMap<GuildId, GuildConfig>;

#[derive(Debug, serde::Deserialize)]
pub struct GuildConfig {
    pub groups: FixedArray<RoleGroup>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RoleGroup {
    pub limit: Option<NonZero<u8>>,
    pub roles: FixedArray<RoleEntry, u8>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RoleEntry {
    pub id: RoleId,
    pub name: FixedString,
}
