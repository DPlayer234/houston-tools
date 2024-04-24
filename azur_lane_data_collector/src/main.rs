use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Debug};
use std::fs;
use std::sync::Arc;
use mlua::prelude::*;

use azur_lane::*;
use azur_lane::ship::*;

mod macros;
mod convert_al;
mod enhance;
mod skill_loader;

const MAX_LEVEL: u32 = 125;
const EXTRA_GROWTH_START: u32 = 100;

fn main() -> Result<(), Box<dyn Error>> {
    let lua = Lua::new();

    lua.load(include_str!("assets/lua_init.lua"))
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .exec()?;

    println!("Init done.");

    // General:
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let ship_data_template: LuaTable = context!(pg.get("ship_data_template"); "global pg.ship_data_template")?;
    let ship_data_template_all: LuaTable = context!(ship_data_template.get("all"); "global pg.ship_data_template.all")?;
    let ship_data_statistics: LuaTable = context!(pg.get("ship_data_statistics"); "global pg.ship_data_statistics")?;

    // Normal enhancement data (may be present even if not used for that ship):
    let ship_data_strengthen: LuaTable = context!(pg.get("ship_data_strengthen"); "global pg.ship_data_strengthen")?;

    // Blueprint/Research ship data:
    let ship_data_blueprint: LuaTable = context!(pg.get("ship_data_blueprint"); "global pg.ship_data_blueprint")?;
    let ship_strengthen_blueprint: LuaTable = context!(pg.get("ship_strengthen_blueprint"); "global pg.ship_strengthen_blueprint")?;

    // META ship data:
    let ship_strengthen_meta: LuaTable = context!(pg.get("ship_strengthen_meta"); "global pg.ship_strengthen_meta")?;
    let ship_meta_repair: LuaTable = context!(pg.get("ship_meta_repair"); "global pg.ship_meta_repair")?;
    let ship_meta_repair_effect: LuaTable = context!(pg.get("ship_meta_repair_effect"); "global pg.ship_meta_repair_effect")?;

    // Retrofit data:
    let ship_data_trans: LuaTable = context!(pg.get("ship_data_trans"); "global pg.ship_data_trans")?;
    let transform_data_template: LuaTable = context!(pg.get("transform_data_template"); "global pg.transform_data_template")?;

    let mut groups = HashMap::new();
    ship_data_template_all.for_each(|_: u32, id: u32| {
        if id >= 900000 && id <= 900999 {
            return Ok(())
        }

        let template: LuaTable = context!(ship_data_template.get(id); "ship_data_template with id {id}")?;
        let statistics: LuaTable = context!(ship_data_statistics.get(id); "ship_data_statistics with id {id}")?;
        
        let group_id: u32 = context!(template.get("group_type"); "group_type of ship_data_template with id {id}")?;
        let strengthen_id: u32 = context!(template.get("strengthen_id"); "strengthen_id of ship_data_template with id {id}")?;
        let _: u32 = context!(template.get("id"); "id of ship_data_template with id {id}")?;
        
        let enhance: Option<LuaTable> = context!(ship_data_strengthen.get(strengthen_id); "ship_data_strengthen with {id}")?;
        let blueprint: Option<LuaTable> = context!(ship_data_blueprint.get(strengthen_id); "ship_data_blueprint with {id}")?;
        let meta: Option<LuaTable> = context!(ship_strengthen_meta.get(strengthen_id); "ship_strengthen_meta with {id}")?;

        let strengthen = match (enhance, blueprint, meta) {
            (_, Some(data), _) => Strengthen::Blueprint(BlueprintStrengthen { data, effect_lookup: &ship_strengthen_blueprint }),
            (_, _, Some(data)) => Strengthen::META(MetaStrengthen { data, repair_lookup: &ship_meta_repair, repair_effect_lookup: &ship_meta_repair_effect }),
            (Some(data), _, _) => Strengthen::Normal(data),
            _ => Err(LuaError::external(DataError::NoStrengthen))?
        };

        let retrofit: Option<LuaTable> = context!(ship_data_trans.get(strengthen_id); "ship_data_trans with {id}")?;
        let retrofit = retrofit.map(|r| Retrofit { data: r, list_lookup: &transform_data_template });

        let ship = ShipSet {
            id,
            template,
            statistics,
            strengthen,
            retrofit_data: retrofit
        };

        groups.entry(group_id)
            .or_insert_with(|| Group { id: group_id, tables: Vec::new() })
            .tables.push(ship);

        Ok(())
    })?;

    let mut candidates = Vec::new();
    for group in groups.into_values() {
        let mlb_max_id = group.id * 10 + 4;
        let Some(mlb) = group.tables.iter().filter(|t| t.id <= mlb_max_id).max_by_key(|t| t.id) else {
            Err(LuaError::external(DataError::NoMlb).with_context(|_| format!("no mlb for ship with id {}", group.id)))?
        };

        let retrofits = group.tables.iter().filter(|t| t.id > mlb.id).cloned().collect();

        candidates.push(ShipCandidate {
            id: group.id,
            mlb: mlb.clone(),
            retrofits,
            retrofit_data: mlb.retrofit_data.clone()
        });
    }

    candidates.sort_by_key(|t| t.id);
    println!("candidates: {}", candidates.len());

    let mut ships = HashMap::new();
    for candidate in candidates {
        let mut mlb = candidate.mlb.to_ship_data(&lua)?;
        
        let mut retrofits: Vec<ShipData> = Vec::new();
        if let Some(ref retrofit_data) = candidate.retrofit_data {
            if retrofits.is_empty() {
                let mut retrofit = mlb.clone();
                enhance::retrofit::apply_retrofit(&lua, &mut retrofit, &retrofit_data)?;

                fix_up_retrofitted_data(&mut retrofit, &candidate.mlb)?;
                retrofits.push(retrofit); 
            }

            for retrofit_set in candidate.retrofits {
                let mut retrofit = retrofit_set.to_ship_data(&lua)?;
                enhance::retrofit::apply_retrofit(&lua, &mut retrofit, &retrofit_data)?;
    
                fix_up_retrofitted_data(&mut retrofit, &retrofit_set)?;
                retrofits.push(retrofit);
            }
        }

        mlb.retrofits = Arc::from(retrofits);
        ships.insert(candidate.id, mlb);
    }

    println!("Writing output...");
    let f = fs::File::create("houston_azur_lane_data.json")?;
    serde_json::to_writer(&f, &DefinitionData {
        ships
    })?;

    println!("Written {} bytes.", f.metadata()?.len());
    drop(f);

    Ok(())
}

#[derive(Debug, Clone)]
struct Group<'a> {
    id: u32,
    tables: Vec<ShipSet<'a>>
}

#[derive(Debug, Clone)]
struct ShipSet<'a> {
    id: u32,
    template: LuaTable<'a>,
    statistics: LuaTable<'a>,
    strengthen: Strengthen<'a>,
    retrofit_data: Option<Retrofit<'a>>
}

#[derive(Debug, Clone)]
struct ShipCandidate<'a> {
    id: u32,
    mlb: ShipSet<'a>,
    retrofits: Vec<ShipSet<'a>>,
    retrofit_data: Option<Retrofit<'a>>
}

#[derive(Debug, Clone)]
enum Strengthen<'a> {
    Normal(LuaTable<'a>),
    Blueprint(BlueprintStrengthen<'a>),
    META(MetaStrengthen<'a>)
}

#[derive(Debug, Clone)]
struct BlueprintStrengthen<'a> {
    data: LuaTable<'a>,
    effect_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
struct MetaStrengthen<'a> {
    data: LuaTable<'a>,
    repair_lookup: &'a LuaTable<'a>,
    repair_effect_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
pub struct Retrofit<'a> {
    data: LuaTable<'a>,
    list_lookup: &'a LuaTable<'a>
}

#[derive(Debug, Clone)]
enum DataError {
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
    fn to_ship_data(&self, lua: &Lua) -> LuaResult<ShipData> {
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

        macro_rules! make_equip_slot {
            ($allowed_at:literal, $index:literal) => {{
                let allow: Vec<u32> = read!(self.template, $allowed_at);
                EquipSlot {
                    allowed: allow.iter().map(|&n| convert_al::to_equip_type(n)).collect(),
                    mount: Some(EquipWeaponMount {
                        efficiency: read!(equipment_proficiency, $index),
                        mounts: read!(base_list, $index),
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
                        efficiency: { let e: Option<f32> = equipment_proficiency.get(4 + index)?; e.unwrap_or_default() },
                        weapons: equip.weapons
                    }))
                    .collect::<LuaResult<Vec<_>>>()?
            ),
            skills: Arc::from(
                skill_loader::load_skills(lua, read!(self.template, "buff_list"))?
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
                    skill_loader::load_skills(lua, read!(self.template, "buff_list_display"))?
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

fn fix_up_retrofitted_data(ship: &mut ShipData, set: &ShipSet) -> LuaResult<()> {
    let buff_list_display: Vec<u32> = set.template.get("buff_list_display")?;
    let mut skills = ship.skills.to_vec();
    skills.sort_by_key(|s| buff_list_display.iter().enumerate().find(|i| *i.1 == s.buff_id).map(|i| i.0).unwrap_or_default());
    ship.skills = Arc::from(skills);

    Ok(())
}
