use std::{borrow::Borrow, collections::HashSet, sync::Arc};

use mlua::prelude::*;
use azur_lane::skill::*;
use azur_lane::equip::*;

use crate::context;
use crate::convert_al;

pub fn load_skill(lua: &Lua, skill_id: u32) -> LuaResult<Skill> {
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let skill_data_template: LuaTable = context!(pg.get("skill_data_template"); "global pg.skill_data_template")?;

    let skill: LuaTable = context!(skill_data_template.get(skill_id); "skill with id {}", skill_id)?;
    let name: String = context!(skill.get("name"); "name of skill with id {}", skill_id)?;
    let mut desc: String = context!(skill.get("desc"); "desc of skill with id {}", skill_id)?;
    let desc_add: Vec<Vec<Vec<String>>> = context!(skill.get("desc_add"); "desc_add of skill with id {}", skill_id)?;

    for (slot, data_set) in desc_add.iter().enumerate() {
        if let Some(last) = data_set.last() {
            if let Some(text) = last.first() {
                let placeholder = format!("${}", slot + 1);
                desc = desc.replace(&placeholder, text);
            }
        }
    }

    let buff = require_buff_data(lua, skill_id)?;

    let category: String = context!(buff.get("color"); "color of buff with id {skill_id}")?;
    let category = convert_al::to_skill_category(&category);

    let mut context = ReferencedWeaponsContext::default();
    search_referenced_weapons(&mut context, lua, buff, skill_id)?;

    Ok(Skill {
        buff_id: skill_id,
        category,
        name: Arc::from(name),
        description: Arc::from(desc),
        barrages: Arc::from(context.attacks)
    })
}

pub fn load_skills(lua: &Lua, skill_ids: Vec<u32>) -> LuaResult<Vec<Skill>> {
    skill_ids.into_iter().map(|id| load_skill(lua, id)).collect()
}

pub fn load_equip(lua: &Lua, equip_id: u32) -> LuaResult<Equip> {
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let equip_data_statistics: LuaTable = context!(pg.get("equip_data_statistics"); "global pg.equip_data_statistics")?;

    let equip_data: LuaTable = context!(equip_data_statistics.get(equip_id); "equip statistics for id {equip_id}")?;
    let weapon_ids: Vec<u32> = context!(equip_data.get("weapon_id"); "weapon_id for equip with id {equip_id}")?;
    let name: String = context!(equip_data.get("name"); "name for equip with id {equip_id}")?;

    let mut weapons = Vec::new();
    for weapon_id in weapon_ids {
        if let Some(weapon) = load_weapon(lua, weapon_id)? {
            weapons.push(weapon);
        }
    }

    Ok(Equip {
        name: Arc::from(name),
        kind: convert_al::to_equip_type(context!(equip_data.get("type"); "type for equip with id {equip_id}")?),
        faction: convert_al::to_faction(context!(equip_data.get("nationality"); "nationality for equip with id {equip_id}")?),
        hull_allowed: Arc::new([]), // todo
        hull_disallowed: Arc::new([]), // todo
        weapons: Arc::from(weapons),
        stat_bonuses: Arc::new([]) // todo
    })
}

pub fn load_equips(lua: &Lua, equip_ids: Vec<u32>) -> LuaResult<Vec<Equip>> {
    equip_ids.into_iter().map(|id| load_equip(lua, id)).collect()
}

pub fn load_weapon(lua: &Lua, weapon_id: u32) -> LuaResult<Option<Weapon>> {
    const RLD_MULT_AT_100: f32 = 0.006650724f32;

    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let weapon_property: LuaTable = context!(pg.get("weapon_property"); "global pg.weapon_property")?;
    let weapon_data: LuaTable = context!(weapon_property.get(weapon_id); "weapon property for id {weapon_id}")?;

    let weapon_type: u32 = context!(weapon_data.get("type"); "weapon type in weapon {weapon_id}")?;
    let reload_max: f32 = weapon_data.get("reload_max")?;

    let data = match RoughWeaponType::from(weapon_type) {
        RoughWeaponType::Bullet => {
            let mut bullets: Vec<Bullet> = Vec::new();

            let bullet_ids: Vec<u32> = context!(weapon_data.get("bullet_ID"); "bullet id in weapon {weapon_id}")?;
            let barrage_ids: Vec<u32> = context!(weapon_data.get("barrage_ID"); "barrage id in weapon {weapon_id}")?;

            for (bullet_id, barrage_id) in bullet_ids.into_iter().zip(barrage_ids) {
                get_sub_barrage(lua, &mut bullets, bullet_id, barrage_id, 1)?;
            }

            WeaponData::Bullets(Barrage {
                damage: weapon_data.get("damage")?,
                coefficient: { let raw: f32 = weapon_data.get("corrected")?; raw / 100f32 },
                scaling: { let raw: f32 = weapon_data.get("attack_attribute_ratio")?; raw / 100f32 },
                scaling_stat: convert_al::weapon_attack_attr_to_stat_kind(weapon_data.get("attack_attribute")?),
                range: weapon_data.get("range")?,
                firing_angle: weapon_data.get("angle")?,
                bullets: Arc::from(bullets)
            })
        },
        RoughWeaponType::Aircraft => {
            let aircraft_template: LuaTable = context!(pg.get("aircraft_template"); "global pg.aircraft_template")?;
            let aircraft: LuaTable = context!(aircraft_template.get(weapon_id); "aircraft_template for id {weapon_id}")?;
            
            let barrage_template: LuaTable = context!(pg.get("barrage_template"); "global pg.barrage_template")?;
            let barrage_ids: Vec<u32> = context!(weapon_data.get("barrage_ID"); "barrage id in weapon {weapon_id}")?;

            let mut amount = 0u32;
            for barrage_id in barrage_ids {
                let barrage: LuaTable = context!(barrage_template.get(barrage_id); "barrage template for id {barrage_id}")?;
                
                let senior_repeat: u32 = context!(barrage.get("senior_repeat"); "senior_repeat in barrage {barrage_id}")?;
                let primal_repeat: u32 = context!(barrage.get("primal_repeat"); "primal_repeat in barrage {barrage_id}")?;

                amount += (senior_repeat + 1) * (primal_repeat + 1);
            }

            let speed: f32 = aircraft.get("speed")?;
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
                weapons: Arc::from(weapons)
            })
        },
        _ => { return Ok(None); }
    };

    Ok(Some(Weapon {
        weapon_id,
        reload_time: reload_max * RLD_MULT_AT_100,
        fixed_delay: weapon_data.get("recover_time")?,
        data,
    }))
}

fn get_sub_barrage(lua: &Lua, bullets: &mut Vec<Bullet>, bullet_id: u32, barrage_id: u32, parent_amount: u32) -> LuaResult<()> {
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let bullet_template: LuaTable = context!(pg.get("bullet_template"); "global pg.bullet_template")?;
    let barrage_template: LuaTable = context!(pg.get("barrage_template"); "global pg.barrage_template")?;

    let bullet: LuaTable = context!(bullet_template.get(bullet_id); "bullet template for id {bullet_id}")?;
    let barrage: LuaTable = context!(barrage_template.get(barrage_id); "barrage template for id {barrage_id}")?;
        
    let senior_repeat: u32 = context!(barrage.get("senior_repeat"); "senior_repeat in barrage {barrage_id}")?;
    let primal_repeat: u32 = context!(barrage.get("primal_repeat"); "primal_repeat in barrage {barrage_id}")?;

    let amount = (senior_repeat + 1) * (primal_repeat + 1) * parent_amount;

    if let LuaValue::Table(extra_param) = bullet.get("extra_param")? {
        let shrapnel: Option<Vec<LuaTable>> = extra_param.get("shrapnel")?;
        if let Some(shrapnel) = shrapnel {
            for emitter in shrapnel {
                let bullet_id: u32 = context!(emitter.get("bullet_ID"); "bullet id in emitter for bullet")?;
                let barrage_id: u32 = context!(emitter.get("barrage_ID"); "barrage id in emitter for bullet")?;
                get_sub_barrage(lua, bullets, bullet_id, barrage_id, amount)?;
            }

            return Ok(());
        }
    }

    if let Some(existing) = bullets.iter_mut().find(|b| b.bullet_id == bullet_id) {
        existing.amount += amount;
    } else {
        let armor_mods: [f32; 3] = bullet.get("damage_type")?;
        let pierce: Option<u32> = context!(bullet.get("pierce_amount"); "pierce_amount in bullet {bullet_id}")?;
        let kind = convert_al::to_bullet_kind(bullet.get("type")?);

        let mut attach_buff = Vec::new();
        let attach_buff_raw: Option<Vec<LuaTable>> = context!(bullet.get("attach_buff"); "attach_buff in bullet {bullet_id}")?;
        if let Some(attach_buff_raw) = attach_buff_raw {
            for buff in attach_buff_raw {
                let buff_id: u32 = buff.get("buff_id")?;
                let probability: Option<f32> = buff.get("rant" /* sic */)?;
                let level: Option<u32> = buff.get("level")?;

                attach_buff.push(BuffInfo {
                    buff_id,
                    probability: probability.map(|f| f * 0.0001).unwrap_or(1f32),
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
            attach_buff: Arc::from(attach_buff),

            spread: if kind == BulletKind::Bomb {
                let hit_type: LuaTable = context!(bullet.get("hit_type"); "hit_type in bullet {bullet_id}")?;
                let extra_param: LuaTable = context!(bullet.get("extra_param"); "extra_param in bullet {bullet_id}")?;

                let spread_x: Option<f32> = context!(extra_param.get("randomOffsetX"); "randomOffsetX in bullet {bullet_id}")?;
                let spread_y: Option<f32> = context!(extra_param.get("randomOffsetZ"); "randomOffsetZ in bullet {bullet_id}")?;

                Some(BulletSpread {
                    spread_x: spread_x.unwrap_or_default(),
                    spread_y: spread_y.unwrap_or_default(),
                    hit_range: context!(hit_type.get("range"); "range in bullet {bullet_id}")?
                })
            } else {
                None
            }
        });
    }

    Ok(())
}

fn search_referenced_weapons(barrages: &mut ReferencedWeaponsContext, lua: &Lua, skill: LuaTable, skill_id: u32) -> LuaResult<()> {
    let len = skill.len()?;
    if let Ok(len) = usize::try_from(len) {
        if len != 0 {
            let level_entry: LuaTable = context!(skill.get(len); "level entry {len} of skill/buff")?;
            let effect_list: Option<Vec<LuaTable>> = context!(level_entry.get("effect_list"); "effect_list of skill/buff level entry {len}")?;
            if let Some(effect_list) = effect_list {
                search_referenced_weapons_in_effect_entry(barrages, lua, effect_list, skill_id)?;
                return Ok(());
            }
        }
    }

    let effect_list: Option<Vec<LuaTable>> = context!(skill.get("effect_list"); "effect_list of skill/buff")?;
    if let Some(effect_list) = effect_list {
        search_referenced_weapons_in_effect_entry(barrages, lua, effect_list, skill_id)?;
    }

    Ok(())
}

fn search_referenced_weapons_in_effect_entry(barrages: &mut ReferencedWeaponsContext, lua: &Lua, effect_list: Vec<LuaTable>, skill_id: u32) -> LuaResult<()> {
    fn get_arg<'a, T: FromLua<'a>>(entry: &LuaTable<'a>, key: &str) -> LuaResult<T> {
        let arg_list: LuaTable = context!(entry.get("arg_list"); "skill/buff effect_list entry arg_list")?;
        context!(arg_list.get(key); "skill/buff effect_list entry arg_list {}", key)
    }

    let mut seen_weapons = HashSet::new();
    let mut attacks = Vec::new();

    for entry in effect_list {
        let entry_type: String = context!(entry.get("type"); "skill/buff effect_list entry type: {:#?}", entry)?;
        match entry_type.borrow() {
            "BattleBuffCastSkill" => {
                let skill_id: u32 = get_arg(&entry, "skill_id")?;
                if barrages.seen_skills.insert(skill_id) {
                    let skill = require_skill_data(lua, skill_id)?;
                    search_referenced_weapons(barrages, lua, skill, skill_id)?;
                }
            }
            "BattleBuffCastSkillRandom" => {
                let skill_id_list: Option<Vec<u32>> = get_arg(&entry, "skill_id_list")?;
                if let Some(skill_id_list) = skill_id_list {
                    for skill_id in skill_id_list {
                        if barrages.seen_skills.insert(skill_id) {
                            let skill = require_skill_data(lua, skill_id)?;
                            search_referenced_weapons(barrages, lua, skill, skill_id)?;
                        }
                    }
                }
            }
            "BattleBuffAddBuff" | "BattleSkillAddBuff" => {
                let buff_id: u32 = get_arg(&entry, "buff_id")?;
                if barrages.seen_buffs.insert(buff_id) {
                    let buff = require_buff_data(lua, buff_id)?;
                    search_referenced_weapons(barrages, lua, buff, buff_id)?;
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
            _ => ()
        }
    }

    if !attacks.is_empty() {
        barrages.attacks.push(SkillBarrage {
            skill_id,
            attacks: Arc::from(attacks)
        })
    }

    Ok(())
}

fn require_buff_data(lua: &Lua, buff_id: u32) -> LuaResult<LuaTable> {
    lua.globals().call_function("require_buff", buff_id)
}

fn require_skill_data(lua: &Lua, skill_id: u32) -> LuaResult<LuaTable> {
    lua.globals().call_function("require_skill", skill_id)
}

#[derive(Debug, Default)]
struct ReferencedWeaponsContext {
    attacks: Vec<SkillBarrage>,
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
        match num {
            1 | 2 | 3 | 16 | 17 | 19 | 23 | 24 | 25 | 28 | 29 | 31 | 32 | 33 => RoughWeaponType::Bullet,
            10 | 11 => RoughWeaponType::Aircraft,
            4 | 22 | 26 | 30 => RoughWeaponType::AntiAir,
            18 => RoughWeaponType::Melee,
            _ => RoughWeaponType::Irrelevant
        }
    }
}
