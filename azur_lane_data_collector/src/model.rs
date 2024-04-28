use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Debug};
use mlua::prelude::*;
use once_cell::sync::Lazy;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

use crate::context;
use crate::convert_al;
use crate::enhance;
use crate::skill_loader;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub name_overrides: HashMap<u32, String>,
    pub predefined_skills: HashMap<u32, Skill>,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    serde_json::from_str(include_str!("assets/config.json")).unwrap()
});

#[derive(Debug, Clone)]
pub struct Group<'a> {
    pub id: u32,
    pub tables: Vec<ShipSet<'a>>
}

#[derive(Debug, Clone)]
pub struct ShipSet<'a> {
    pub id: u32,
    pub template: LuaTable<'a>,
    pub statistics: LuaTable<'a>,
    pub strengthen: Strengthen<'a>,
    pub retrofit_data: Option<Retrofit<'a>>
}

#[derive(Debug, Clone)]
pub struct ShipCandidate<'a> {
    pub id: u32,
    pub mlb: ShipSet<'a>,
    pub retrofits: Vec<ShipSet<'a>>,
    pub retrofit_data: Option<Retrofit<'a>>,
    pub skins: Vec<SkinSet<'a>>
}

#[derive(Debug, Clone)]
pub struct SkinSet<'a> {
    pub skin_id: u32,
    pub template: LuaTable<'a>,
    pub words: LuaTable<'a>,
    pub words_extra: Option<LuaTable<'a>>
}

#[derive(Debug, Clone)]
pub enum Strengthen<'a> {
    Normal(LuaTable<'a>),
    Blueprint(BlueprintStrengthen<'a>),
    META(MetaStrengthen<'a>)
}

#[derive(Debug, Clone)]
pub struct BlueprintStrengthen<'a> {
    pub data: LuaTable<'a>,
    pub effect_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
pub struct MetaStrengthen<'a> {
    pub data: LuaTable<'a>,
    pub repair_lookup: &'a LuaTable<'a>,
    pub repair_effect_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
pub struct Retrofit<'a> {
    pub data: LuaTable<'a>,
    pub list_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
pub enum DataError {
    NoMlb,
    NoStrengthen
}

impl Error for DataError {}
impl Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl ShipSet<'_> {
    pub fn to_ship_data(&self, lua: &Lua) -> LuaResult<ShipData> {
        macro_rules! read {
            ($table:expr, $field:expr) => {
                context!($table.get($field); "{} of ship with id {}", $field, self.id)?
            };
        }

        let attrs: LuaTable = read!(self.statistics, "attrs");
        let attrs_growth: LuaTable = read!(self.statistics, "attrs_growth");

        macro_rules! calc_stat {
            ($index:literal) => {{
                let base: f32 = attrs.get($index)?;
                let grow: f32 = attrs_growth.get($index)?;
                ShipStatValue::new(base, grow, 0f32)
            }};
        }

        macro_rules! get_non_grow_stat {
            ($index:literal) => {{
                let base: f32 = attrs.get($index)?;
                base
            }};
        }

        let base_list: LuaTable = read!(self.statistics, "base_list");
        let parallel_max: LuaTable = read!(self.statistics, "parallel_max");
        let preload_count: LuaTable = read!(self.statistics, "preload_count");
        let equipment_proficiency: LuaTable = read!(self.statistics, "equipment_proficiency");

        let mut buff_list: Vec<u32> = read!(self.template, "buff_list");
        let buff_list_display: Vec<u32> = read!(self.template, "buff_list_display");
        let hide_buff_list: Vec<u32> = read!(self.template, "hide_buff_list");
        intersect(&mut buff_list, &buff_list_display);

        let extra_main_guns: u8 =
            if hide_buff_list.contains(&1) { 1 }
            else if hide_buff_list.contains(&2) { 2 }
            else { 0 };

        macro_rules! make_equip_slot {
            ($allowed_at:literal, $index:literal) => {{
                let allow: Vec<u32> = read!(self.template, $allowed_at);
                let mut mounts: u8 = read!(base_list, $index);
                if $index == 1 { mounts += extra_main_guns; }
                
                EquipSlot {
                    allowed: allow.iter().map(|&n| convert_al::to_equip_type(n)).collect(),
                    mount: Some(EquipWeaponMount {
                        efficiency: read!(equipment_proficiency, $index),
                        mounts,
                        parallel: read!(parallel_max, $index),
                        preload: read!(preload_count, $index)
                    })
                }
            }};
            ($allowed_at:literal) => {{
                let allow: Vec<u32> = read!(self.template, $allowed_at);
                EquipSlot {
                    allowed: allow.iter().map(|&n| convert_al::to_equip_type(n)).collect(),
                    mount: None
                }
            }};
        }

        let mut ship = ShipData {
            group_id: read!(self.template, "group_type"),
            name: From::<String>::from(read!(self.statistics, "name")), 
            rarity: convert_al::to_rarity(read!(self.statistics, "rarity")),
            faction: convert_al::to_faction(read!(self.statistics, "nationality")),
            hull_type: convert_al::to_hull_type(read!(self.statistics, "type")),
            stars: read!(self.template, "star_max"),
            enhance_kind: EnhanceKind::Normal, // TODO
            stats: ShipStats {
                hp: calc_stat!(1),
                armor: convert_al::to_armor_type(read!(self.statistics, "armor_type")),
                rld: calc_stat!(6),
                fp: calc_stat!(2),
                trp: calc_stat!(3),
                eva: calc_stat!(9),
                aa: calc_stat!(4),
                avi: calc_stat!(5),
                acc: calc_stat!(8),
                asw: calc_stat!(12),
                spd: get_non_grow_stat!(10),
                lck: get_non_grow_stat!(11),
                cost: read!(self.template, "oil_at_end"),
                oxy: read!(self.statistics, "oxy_max"),
                amo: read!(self.statistics, "ammo")
            },
            equip_slots: vec![
                make_equip_slot!("equip_1", 1),
                make_equip_slot!("equip_2", 2),
                make_equip_slot!("equip_3", 3),
                make_equip_slot!("equip_4"),
                make_equip_slot!("equip_5")
            ],
            shadow_equip: skill_loader::load_equips(lua, read!(self.statistics, "fix_equip_list"))?.into_iter()
                .enumerate()
                .map(|(index, equip)| Ok(ShadowEquip {
                    name: equip.name,
                    efficiency: { let e: Option<f32> = equipment_proficiency.get(4 + index)?; e.unwrap_or(1f32) },
                    weapons: equip.weapons
                }))
                .collect::<LuaResult<Vec<_>>>()?,
            skills: skill_loader::load_skills(lua, buff_list)?,
            retrofits: Vec::new(),
            skins: Vec::new()
        };

        if ship.hull_type.data().team_type == TeamType::Submarine {
            // I can't explain it but submarine fleet ship costs seem to be 1 too high
            ship.stats.cost -= 1;
        }

        match &self.strengthen {
            Strengthen::Normal(data) => {
                // ship_data_strengthen
                ship.enhance_kind = EnhanceKind::Normal;
                add_strengthen_stats(&mut ship, &read!(data, "durability"))?;
            }
            Strengthen::Blueprint(ex) => {
                // ship_data_blueprint
                ship.enhance_kind = EnhanceKind::Research;

                let mut effects: Vec<u32> = read!(ex.data, "strengthen_effect");
                effects.append(&mut read!(ex.data, "fate_strengthen"));

                for id in effects {
                    enhance::blueprint::add_blueprint_effect(lua, &mut ship, &read!(ex.effect_lookup, id))?;
                }
            }
            Strengthen::META(ex) => {
                // ship_strengthen_meta
                ship.enhance_kind = EnhanceKind::META;

                for repair_part in ["repair_cannon", "repair_torpedo", "repair_air", "repair_reload"] {
                    let parts: Vec<u32> = read!(ex.data, repair_part);
                    for id in parts {
                        enhance::meta::add_repair(&mut ship, &read!(ex.repair_lookup, id))?;
                    }
                }

                let repair_effects: Vec<LuaTable> = read!(ex.data, "repair_effect");
                for table in repair_effects {
                    let id: u32 = table.get(2)?;
                    enhance::meta::add_repair_effect(&mut ship, &read!(ex.repair_effect_lookup, id))?;
                }

                // META ships have a definition for "buff_list_task" but this seems to go unused
                // and Fusou META doesn't even have the right data here. Just use the display list.
                ship.skills = skill_loader::load_skills(lua, buff_list_display)?;
            }
        }

        Ok(ship)
    }
}

fn add_strengthen_stats(ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    fn b(n: f32) -> ShipStatValue { ShipStatValue::new(n, 0f32, 0f32) }
    ship.stats.fp += { let v: f32 = table.get(1)?; b(v) };
    ship.stats.trp += { let v: f32 = table.get(2)?; b(v) };
    ship.stats.aa += { let v: f32 = table.get(3)?; b(v) };
    ship.stats.avi += { let v: f32 = table.get(4)?; b(v) };
    ship.stats.rld += { let v: f32 = table.get(5)?; b(v) };
    Ok(())
}

fn intersect<T: Eq>(target: &mut Vec<T>, other: &[T]) {
    let mut try_again = true;
    while try_again {
        try_again = false;
        for (index, item) in target.iter().enumerate() {
            if !other.contains(item) {
                target.remove(index);
                try_again = true;
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AugmentCandidate<'a> {
    pub id: u32,
    pub table: LuaTable<'a>
}

impl AugmentCandidate<'_> {
    pub fn to_augment(&self, lua: &Lua) -> LuaResult<Augment> {
        macro_rules! read {
            ($field:expr) => {
                context!(self.table.get($field); "{} of augment with id {}", $field, self.id)?
            };
        }

        macro_rules! read_stat {
            ($field:expr) => {{
                let temp: String = read!($field);
                convert_al::to_stat_kind(&temp)
            }};
        }

        let effect: u32 = read!("effect_id");
        let effect = match effect {
            0 => None,
            _ => Some(skill_loader::load_skill(lua, effect)?)
        };

        let skill_upgrade: Vec<LuaTable> = read!("skill_upgrade");
        let skill_upgrade = match skill_upgrade.into_iter().next() {
            Some(skill_upgrade) => {
                let skill_id: u32 = context!(skill_upgrade.get(2); "skill_upgrade id for augment {}", self.id)?;
                Some(skill_loader::load_skill(lua, skill_id)?)
            }
            None => None,
        };

        let unique_ship_id: u32 = read!("unique");
        let unique_ship_id = if unique_ship_id != 0 { Some(unique_ship_id) } else { None };

        Ok(Augment {
            augment_id: self.id,
            name: From::<String>::from(read!("name")),
            stat_bonuses: vec![
                AugmentStatBonus {
                    stat_kind: read_stat!("attribute_1"),
                    amount: read!("value_1"),
                    random: read!("value_1_random")
                },
                AugmentStatBonus {
                    stat_kind: read_stat!("attribute_2"),
                    amount: read!("value_2"),
                    random: read!("value_2_random")
                }
            ],
            allowed: {
                let allowed: Vec<u32> = read!("usability");
                allowed.into_iter().map(convert_al::to_hull_type).collect()
            },
            effect,
            unique_ship_id,
            skill_upgrade,
        })
    }
}
