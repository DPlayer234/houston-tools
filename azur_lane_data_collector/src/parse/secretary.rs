use azur_lane::secretary::*;
use mlua::prelude::*;

use super::skin::to_main_screen;
use crate::{context, CONFIG};

pub fn load_special_secretary(_lua: &Lua, data: &LuaTable) -> LuaResult<SpecialSecretary> {
    let id: u32 = data.get("id")?;

    macro_rules! get {
        ($key:literal) => {{
            let text: String = data.get($key).with_context(context!(
                "secretary word {} for secretary {}",
                $key,
                id
            ))?;
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }};
    }

    Ok(SpecialSecretary {
        id,
        name: data.get("name")?,
        kind: CONFIG
            .special_secretary_kinds
            .get(data.get::<usize>("type")?)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_owned()),
        main_screen: to_main_screen(get!("main").as_deref()).collect(),
        touch: get!("touch"),
        mission_reminder: get!("mission"),
        mission_complete: get!("mission_complete"),
        mail_reminder: get!("mail"),
        return_to_port: get!("home"),
        commission_complete: get!("expedition"),
        christmas: get!("shengdan"),
        new_years_eve: get!("chuxi"),
        new_years_day: get!("xinnian"),
        valentines: get!("qingrenjie"),
        mid_autumn_festival: get!("zhongqiu"),
        halloween: get!("wansheng"),
        event_reminder: get!("huodong"),
        change_module: get!("genghuan"),
        chime: get_chimes(id, data),
    })
}

fn get_chimes(id: u32, data: &LuaTable) -> Option<Box<[String; 24]>> {
    macro_rules! get {
        ($key:literal) => {{
            data.get::<String>($key)
                .with_context(context!("secretary word {} for secretary {}", $key, id))
                .ok()
                .filter(|s| !s.is_empty())?
        }};
    }

    Some(Box::new([
        get!("chime_0"),
        get!("chime_1"),
        get!("chime_2"),
        get!("chime_3"),
        get!("chime_4"),
        get!("chime_5"),
        get!("chime_6"),
        get!("chime_7"),
        get!("chime_8"),
        get!("chime_9"),
        get!("chime_10"),
        get!("chime_11"),
        get!("chime_12"),
        get!("chime_13"),
        get!("chime_14"),
        get!("chime_15"),
        get!("chime_16"),
        get!("chime_17"),
        get!("chime_18"),
        get!("chime_19"),
        get!("chime_20"),
        get!("chime_21"),
        get!("chime_22"),
        get!("chime_23"),
    ]))
}
