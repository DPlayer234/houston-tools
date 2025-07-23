use azur_lane::GameServer;
use azur_lane::secretary::*;
use mlua::prelude::*;
use small_fixed_array::{FixedString, TruncatingInto as _};

use super::skin::to_main_screen;
use crate::intl_util::IterExt as _;
use crate::{CONFIG, context};

pub fn load_special_secretary(
    lua: &Lua,
    data: &LuaTable,
    server: GameServer,
) -> LuaResult<SpecialSecretary> {
    let id: u32 = data.get("id")?;

    let kind_name = if data.get::<u32>("unlock_type")? == 4 {
        // type 4 is a "skin" unlock, so we grab the kind name from the corresponding
        // skin data. i don't expect there to be too many but may as well do it properly
        let [unlock] = data.get::<[u32; 1]>("unlock")?;
        lua.globals()
            .get::<LuaTable>("pg")
            .context("global pg")?
            .get::<LuaTable>("ship_skin_template")
            .context("global pg.ship_skin_template")?
            .get::<LuaTable>(unlock)
            .with_context(context!("skin with id {unlock}"))?
            .get::<String>("name")
            .with_context(context!("name for ship_skin_template {unlock}"))?
    } else {
        // otherwise, take them from the configuration list. i didn't figure out how the
        // game gets them -- if it does at all. there isn't really a need for the game
        // to have some way to map the type to a string name after all
        CONFIG
            .special_secretary_kinds
            .get(data.get::<usize>("type")?)
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_owned())
    };

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
                Some(FixedString::<u32>::from_string_trunc(text))
            }
        }};
    }

    let words = SpecialSecretaryWords {
        server,
        login: get!("login"),
        main_screen: to_main_screen(get!("main").as_deref()).collect_fixed_array(),
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
    };

    Ok(SpecialSecretary {
        id,
        name: data.get::<String>("name")?.trunc_into(),
        kind: kind_name.trunc_into(),
        words: vec![words].trunc_into(),
    })
}

fn get_chimes(id: u32, data: &LuaTable) -> Option<Box<[FixedString; 24]>> {
    macro_rules! get {
        ($key:literal) => {{
            data.get::<String>($key)
                .with_context(context!("secretary word {} for secretary {}", $key, id))
                .ok()
                .filter(|s| !s.is_empty())?
                .trunc_into()
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
