use crate::context;
use crate::model::*;
use crate::convert_al;
use mlua::prelude::*;
use azur_lane::ship::*;

pub fn load_skin(set: &SkinSet) -> LuaResult<ShipSkin> {
    macro_rules! get {
        ($key:literal) => {
            context!(set.template.get($key); "skin template {} for skin {}", $key, set.skin_id)?
        };
    }

    let mut skin = ShipSkin {
        skin_id: set.skin_id,
        name: get!("name"),
        description: get!("desc"),
        words: load_words(set)?,
        words_extra: None // todo
    };

    if let Some(ref extra) = set.words_extra {
        skin.words_extra = Some(Box::new(load_words_extra(set, extra, &skin.words)?));
    }

    Ok(skin)
}

fn load_words(set: &SkinSet) -> LuaResult<ShipSkinWords> {
    macro_rules! get {
        ($key:literal) => {{
            let text: String = context!(set.words.get($key); "skin word {} for skin {}", $key, set.skin_id)?;
            if text.is_empty() { None } else { Some(text) }
        }};
    }

    Ok(ShipSkinWords { 
        description: get!("drop_descrip"),
        introduction: get!("profile"),
        acquisition: get!("unlock"),
        login: get!("login"),
        details: get!("detail"),
        main_screen: to_main_screen_vec(get!("main")),
        touch: get!("touch"),
        special_touch: get!("touch2"),
        rub: get!("headtouch"),
        mission_reminder: get!("mission"),
        mission_complete: get!("mission_complete"),
        mail_reminder: get!("mail"),
        return_to_port: get!("home"),
        commission_complete: get!("expedition"),
        enhance: get!("upgrade"),
        flagship_fight: get!("battle"),
        victory: get!("win_mvp"),
        defeat: get!("lose"),
        skill: get!("skill"),
        low_health: get!("hp_warning"),
        disappointed: get!("feeling1"),
        stranger: get!("feeling2"),
        friendly: get!("feeling3"),
        like: get!("feeling4"),
        love: get!("feeling5"),
        oath: get!("propose"),
        couple_encourage: {
            let tables: Option<Vec<LuaTable>> = context!(set.words.get("couple_encourage"); "skin word couple_encourage").ok();
            tables.into_iter().flatten().map(|t| load_couple_encourage(set, t)).collect::<LuaResult<_>>()?
        }
    })
}

fn load_words_extra(set: &SkinSet, table: &LuaTable, base: &ShipSkinWords) -> LuaResult<ShipSkinWords> {
    macro_rules! get {
        ($key:literal) => {{
            let value: LuaValue = context!(table.get($key); "skin word extra {} for skin {}", $key, set.skin_id)?;
            match value {
                LuaValue::Table(t) => {
                    let t: LuaTable = t.get(1)?;
                    let text: String = t.get(2)?;
                    if text.is_empty() { None } else { Some(text) }
                }
                _ => None
            }
        }};
    }

    let mut main_screen = to_main_screen_vec(get!("main"));

    main_screen.extend(get!("main_extra").iter()
        .flat_map(|s| s.split('|')).enumerate()
        .map(|(index, text)| ShipMainScreenLine::new(index + base.main_screen.len(), text.to_owned())));

    Ok(ShipSkinWords { 
        description: get!("drop_descrip"),
        introduction: get!("profile"),
        acquisition: get!("unlock"),
        login: get!("login"),
        details: get!("detail"),
        main_screen,
        touch: get!("touch"),
        special_touch: get!("touch2"),
        rub: get!("headtouch"),
        mission_reminder: get!("mission"),
        mission_complete: get!("mission_complete"),
        mail_reminder: get!("mail"),
        return_to_port: get!("home"),
        commission_complete: get!("expedition"),
        enhance: get!("upgrade"),
        flagship_fight: get!("battle"),
        victory: get!("win_mvp"),
        defeat: get!("lose"),
        skill: get!("skill"),
        low_health: get!("hp_warning"),
        disappointed: get!("feeling1"),
        stranger: get!("feeling2"),
        friendly: get!("feeling3"),
        like: get!("feeling4"),
        love: get!("feeling5"),
        oath: get!("propose"),
        couple_encourage: Vec::new()
    })
}

fn to_main_screen_vec(raw: Option<String>) -> Vec<ShipMainScreenLine> {
    raw.iter().flat_map(|s| s.split('|')).enumerate()
        .filter(|(_, text)| !text.is_empty() && *text != "nil")
        .map(|(index, text)| ShipMainScreenLine::new(index, text.to_owned()))
        .collect()
}

fn load_couple_encourage(set: &SkinSet, table: LuaTable) -> LuaResult<ShipCoupleEncourage> {
    let filter: Vec<u32> = context!(table.get(1); "couple_encourage 1 for skin {}", set.skin_id)?;
    let mode: Option<u32> = context!(table.get(4); "couple_encourage 4 for skin {}", set.skin_id)?;

    Ok(ShipCoupleEncourage {
        amount: context!(table.get(2); "couple_encourage 2 for skin {}", set.skin_id)?,
        text: context!(table.get(3); "couple_encourage 3 for skin {}", set.skin_id)?,
        condition: match mode {
            Some(1) => ShipCouple::HullType(filter.into_iter().map(convert_al::to_hull_type).collect()),
            Some(2) => ShipCouple::Rarity(filter.into_iter().map(convert_al::to_rarity).collect()),
            Some(3) => ShipCouple::Faction(filter.into_iter().map(convert_al::to_faction).collect()),
            Some(4) => ShipCouple::Illustrator,
            _ => ShipCouple::ShipGroup(filter),
        }
    })
}