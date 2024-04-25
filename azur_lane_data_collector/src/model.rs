use std::error::Error;
use std::fmt::{Display, Debug};
use std::sync::Arc;
use mlua::prelude::*;
use azur_lane::ship::*;

use crate::context;
use crate::convert_al;
use crate::enhance;
use crate::skill_loader;

const MAX_LEVEL: u32 = 125;
const EXTRA_GROWTH_START: u32 = 100;

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
    pub retrofit_data: Option<Retrofit<'a>>
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
        let attrs_growth_extra: LuaTable = read!(self.statistics, "attrs_growth_extra");

        macro_rules! calc_stat {
            ($index:literal) => {{
                let base: f32 = attrs.get($index)?;
                let grow: f32 = attrs_growth.get($index)?;
                let grow_ex: f32 = attrs_growth_extra.get($index)?;
                
                base + (grow * (MAX_LEVEL - 1) as f32 + grow_ex * (MAX_LEVEL - EXTRA_GROWTH_START) as f32) / 1000f32
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
                spd: calc_stat!(10),
                lck: calc_stat!(11),
                cost: read!(self.template, "oil_at_end"),
                oxy: read!(self.statistics, "oxy_max"),
                amo: read!(self.statistics, "ammo")
            },
            equip_slots: Arc::new([
                make_equip_slot!("equip_1", 1),
                make_equip_slot!("equip_2", 2),
                make_equip_slot!("equip_3", 3),
                make_equip_slot!("equip_4"),
                make_equip_slot!("equip_5")
            ]),
            shadow_equip: Arc::from(
                skill_loader::load_equips(lua, read!(self.statistics, "fix_equip_list"))?.into_iter()
                    .enumerate()
                    .map(|(index, equip)| Ok(ShadowEquip {
                        name: equip.name,
                        efficiency: { let e: Option<f32> = equipment_proficiency.get(4 + index)?; e.unwrap_or_default() },
                        weapons: equip.weapons
                    }))
                    .collect::<LuaResult<Vec<_>>>()?
            ),
            skills: Arc::from(
                skill_loader::load_skills(lua, buff_list)?
            ),
            retrofits: Arc::new([]),
            wiki_name: None
        };

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
                ship.skills = Arc::from(
                    skill_loader::load_skills(lua, buff_list_display)?
                );
            }
        }

        Ok(ship)
    }
}

fn add_strengthen_stats(ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    ship.stats.fp += { let v: f32 = table.get(1)?; v };
    ship.stats.trp += { let v: f32 = table.get(2)?; v };
    ship.stats.aa += { let v: f32 = table.get(3)?; v };
    ship.stats.avi += { let v: f32 = table.get(4)?; v };
    ship.stats.rld += { let v: f32 = table.get(5)?; v };
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