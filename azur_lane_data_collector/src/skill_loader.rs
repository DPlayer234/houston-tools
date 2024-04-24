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
    let mut context = ReferencedWeaponsContext::default();
    search_referenced_weapons(&mut context, lua, buff)?;

    Ok(Skill {
        buff_id: skill_id,
        name: Arc::from(name),
        description: Arc::from(desc),
        weapons: Arc::from(context.attacks)
    })
}

pub fn load_skills(lua: &Lua, skill_ids: Vec<u32>) -> LuaResult<Vec<Skill>> {
    skill_ids.into_iter().map(|id| load_skill(lua, id)).collect()
}

// WIP:
pub fn load_weapon(lua: &Lua, weapon_id: u32) -> LuaResult<Option<Weapon>> {
    const RLD_MULT_AT_100: f32 = 0.29533408f32;

    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
    let weapon_property: LuaTable = context!(pg.get("weapon_property"); "global pg.weapon_property")?;
    let bullet_template: LuaTable = context!(pg.get("bullet_template"); "global pg.bullet_template")?;
    let barrage_template: LuaTable = context!(pg.get("barrage_template"); "global pg.barrage_template")?;
    let aircraft_template: LuaTable = context!(pg.get("aircraft_template"); "global pg.aircraft_template")?;

    let weapon_data: LuaTable = context!(weapon_property.get(weapon_id); "weapon property for id {weapon_id}")?;

    let bullet: LuaTable = context!(weapon_data.get("bullet_ID"); "bullet id in weapon {weapon_id}")?;
    let bullet: Option<u32> = context!(bullet.get(1); "first bullet id in weapon {weapon_id}")?;
    let Some(bullet) = bullet else {
        return Ok(None);
    };

    let reload_max: f32 = weapon_data.get("reload_max")?;

    let data =
    if let Some(bullet) = { let bullet: Option<LuaTable> = context!(bullet_template.get(bullet); "bullet template for id {bullet}")?; bullet } {
        let barrage: LuaTable = context!(weapon_data.get("barrage_ID"); "barrage id in weapon {weapon_id}")?;
        let barrage: u32 = context!(barrage.get(1); "first barrage id in weapon {weapon_id}")?;
        let barrage: LuaTable = context!(barrage_template.get(barrage); "barrage template for id {barrage}")?;

        let mut armor_mods: [f32; 3] = bullet.get("damage_type")?;

        let mut kind: BulletKind = BulletKind::HE; // todo
        let mut pierce: Option<u32> = context!(bullet.get("pierce_amount"); "pierce_amount in weapon {weapon_id}")?;

        let senior_repeat: u32 = context!(barrage.get("senior_repeat"); "senior_repeat in weapon {weapon_id}")?;
        let primal_repeat: u32 = context!(barrage.get("primal_repeat"); "primal_repeat in weapon {weapon_id}")?;
        let mut amount = (senior_repeat + 1) * (primal_repeat + 1);

        if let LuaValue::Table(extra_param) = bullet.get("extra_param")? {
            let shrapnel: Option<Vec<LuaTable>> = extra_param.get("shrapnel")?;
            if let Some(shrapnel) = shrapnel {
                let mut sub_mult = 0u32;
                for emitter in shrapnel {
                    let bullet: u32 = context!(emitter.get("bullet_ID"); "bullet id in emitter for bullet")?;
                    let bullet: LuaTable = context!(bullet_template.get(bullet); "bullet template for id {bullet} in emitter")?;

                    let barrage: u32 = context!(emitter.get("barrage_ID"); "barrage id in emitter for bullet")?;
                    let barrage: LuaTable = context!(barrage_template.get(barrage); "barrage template for id {barrage} in emitter")?;

                    // todo: set kind of based on shrapnel bullet
                    armor_mods = bullet.get("damage_type")?;

                    let sub_pierce: Option<u32> = context!(bullet.get("pierce_amount"); "pierce_amount in emitter for bullet")?;
                    pierce = sub_pierce.max(pierce);

                    let senior_repeat: u32 = context!(barrage.get("senior_repeat"); "senior_repeat in emitter for bullet")?;
                    let primal_repeat: u32 = context!(barrage.get("primal_repeat"); "primal_repeat in emitter for bullet")?;
                    sub_mult += (senior_repeat + 1) * (primal_repeat + 1);
                }

                amount *= sub_mult;
            }
        }

        WeaponData::Bullet(Bullet {
            damage: weapon_data.get("damage")?,
            coefficient: weapon_data.get("corrected")?,
            scaling: { let raw: f32 = weapon_data.get("attack_attribute_ratio")?; raw / 100f32 },
            scaling_stat: convert_al::to_stat_kind(weapon_data.get("attack_attribute")?),
            range: weapon_data.get("range")?,
            firing_angle: weapon_data.get("angle")?,
    
            pierce: pierce.unwrap_or_default(),
            kind,
            velocity: bullet.get("velocity")?,
            modifiers: ArmorModifiers(armor_mods[0], armor_mods[1], armor_mods[2]),
    
            amount,
        })
    } else {
        let aircraft: LuaTable = context!(aircraft_template.get(bullet); "bullet aircraft_template for id {bullet}")?;
        let speed: f32 = aircraft.get("speed")?;
        let weapons: Vec<u32> = aircraft.get("weapon_ID")?;
        let weapons = weapons.into_iter()
            .map(|id| load_weapon(lua, id))
            .collect::<LuaResult<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        WeaponData::Aircraft(Aircraft {
            speed,
            weapons: Arc::from(weapons)
        })
    };

    Ok(Some(Weapon {
        weapon_id,
        reload_time: reload_max * RLD_MULT_AT_100,
        fixed_delay: weapon_data.get("recover_time")?,
        data,
    }))
}

fn search_referenced_weapons(barrages: &mut ReferencedWeaponsContext, lua: &Lua, skill: LuaTable) -> LuaResult<()> {
    let effect_list: Option<Vec<LuaTable>> = context!(skill.get("effect_list"); "effect_list of skill/buff")?;
    if let Some(effect_list) = effect_list {
        for entry in effect_list {
            search_referenced_weapons_in_effect_entry(barrages, lua, &entry)?;
        }
    }

    let len = skill.len()?;
    if let Ok(len) = usize::try_from(len) {
        if len != 0 {
            let level_entry: LuaTable = context!(skill.get(len); "level entry {len} of skill/buff")?;
            let effect_list: Option<Vec<LuaTable>> = context!(level_entry.get("effect_list"); "effect_list of skill/buff level entry {len}")?;
            if let Some(effect_list) = effect_list {
                for entry in effect_list {
                    search_referenced_weapons_in_effect_entry(barrages, lua, &entry)?;
                }
            }
        }
    }

    Ok(())
}

fn search_referenced_weapons_in_effect_entry(barrages: &mut ReferencedWeaponsContext, lua: &Lua, entry: &LuaTable) -> LuaResult<()> {
    fn get_arg(entry: &LuaTable, key: &str) -> LuaResult<u32> {
        let arg_list: LuaTable = context!(entry.get("arg_list"); "skill/buff effect_list entry arg_list")?;
        context!(arg_list.get(key); "skill/buff effect_list entry arg_list {}", key)
    }

    let entry_type: String = context!(entry.get("type"); "skill/buff effect_list entry type")?;
    match entry_type.borrow() {
        "BattleBuffCastSkill" => {
            let skill_id = get_arg(entry, "skill_id")?;
            if barrages.seen_skills.insert(skill_id) {
                let skill = require_skill_data(lua, skill_id)?;
                search_referenced_weapons(barrages, lua, skill)?;
            }
        },
        "BattleBuffAddBuff" => {
            let buff_id = get_arg(entry, "buff_id")?;
            if barrages.seen_buffs.insert(buff_id) {
                let buff = require_buff_data(lua, buff_id)?;
                search_referenced_weapons(barrages, lua, buff)?;
            }
        },
        "BattleSkillFire" => {
            let weapon_id = get_arg(entry, "weapon_id")?;
            if barrages.seen_weapons.insert(weapon_id) {
                let target: Option<String> = entry.get("target_choise" /* sic */)?;
                let target = target.as_deref().map(convert_al::to_barrage_target).unwrap_or(SkillAttackTarget::Random);
                if let Some(weapon) = load_weapon(lua, weapon_id)? {
                    barrages.attacks.push(SkillAttack { weapon, target });
                }
            }
        },
        _ => (),
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
    attacks: Vec<SkillAttack>,
    seen_skills: HashSet<u32>,
    seen_buffs: HashSet<u32>,
    seen_weapons: HashSet<u32>,
}