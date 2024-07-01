use std::collections::HashSet;

use mlua::prelude::*;

use azur_lane::skill::*;
use azur_lane::equip::*;

use crate::context;
use crate::CONFIG;
use crate::convert_al;

/// Loads a skill from the Lua state.
pub fn load_skill(lua: &Lua, skill_id: u32) -> LuaResult<Skill> {
    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;
    let skill_data_template: LuaTable = pg.get("skill_data_template").context("global pg.skill_data_template")?;

    let skill: LuaTable = skill_data_template.get(skill_id).with_context(context!("skill with id {}", skill_id))?;
    let name: String = skill.get("name").with_context(context!("name of skill with id {}", skill_id))?;
    let mut desc: String = skill.get("desc").with_context(context!("desc of skill with id {}", skill_id))?;
    let desc_add: Vec<Vec<Vec<String>>> = skill.get("desc_add").with_context(context!("desc_add of skill with id {}", skill_id))?;

    for (slot, data_set) in desc_add.iter().enumerate() {
        if let Some(last) = data_set.last() {
            if let Some(text) = last.first() {
                let placeholder = format!("${}", slot + 1);
                desc = desc.replace(&placeholder, text);
            }
        }
    }

    if let Some(skill) = CONFIG.predefined_skills.get(&skill_id) {
        let mut skill = skill.clone();
        skill.name = name;
        skill.description = desc;

        return Ok(skill);
    }

    let category: u32 = skill.get("type").with_context(context!("type of skill with id {skill_id}"))?;
    let category = convert_al::to_skill_category(category);

    let buff = require_buff_data(lua, skill_id)?;
    let mut context = ReferencedWeaponsContext::default();
    search_referenced_weapons(&mut context, lua, buff, skill_id)?;

    Ok(Skill {
        buff_id: skill_id,
        category,
        name,
        description: desc,
        barrages: context.barrages,
        new_weapons: context.new_weapons,
    })
}

/// Loads skills from the Lua state.
pub fn load_skills(lua: &Lua, skill_ids: Vec<u32>) -> LuaResult<Vec<Skill>> {
    skill_ids.into_iter().map(|id| load_skill(lua, id)).collect()
}

/// Loads a piece of equipment from the Lua state.
pub fn load_equip(lua: &Lua, equip_id: u32) -> LuaResult<Equip> {
    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;
    let equip_data_statistics: LuaTable = pg.get("equip_data_statistics").context("global pg.equip_data_statistics")?;

    let equip_data: LuaTable = equip_data_statistics.get(equip_id).with_context(context!("equip statistics for id {equip_id}"))?;
    let weapon_ids: Vec<u32> = equip_data.get("weapon_id").with_context(context!("weapon_id for equip with id {equip_id}"))?;
    let skill_ids: Vec<u32> = equip_data.get("skill_id").with_context(context!("skill_id for equip with id {equip_id}"))?;
    let name: String = equip_data.get("name").with_context(context!("name for equip with id {equip_id}"))?;
    let description: String = equip_data.get("descrip").with_context(context!("descrip for equip with id {equip_id}"))?;

    let mut weapons = Vec::new();
    for weapon_id in weapon_ids {
        if let Some(weapon) = load_weapon(lua, weapon_id)? {
            weapons.push(weapon);
        }
    }

    let skills = skill_ids.into_iter()
        .map(|id| load_skill(lua, id))
        .collect::<LuaResult<Vec<_>>>()?;

    macro_rules! stat_bonus {
        ($index:literal) => {{
            match equip_data.get(concat!("attribute_", $index)).with_context(context!("attribute_{} for equip with id {equip_id}", $index))? {
                Some(stat_kind) => {
                    let stat_kind: String = stat_kind;
                    Some(EquipStatBonus {
                        stat_kind: convert_al::to_stat_kind(&stat_kind),
                        amount: equip_data.get(concat!("value_", $index)).with_context(context!("value_{} for equip with id {equip_id}", $index))?
                    })
                }
                None => None
            }
        }};
    }

    Ok(Equip {
        equip_id,
        name,
        description,
        rarity: convert_al::to_equip_rarity(equip_data.get("rarity").with_context(context!("rarity for equip with id {equip_id}"))?),
        kind: convert_al::to_equip_type(equip_data.get("type").with_context(context!("type for equip with id {equip_id}"))?),
        faction: convert_al::to_faction(equip_data.get("nationality").with_context(context!("nationality for equip with id {equip_id}"))?),
        hull_allowed: Vec::new(), // todo
        hull_disallowed: Vec::new(), // todo
        weapons,
        skills,
        stat_bonuses: [stat_bonus!(1), stat_bonus!(2), stat_bonus!(3)].into_iter().flatten().collect()
    })
}

/// Loads equipment pieces from the Lua state.
pub fn load_equips(lua: &Lua, equip_ids: Vec<u32>) -> LuaResult<Vec<Equip>> {
    equip_ids.into_iter().map(|id| load_equip(lua, id)).collect()
}

/// Loads a weapon from the Lua state.
pub fn load_weapon(lua: &Lua, weapon_id: u32) -> LuaResult<Option<Weapon>> {
    const RLD_MULT_AT_100: f64 = 0.006650724;

    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;
    let weapon_property: LuaTable = pg.get("weapon_property").context("global pg.weapon_property")?;
    let weapon_data: LuaTable = weapon_property.get(weapon_id).with_context(context!("weapon property for id {weapon_id}"))?;
    let weapon_name = get_weapon_name(&pg, weapon_id)?;

    let weapon_type: u32 = weapon_data.get("type").with_context(context!("weapon type in weapon {weapon_id}"))?;
    let reload_max: f64 = weapon_data.get("reload_max")?;
    let mut fixed_delay = weapon_data.get("auto_aftercast")?;

    let kind = convert_al::to_weapon_kind(weapon_type);
    let data = match RoughWeaponType::from(weapon_type) {
        RoughWeaponType::Bullet => {
            WeaponData::Bullets(get_barrage(lua, weapon_id, &weapon_data)?)
        }
        RoughWeaponType::AntiAir => {
            let mut barrage = get_barrage(lua, weapon_id, &weapon_data)?;

            if !matches!(kind, WeaponKind::AirToAir) {
                fixed_delay = 0.8667;
            }

            // It appears that AA barrage data indicates AA guns fire twice.
            // But this doesn't happen because AA guns work way differently.
            for bullet in &mut barrage.bullets {
                bullet.amount -= 1;
            }

            WeaponData::AntiAir(barrage)
        }
        RoughWeaponType::Aircraft => {
            let aircraft_template: LuaTable = pg.get("aircraft_template").context("global pg.aircraft_template")?;
            let aircraft: LuaTable = aircraft_template.get(weapon_id).with_context(context!("aircraft_template for id {weapon_id}"))?;

            let barrage_template: LuaTable = pg.get("barrage_template").context("global pg.barrage_template")?;
            let barrage_ids: Vec<u32> = weapon_data.get("barrage_ID").with_context(context!("barrage id in weapon {weapon_id}"))?;

            let mut amount = 0u32;
            for barrage_id in barrage_ids {
                let barrage: LuaTable = barrage_template.get(barrage_id).with_context(context!("barrage template for id {barrage_id}"))?;

                let senior_repeat: u32 = barrage.get("senior_repeat").with_context(context!("senior_repeat in barrage {barrage_id}"))?;
                let primal_repeat: u32 = barrage.get("primal_repeat").with_context(context!("primal_repeat in barrage {barrage_id}"))?;

                amount += (senior_repeat + 1) * (primal_repeat + 1);
            }

            let speed: f64 = aircraft.get("speed")?;
            let weapons: Vec<u32> = aircraft.get("weapon_ID")?;
            let weapons = weapons.into_iter()
                .map(|id| load_weapon(lua, id))
                .collect::<LuaResult<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            WeaponData::Aircraft(Aircraft {
                aircraft_id: weapon_id,
                amount,
                speed,
                weapons
            })
        }
        _ => { return Ok(None); }
    };

    Ok(Some(Weapon {
        weapon_id,
        name: weapon_name,
        reload_time: reload_max * RLD_MULT_AT_100,
        fixed_delay,
        kind,
        data,
    }))
}

fn get_weapon_name(pg: &LuaTable, weapon_id: u32) -> LuaResult<Option<String>> {
    let weapon_name: LuaTable = pg.get("weapon_name").context("global pg.weapon_name")?;
    let weapon_name: Option<LuaTable> = weapon_name.get(weapon_id).with_context(context!("weapon_name for id {weapon_id}"))?;
    match weapon_name {
        Some(weapon_name) => weapon_name.get("name").with_context(context!("name of weapon_name for id {weapon_id}")),
        None => Ok(None),
    }
}

fn get_barrage(lua: &Lua, weapon_id: u32, weapon_data: &LuaTable) -> LuaResult<Barrage> {
    let mut bullets: Vec<Bullet> = Vec::new();
    let bullet_ids: Vec<u32> = weapon_data.get("bullet_ID").with_context(context!("bullet id in weapon {weapon_id}"))?;
    let barrage_ids: Vec<u32> = weapon_data.get("barrage_ID").with_context(context!("barrage id in weapon {weapon_id}"))?;

    let mut salvo_time = 0.0;
    let mut bullet_time = 0.0;
    for (bullet_id, barrage_id) in bullet_ids.into_iter().zip(barrage_ids) {
        bullet_time = get_sub_barrage(lua, &mut bullets, &mut salvo_time, bullet_id, barrage_id, 1)?;
    }

    salvo_time -= bullet_time;

    Ok(Barrage {
        damage: weapon_data.get("damage")?,
        coefficient: { let raw: f64 = weapon_data.get("corrected")?; raw * 0.01 },
        scaling: { let raw: f64 = weapon_data.get("attack_attribute_ratio")?; raw * 0.01 },
        scaling_stat: convert_al::weapon_attack_attr_to_stat_kind(weapon_data.get("attack_attribute")?),
        range: weapon_data.get("range")?,
        firing_angle: weapon_data.get("angle")?,
        salvo_time,
        bullets
    })
}

fn get_sub_barrage(lua: &Lua, bullets: &mut Vec<Bullet>, salvo_time: &mut f64, bullet_id: u32, barrage_id: u32, parent_amount: u32) -> LuaResult<f64> {
    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;
    let bullet_template: LuaTable = pg.get("bullet_template").context("global pg.bullet_template")?;
    let barrage_template: LuaTable = pg.get("barrage_template").context("global pg.barrage_template")?;

    let bullet: LuaTable = bullet_template.get(bullet_id).with_context(context!("bullet template for id {bullet_id}"))?;
    let barrage: LuaTable = barrage_template.get(barrage_id).with_context(context!("barrage template for id {barrage_id}"))?;

    let senior_delay: f64 = barrage.get("senior_delay").with_context(context!("senior_delay in barrage {barrage_id}"))?;
    let senior_repeat: u32 = barrage.get("senior_repeat").with_context(context!("senior_repeat in barrage {barrage_id}"))?;
    let primal_repeat: u32 = barrage.get("primal_repeat").with_context(context!("primal_repeat in barrage {barrage_id}"))?;

    let amount = (senior_repeat + 1) * (primal_repeat + 1) * parent_amount;
    *salvo_time += f64::from(senior_repeat + 1) * senior_delay;

    if let LuaValue::Table(extra_param) = bullet.get("extra_param")? {
        let shrapnel: Option<Vec<LuaTable>> = extra_param.get("shrapnel")?;
        if let Some(shrapnel) = shrapnel {
            for emitter in shrapnel {
                let bullet_id: u32 = emitter.get("bullet_ID").context("bullet id in emitter for bullet")?;
                let barrage_id: u32 = emitter.get("barrage_ID").context("barrage id in emitter for bullet")?;
                get_sub_barrage(lua, bullets, &mut 0.0, bullet_id, barrage_id, amount)?;
            }

            return Ok(senior_delay);
        }
    }

    if let Some(existing) = bullets.iter_mut().find(|b| b.bullet_id == bullet_id) {
        existing.amount += amount;
    } else {
        let armor_mods: [f64; 3] = bullet.get("damage_type")?;
        let pierce: Option<u32> = bullet.get("pierce_amount").with_context(context!("pierce_amount in bullet {bullet_id}"))?;
        let kind = convert_al::to_bullet_kind(bullet.get("type")?);

        let mut attach_buff = Vec::new();
        let attach_buff_raw: Option<Vec<LuaTable>> = bullet.get("attach_buff").with_context(context!("attach_buff in bullet {bullet_id}"))?;
        if let Some(attach_buff_raw) = attach_buff_raw {
            for buff in attach_buff_raw {
                let buff_id: u32 = buff.get("buff_id")?;
                let probability: Option<f64> = buff.get("rant" /* sic */)?;
                let level: Option<u32> = buff.get("level")?;

                attach_buff.push(BuffInfo {
                    buff_id,
                    probability: probability.map(|f| f * 0.0001).unwrap_or(1f64),
                    level: level.unwrap_or(1)
                })
            }
        }

        bullets.push(Bullet {
            bullet_id,
            amount,
            kind,
            ammo: convert_al::to_ammo_kind(bullet.get("ammo_type")?),
            pierce: pierce.unwrap_or_default(),
            velocity: bullet.get("velocity")?,
            modifiers: ArmorModifiers::from(armor_mods),
            attach_buff,

            spread: if kind == BulletKind::Bomb {
                let hit_type: LuaTable = bullet.get("hit_type").with_context(context!("hit_type in bullet {bullet_id}"))?;
                let extra_param: LuaTable = bullet.get("extra_param").with_context(context!("extra_param in bullet {bullet_id}"))?;

                let spread_x: Option<f64> = extra_param.get("randomOffsetX").with_context(context!("randomOffsetX in bullet {bullet_id}"))?;
                let spread_y: Option<f64> = extra_param.get("randomOffsetZ").with_context(context!("randomOffsetZ in bullet {bullet_id}"))?;

                Some(BulletSpread {
                    spread_x: spread_x.unwrap_or_default(),
                    spread_y: spread_y.unwrap_or_default(),
                    hit_range: hit_type.get("range").with_context(context!("range in bullet {bullet_id}"))?
                })
            } else {
                None
            }
        });
    }

    Ok(senior_delay)
}

fn search_referenced_weapons(context: &mut ReferencedWeaponsContext, lua: &Lua, skill: LuaTable, skill_id: u32) -> LuaResult<()> {
    let len = skill.len()?;
    if let Ok(len) = usize::try_from(len) {
        if len != 0 {
            let level_entry: LuaTable = skill.get(len).with_context(context!("level entry {len} of skill/buff"))?;
            let effect_list: Option<Vec<LuaTable>> = level_entry.get("effect_list").with_context(context!("effect_list of skill/buff level entry {len}"))?;
            if let Some(effect_list) = effect_list {
                search_referenced_weapons_in_effect_entry(context, lua, effect_list, skill_id)?;
                return Ok(());
            }
        }
    }

    let effect_list: Option<Vec<LuaTable>> = skill.get("effect_list").context("effect_list of skill/buff")?;
    if let Some(effect_list) = effect_list {
        search_referenced_weapons_in_effect_entry(context, lua, effect_list, skill_id)?;
    }

    Ok(())
}

fn search_referenced_weapons_in_effect_entry(context: &mut ReferencedWeaponsContext, lua: &Lua, effect_list: Vec<LuaTable>, skill_id: u32) -> LuaResult<()> {
    fn get_arg<'a, T: FromLua<'a>>(entry: &LuaTable<'a>, key: &str) -> LuaResult<T> {
        let arg_list: LuaTable = entry.get("arg_list").context("skill/buff effect_list entry arg_list")?;
        arg_list.get(key).with_context(context!("skill/buff effect_list entry arg_list {}", key))
    }

    let mut seen_weapons = HashSet::new();
    let mut attacks = Vec::new();

    for entry in effect_list {
        let entry_type: String = entry.get("type").with_context(context!("skill/buff effect_list entry type: {:#?}", entry))?;
        match entry_type.as_str() {
            "BattleBuffCastSkill" => {
                let skill_id: u32 = get_arg(&entry, "skill_id")?;
                if context.seen_skills.insert(skill_id) {
                    let skill = require_skill_data(lua, skill_id)?;
                    search_referenced_weapons(context, lua, skill, skill_id)?;
                }
            }
            "BattleBuffCastSkillRandom" => {
                let skill_id_list: Option<Vec<u32>> = get_arg(&entry, "skill_id_list")?;
                if let Some(skill_id_list) = skill_id_list {
                    for skill_id in skill_id_list {
                        if context.seen_skills.insert(skill_id) {
                            let skill = require_skill_data(lua, skill_id)?;
                            search_referenced_weapons(context, lua, skill, skill_id)?;
                        }
                    }
                }
            }
            "BattleBuffAddBuff" | "BattleSkillAddBuff" => {
                let buff_id: u32 = get_arg(&entry, "buff_id")?;
                if context.seen_buffs.insert(buff_id) {
                    let buff = require_buff_data(lua, buff_id)?;
                    search_referenced_weapons(context, lua, buff, buff_id)?;
                }
            }
            "BattleSkillFire" => {
                let weapon_id: u32 = get_arg(&entry, "weapon_id")?;
                if seen_weapons.insert(weapon_id) {
                    let target: LuaValue = entry.get("target_choise" /* sic */)?;
                    let target = match target {
                        LuaValue::String(s) => s.to_str()?.to_owned(),
                        LuaValue::Table(t) => t.get(1)?,
                        _ => String::new()
                    };

                    let target = convert_al::to_skill_target(&target);
                    if let Some(weapon) = load_weapon(lua, weapon_id)? {
                        attacks.push(SkillAttack { weapon, target });
                    }
                }
            }
            "BattleBuffNewWeapon" => {
                let weapon_id: u32 = get_arg(&entry, "weapon_id")?;
                if !context.new_weapons.iter().any(|w| w.weapon_id == weapon_id) {
                    if let Some(weapon) = load_weapon(lua, weapon_id)? {
                        context.new_weapons.push(weapon);
                    }
                }
            }
            _ => ()
        }
    }

    if !attacks.is_empty() {
        context.barrages.push(SkillBarrage {
            skill_id,
            attacks
        });
    }

    Ok(())
}

/// Calls our "require_buff" Lua helper to get buff data.
fn require_buff_data(lua: &Lua, buff_id: u32) -> LuaResult<LuaTable> {
    lua.globals().call_function("require_buff", buff_id)
}

/// Calls our "require_skill" Lua helper to get skill data.
fn require_skill_data(lua: &Lua, skill_id: u32) -> LuaResult<LuaTable> {
    lua.globals().call_function("require_skill", skill_id)
}

#[derive(Debug, Default)]
struct ReferencedWeaponsContext {
    barrages: Vec<SkillBarrage>,
    new_weapons: Vec<Weapon>,
    seen_skills: HashSet<u32>,
    seen_buffs: HashSet<u32>,
}

#[derive(Debug, Clone, Copy)]
enum RoughWeaponType {
    Bullet,
    Aircraft,
    AntiAir,
    Melee,
    Irrelevant
}

impl RoughWeaponType {
    fn from(num: u32) -> RoughWeaponType {
        // note: 24 is BEAM, might need other handling
        // note: 4 is air-to-air attacks
        match num {
            1 | 2 | 3 | 16 | 17 | 19 | 23 | 24 | 25 | 28 | 29 | 31 | 32 | 33 => RoughWeaponType::Bullet,
            10 | 11 => RoughWeaponType::Aircraft,
            4 | 22 | 26 | 30 => RoughWeaponType::AntiAir,
            18 => RoughWeaponType::Melee,
            _ => RoughWeaponType::Irrelevant
        }
    }
}
