use azur_lane::GameServer;
use azur_lane::skin::*;
use mlua::prelude::*;
use small_fixed_array::{FixedArray, FixedString, ValidLength as _};

use crate::intl_util::{IntoFixed as _, IterExt as _, TryIterExt as _};
use crate::model::*;
use crate::{context, convert_al};

pub fn load_skin(set: &SkinSet, server: GameServer) -> LuaResult<ShipSkin> {
    macro_rules! get {
        ($key:literal) => {
            set.template
                .get::<String>($key)
                .with_context(context!("skin template {} for skin {}", $key, set.skin_id))?
                .into_fixed()
        };
    }

    let mut skin = ShipSkin {
        skin_id: set.skin_id,
        image_key: get!("painting"),
        name: get!("name"),
        description: get!("desc"),
        words: vec![load_words(set, server)?].into_fixed(),
        words_extra: FixedArray::new(), // loaded below
    };

    if let Some(extra) = &set.words_extra {
        skin.words_extra = vec![load_words_extra(set, extra, &skin.words[0], server)?].into_fixed();
    }

    Ok(skin)
}

fn load_words(set: &SkinSet, server: GameServer) -> LuaResult<ShipSkinWords> {
    macro_rules! get {
        ($src:literal) => {{
            let text: String = set.words.get($src).with_context(context!(
                "skin word {} for skin {}",
                $src,
                set.skin_id
            ))?;
            if text.is_empty() {
                None
            } else {
                Some(FixedString::<u32>::from_string_trunc(text))
            }
        }};
        ($key:ident, $src:literal) => {
            get!($src).map(|x| (ShipSkinWordKey::$key, x))
        };
    }

    let sparse = [
        get!(description, "drop_descrip"),
        get!(introduction, "profile"),
        get!(acquisition, "unlock"),
        get!(login, "login"),
        get!(details, "detail"),
        get!(touch, "touch"),
        get!(special_touch, "touch2"),
        get!(rub, "headtouch"),
        get!(mission_reminder, "mission"),
        get!(mission_complete, "mission_complete"),
        get!(mail_reminder, "mail"),
        get!(return_to_port, "home"),
        get!(commission_complete, "expedition"),
        get!(enhance, "upgrade"),
        get!(flagship_fight, "battle"),
        get!(victory, "win_mvp"),
        get!(defeat, "lose"),
        get!(skill, "skill"),
        get!(low_health, "hp_warning"),
        get!(disappointed, "feeling1"),
        get!(stranger, "feeling2"),
        get!(friendly, "feeling3"),
        get!(crush, "feeling4"),
        get!(love, "feeling5"),
        get!(oath, "propose"),
        get!(gift_prefer, "gift_prefer"),
        get!(gift_dislike, "gift_dislike"),
    ];

    Ok(ShipSkinWords {
        server,
        main_screen: to_main_screen(get!("main").as_deref()).collect_fixed_array(),
        couple_encourage: set
            .words
            .get::<Vec<LuaTable>>("couple_encourage")
            .context("skin word couple_encourage")
            .into_iter()
            .flatten()
            .map(|t| load_couple_encourage(set, t))
            .try_collect_fixed_array()?,
        sparse: SparseShipSkinWords::new(sparse.into_iter().flatten().collect_fixed_array()),
    })
}

fn load_words_extra(
    set: &SkinSet,
    table: &LuaTable,
    base: &ShipSkinWords,
    server: GameServer,
) -> LuaResult<ShipSkinWords> {
    macro_rules! get {
        ($src:literal) => {{
            let value: LuaValue = table.get($src).with_context(context!(
                "skin word extra {} for skin {}",
                $src,
                set.skin_id
            ))?;

            match value {
                LuaValue::Table(t) => {
                    let t: LuaTable = t.get(1)?;
                    let text: String = t.get(2)?;
                    (!text.is_empty()).then(|| FixedString::<u32>::from_string_trunc(text))
                },
                _ => None,
            }
        }};
        ($key:ident, $src:literal) => {
            get!($src).map(|x| (ShipSkinWordKey::$key, x))
        };
    }

    let main_screen = to_main_screen(get!("main").as_deref())
        .chain(to_main_screen(get!("main_extra").as_deref()).map(|line| {
            let index = line.index();
            line.with_index(index + base.main_screen.len().to_usize())
        }))
        .collect_fixed_array();

    let sparse = [
        get!(description, "drop_descrip"),
        get!(introduction, "profile"),
        get!(acquisition, "unlock"),
        get!(login, "login"),
        get!(details, "detail"),
        get!(touch, "touch"),
        get!(special_touch, "touch2"),
        get!(rub, "headtouch"),
        get!(mission_reminder, "mission"),
        get!(mission_complete, "mission_complete"),
        get!(mail_reminder, "mail"),
        get!(return_to_port, "home"),
        get!(commission_complete, "expedition"),
        get!(enhance, "upgrade"),
        get!(flagship_fight, "battle"),
        get!(victory, "win_mvp"),
        get!(defeat, "lose"),
        get!(skill, "skill"),
        get!(low_health, "hp_warning"),
        get!(disappointed, "feeling1"),
        get!(stranger, "feeling2"),
        get!(friendly, "feeling3"),
        get!(crush, "feeling4"),
        get!(love, "feeling5"),
        get!(oath, "propose"),
        get!(gift_prefer, "gift_prefer"),
        get!(gift_dislike, "gift_dislike"),
    ];

    Ok(ShipSkinWords {
        server,
        main_screen,
        couple_encourage: FixedArray::empty(),
        sparse: SparseShipSkinWords::new(sparse.into_iter().flatten().collect_fixed_array()),
    })
}

pub fn to_main_screen(raw: Option<&str>) -> impl Iterator<Item = ShipMainScreenLine> + '_ {
    raw.into_iter()
        .flat_map(|s| s.split('|'))
        .enumerate()
        .filter(|(_, text)| !text.is_empty() && *text != "nil")
        .map(|(index, text)| ShipMainScreenLine::new(index, FixedString::from_str_trunc(text)))
}

fn load_couple_encourage(set: &SkinSet, table: LuaTable) -> LuaResult<ShipCoupleEncourage> {
    let filter: Vec<u32> = table
        .get(1)
        .with_context(context!("couple_encourage 1 for skin {}", set.skin_id))?;
    let mode: Option<u32> = table
        .get(4)
        .with_context(context!("couple_encourage 4 for skin {}", set.skin_id))?;

    fn map<T>(filter: Vec<u32>, map: impl FnMut(u32) -> T) -> FixedArray<T> {
        filter.into_iter().map(map).collect_fixed_array()
    }

    Ok(ShipCoupleEncourage {
        amount: table
            .get(2)
            .with_context(context!("couple_encourage 2 for skin {}", set.skin_id))?,
        line: table
            .get::<String>(3)
            .with_context(context!("couple_encourage 3 for skin {}", set.skin_id))?
            .into_fixed(),
        condition: match mode {
            // note:
            // - Warspite, Admiral Hipper, Zeppy, and Peter Strasser define lines without a filter
            //   type, clearly intended to be ShipGroup. these lines do not work, but we include
            //   them as intended anyways
            // - Hatsuharu and Richelieu have lines defined with the wrong filter type
            None | Some(0) => ShipCouple::ShipGroup(filter.into_fixed()),
            Some(1) => ShipCouple::HullType(map(filter, convert_al::to_hull_type)),
            Some(2) => ShipCouple::Rarity(map(filter, convert_al::to_rarity)),
            Some(3) => ShipCouple::Faction(map(filter, convert_al::to_faction)),
            Some(4) => ShipCouple::Illustrator,
            Some(5) => ShipCouple::Team,
            _ => ShipCouple::Unknown,
        },
    })
}
