use std::collections::HashMap;
use std::io::Write as _;
use std::mem::take;
use std::path::{Component, Path};
use std::{fs, io};

use azur_lane::equip::*;
use azur_lane::secretary::*;
use azur_lane::ship::*;
use azur_lane::skin::*;
use azur_lane::{DefinitionData, GameServer, juustagram};
use clap::Parser;
use mlua::prelude::*;
use small_fixed_array::{FixedArray, FixedString, ValidLength as _};

mod convert_al;
mod enhance;
mod intl_util;
mod log;
mod macros;
mod model;
mod parse;

use crate::intl_util::{FixedArrayExt as _, IntoFixed as _, IterExt as _, TryIterExt as _};
use crate::model::*;

#[derive(Debug, Parser)]
struct Cli {
    /// The path that the game scripts live in.
    ///
    /// This is the directory that contains, among others, `config.lua`.
    ///
    /// If you get an error, that it couldn't find a Lua file, you chose the
    /// wrong directory.
    #[arg(short, long, num_args = 1.., required = true)]
    inputs: Vec<String>,

    /// The output directory.
    ///
    /// The directory is created if it's missing.
    #[arg(short, long)]
    out: Option<String>,

    /// The path that holds the game assets.
    ///
    /// This essentially points to the game's `AssetBundles` directory.
    /// Currently, only chibis (`shipmodels`) are loaded.
    ///
    /// If not specified, no resources will be loaded.
    #[arg(long)]
    assets: Option<String>,

    /// Minimize the output JSON file.
    #[arg(short, long)]
    minimize: bool,

    /// Override whether this program outputs color.
    ///
    /// Auto-detection is performed, but in case it is wrong, you may use this
    /// to override the default.
    #[arg(long)]
    color: Option<bool>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    log::use_color(cli.color);

    match option_env!("GIT_HASH") {
        Some(git_hash) => log::info!("Azur Lane Data Collector [Commit: {git_hash}]"),
        None => log::info!("Azur Lane Data Collector [Unknown Commit]"),
    };

    let out_data = {
        anyhow::ensure!(
            cli.inputs
                .iter()
                .filter_map(|s| guess_server(s))
                .is_unique(),
            "multiple `--inputs` end with the same server shorthand, but server shorthands must be unique"
        );

        let merge_inputs = anyhow::Context::context(
            cli.inputs
                .iter()
                .skip(1)
                .map(|p| guess_server(p).map(|s| (s, p.as_str())))
                .collect::<Option<Vec<_>>>(),
            "in the case of multiple `--inputs`, paths must end with a server shorthand, i.e. `EN` or `JP`",
        )?;

        // Expect at least 1 input
        let input = &cli.inputs[0];
        let server = guess_server(input).unwrap_or_default();
        let mut out_data = load_definition(&cli.inputs[0], server)?;

        for (server, input) in merge_inputs {
            let next = load_definition(input, server)?;
            merge_out_data(&mut out_data, next);
        }

        out_data
    };

    let out_dir = cli.out.as_deref().unwrap_or("azur_lane_data");
    {
        let action = log::action!("Writing `main.json`.")
            .unbounded()
            .suffix(" KB")
            .start();

        fs::create_dir_all(out_dir)?;
        let file = fs::File::create(Path::new(out_dir).join("main.json"))?;
        let file = io::BufWriter::new(file);
        let mut action = log::ActionWrite::new(action, file);
        if cli.minimize {
            serde_json::to_writer(&mut action, &out_data)?;
        } else {
            serde_json::to_writer_pretty(&mut action, &out_data)?;
        }

        action.finish();
    }

    if let Some(assets) = cli.assets.as_deref() {
        // Extract and save chibis for all skins.
        fs::create_dir_all(Path::new(out_dir).join("chibi"))?;

        let total_count = out_data
            .ships
            .iter()
            .map(|s| s.skins.len().to_usize())
            .sum();
        let mut action = log::action!("Extracting chibis.")
            .bounded_total(total_count)
            .start();

        let mut extract_count = 0usize;
        let mut new_count = 0usize;

        for skin in out_data.ships.iter().flat_map(|s| s.skins.iter()) {
            if let Some(image) = parse::image::load_chibi_image(&action, assets, &skin.image_key)? {
                extract_count += 1;

                let path = utils::join_path!(out_dir, "chibi", skin.image_key.as_str(); "webp");
                if let Ok(mut f) = fs::File::create_new(path) {
                    new_count += 1;

                    f.write_all(&image)?;
                }
            }

            action.update_amount(extract_count);
        }

        action.finish();
        log::info!("{new_count} new chibi(s).");
    }

    Ok(())
}

fn load_definition(input: &str, server: GameServer) -> anyhow::Result<DefinitionData> {
    let lua = init_lua(input)?;
    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;

    let ships = load_ships(&lua, &pg, server)?;
    let equips = load_equips(&lua, &pg)?;
    let augments = load_augments(&lua, &pg)?;
    let juustagram_chats = load_juustagram_chats(&lua, &pg)?;
    let special_secretaries = load_special_secretaries(&lua, &pg, server)?;

    Ok(DefinitionData {
        ships,
        equips,
        augments,
        juustagram_chats,
        special_secretaries,
    })
}

fn init_lua(input: &str) -> anyhow::Result<Lua> {
    let action = log::action!("Initializing Lua for: `{input}`").start();

    let lua = Lua::new();

    lua.globals().raw_set("AZUR_LANE_DATA_PATH", input)?;
    lua.load(include_str!("../assets/lua_init.lua"))
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .exec()?;

    action.finish();
    Ok(lua)
}

fn load_ships(lua: &Lua, pg: &LuaTable, server: GameServer) -> anyhow::Result<FixedArray<Ship>> {
    let ship_data_template: LuaTable = pg
        .get("ship_data_template")
        .context("global pg.ship_data_template")?;
    let ship_data_template_all: LuaTable = ship_data_template
        .get("all")
        .context("global pg.ship_data_template.all")?;
    let ship_data_statistics: LuaTable = pg
        .get("ship_data_statistics")
        .context("global pg.ship_data_statistics")?;

    // Normal enhancement data (may be present even if not used for that ship):
    let ship_data_strengthen: LuaTable = pg
        .get("ship_data_strengthen")
        .context("global pg.ship_data_strengthen")?;

    // Blueprint/Research ship data:
    let ship_data_blueprint: LuaTable = pg
        .get("ship_data_blueprint")
        .context("global pg.ship_data_blueprint")?;
    let ship_strengthen_blueprint: LuaTable = pg
        .get("ship_strengthen_blueprint")
        .context("global pg.ship_strengthen_blueprint")?;

    // META ship data:
    let ship_strengthen_meta: LuaTable = pg
        .get("ship_strengthen_meta")
        .context("global pg.ship_strengthen_meta")?;
    let ship_meta_repair: LuaTable = pg
        .get("ship_meta_repair")
        .context("global pg.ship_meta_repair")?;
    let ship_meta_repair_effect: LuaTable = pg
        .get("ship_meta_repair_effect")
        .context("global pg.ship_meta_repair_effect")?;

    // Retrofit data:
    let ship_data_trans: LuaTable = pg
        .get("ship_data_trans")
        .context("global pg.ship_data_trans")?;
    let transform_data_template: LuaTable = pg
        .get("transform_data_template")
        .context("global pg.transform_data_template")?;

    // Fleet tech:
    let fleet_tech_ship_template: LuaTable = pg
        .get("fleet_tech_ship_template")
        .context("global pg.fleet_tech_ship_template")?;

    // Skin/word data:
    let ship_skin_template: LuaTable = pg
        .get("ship_skin_template")
        .context("global pg.ship_skin_template")?;
    let ship_skin_template_get_id_list_by_ship_group: LuaTable = ship_skin_template
        .get("get_id_list_by_ship_group")
        .context("global pg.ship_skin_template.get_id_list_by_ship_group")?;
    let ship_skin_words: LuaTable = pg
        .get("ship_skin_words")
        .context("global pg.ship_skin_words")?;
    let ship_skin_words_extra: LuaTable = pg
        .get("ship_skin_words_extra")
        .context("global pg.ship_skin_words_extra")?;

    let mut action = log::action!("Finding ship groups.")
        .unbounded()
        .suffix("..")
        .start();

    let mut groups = HashMap::new();
    ship_data_template_all.for_each(|_: u32, id: u32| {
        if (900000..=900999).contains(&id) {
            return Ok(());
        }

        let template: LuaTable = ship_data_template
            .get(id)
            .with_context(context!("ship_data_template with id {id}"))?;
        let group_id: u32 = template
            .get("group_type")
            .with_context(context!("group_type of ship_data_template with id {id}"))?;

        groups
            .entry(group_id)
            .or_insert_with(|| {
                action.inc_amount();
                ShipGroup {
                    id: group_id,
                    members: Vec::new(),
                }
            })
            .members
            .push(id);

        Ok(())
    })?;

    let total = action.amount();
    action.finish();

    let make_ship_set = |id: u32| -> LuaResult<ShipSet<'_>> {
        let template: LuaTable = ship_data_template
            .get(id)
            .with_context(context!("!ship_data_template with id {id}"))?;
        let statistics: LuaTable = ship_data_statistics
            .get(id)
            .with_context(context!("ship_data_statistics with id {id}"))?;

        let strengthen_id: u32 = template
            .get("strengthen_id")
            .with_context(context!("strengthen_id of ship_data_template with id {id}"))?;
        let _: u32 = template
            .get("id")
            .with_context(context!("id of ship_data_template with id {id}"))?;

        let enhance: Option<LuaTable> = ship_data_strengthen
            .get(strengthen_id)
            .with_context(context!("ship_data_strengthen with {id}"))?;
        let blueprint: Option<LuaTable> = ship_data_blueprint
            .get(strengthen_id)
            .with_context(context!("ship_data_blueprint with {id}"))?;
        let meta: Option<LuaTable> = ship_strengthen_meta
            .get(strengthen_id)
            .with_context(context!("ship_strengthen_meta with {id}"))?;

        let strengthen = match (enhance, blueprint, meta) {
            (_, Some(data), _) => Strengthen::Blueprint(BlueprintStrengthen {
                data,
                effect_lookup: &ship_strengthen_blueprint,
            }),
            (_, _, Some(data)) => Strengthen::Meta(MetaStrengthen {
                data,
                repair_lookup: &ship_meta_repair,
                repair_effect_lookup: &ship_meta_repair_effect,
            }),
            (Some(data), _, _) => Strengthen::Normal(data),
            _ => Err(LuaError::external(DataError::NoStrengthen))?,
        };

        let retrofit_data: Option<LuaTable> = ship_data_trans
            .get(strengthen_id)
            .with_context(context!("ship_data_trans with {id}"))?;
        let retrofit_data = retrofit_data.map(|r| RetrofitSet {
            data: r,
            list_lookup: &transform_data_template,
        });

        Ok(ShipSet {
            id,
            template,
            statistics,
            strengthen,
            retrofit_data,
        })
    };

    let make_fleet_tech = |id: u32| -> LuaResult<Option<FleetTechInfo>> {
        let fleet_tech: Option<LuaTable> = fleet_tech_ship_template
            .get(id)
            .with_context(context!("fleet_tech_ship_template with {id}"))?;

        match fleet_tech {
            Some(f) => parse::fleet_tech::load_ship_tech(lua, id, f).map(Some),
            None => Ok(None),
        }
    };

    let mut action = log::action!("Building ship groups.")
        .bounded_total(total)
        .start();

    let config = &*CONFIG;
    let make_ship_from_group = |group: ShipGroup| {
        let members: Vec<_> = group.members.into_iter().map(make_ship_set).try_collect()?;

        let mlb_max_id = group.id * 10 + 4;
        let raw_mlb = members
            .iter()
            .filter(|t| t.id <= mlb_max_id)
            .max_by_key(|t| t.id)
            .ok_or_else(|| {
                LuaError::external(DataError::NoMlb)
                    .context(format!("no mlb for ship with id {}", group.id))
            })?;

        let raw_retrofits = members.iter().filter(|t| t.id > raw_mlb.id);

        let make_skin = |skin_id| -> LuaResult<SkinSet> {
            Ok(SkinSet {
                skin_id,
                template: ship_skin_template.get(skin_id).with_context(context!(
                    "skin template {} for ship {}",
                    skin_id,
                    group.id
                ))?,
                words: ship_skin_words.get(skin_id).with_context(context!(
                    "skin words {} for ship {}",
                    skin_id,
                    group.id
                ))?,
                words_extra: ship_skin_words_extra.get(skin_id).with_context(context!(
                    "skin words extra {} for ship {}",
                    skin_id,
                    group.id
                ))?,
            })
        };

        let raw_skins: Vec<_> = ship_skin_template_get_id_list_by_ship_group
            .get::<Vec<u32>>(group.id)
            .with_context(context!("skin ids for ship with id {}", group.id))?
            .into_iter()
            .map(make_skin)
            .try_collect()?;

        let mut base = parse::ship::load_ship_data(lua, raw_mlb)?;
        if let Some(name_override) = config.name_overrides.get(&base.group_id) {
            base.name = FixedString::from_str_trunc(name_override);
        }

        let mut retrofits = FixedArray::new();
        if let Some(retrofit_data) = &raw_mlb.retrofit_data {
            retrofits = raw_retrofits
                .map(|retrofit_set| {
                    let base = parse::ship::load_ship_data(lua, retrofit_set)?;
                    let mut retrofit = Retrofit { base };
                    enhance::retrofit::apply_retrofit(lua, &mut retrofit, retrofit_data)?;
                    fix_up_retrofitted_data(&mut retrofit, retrofit_set)?;
                    Ok::<_, LuaError>(retrofit)
                })
                .try_collect_fixed_array()?;

            if retrofits.is_empty() {
                let mut retrofit = Retrofit { base: base.clone() };
                enhance::retrofit::apply_retrofit(lua, &mut retrofit, retrofit_data)?;
                fix_up_retrofitted_data(&mut retrofit, raw_mlb)?;
                retrofits.push(retrofit);
            }
        }

        let skins = raw_skins
            .into_iter()
            .map(|s| parse::skin::load_skin(&s, server))
            .try_collect_fixed_array()?;

        let fleet_tech = make_fleet_tech(group.id)?;

        action.inc_amount();
        Ok::<_, LuaError>(Ship {
            base,
            retrofits,
            skins,
            fleet_tech,
        })
    };

    let mut ships = groups
        .into_values()
        .map(make_ship_from_group)
        .try_collect_fixed_array()?;

    action.finish();

    ships.sort_unstable_by_key(|t| t.base.group_id);
    Ok(ships)
}

fn load_equips(lua: &Lua, pg: &LuaTable) -> anyhow::Result<FixedArray<Equip>> {
    let equip_data_template: LuaTable = pg
        .get("equip_data_template")
        .context("global pg.equip_data_template")?;
    let equip_data_template_all: LuaTable = equip_data_template
        .get("all")
        .context("global pg.equip_data_template.all")?;
    let equip_data_statistics: LuaTable = pg
        .get("equip_data_statistics")
        .context("global pg.equip_data_statistics")?;

    let mut action = log::action!("Finding equips.")
        .unbounded()
        .suffix("..")
        .start();

    let mut equips = Vec::new();
    equip_data_template_all.for_each(|_: u32, id: u32| {
        let template: LuaTable = equip_data_template
            .get(id)
            .with_context(context!("equip_data_template with id {id}"))?;
        let statistics: LuaTable = equip_data_statistics
            .get(id)
            .with_context(context!("equip_data_statistics with id {id}"))?;

        let next: u32 = template
            .get("next")
            .with_context(context!("base of equip_data_template with id {id}"))?;
        let tech: u32 = statistics
            .get("tech")
            .with_context(context!("tech of equip_data_statistics with id {id}"))?;
        if next == 0 && matches!(tech, 0 | 3..) {
            action.inc_amount();
            equips.push(id);
        }

        Ok(())
    })?;

    let total = action.amount();
    action.finish();

    let mut action = log::action!("Building equips.")
        .bounded_total(total)
        .start();

    let make_equip = |id| {
        let equip = parse::skill::load_equip(lua, id)?;
        action.inc_amount();
        Ok::<_, LuaError>(equip)
    };

    let mut equips = equips
        .into_iter()
        .map(make_equip)
        .try_collect_fixed_array()?;

    action.finish();

    equips.sort_unstable_by_key(|t| (t.faction, t.kind, t.equip_id));
    Ok(equips)
}

fn load_augments(lua: &Lua, pg: &LuaTable) -> anyhow::Result<FixedArray<Augment>> {
    let spweapon_data_statistics: LuaTable = pg
        .get("spweapon_data_statistics")
        .context("global pg.spweapon_data_statistics")?;
    let spweapon_data_statistics_all: LuaTable = spweapon_data_statistics
        .get("all")
        .context("global pg.spweapon_data_statistics.all")?;

    let mut action = log::action!("Finding augments.")
        .unbounded()
        .suffix("..")
        .start();

    let mut groups: HashMap<u32, u32> = HashMap::new();
    spweapon_data_statistics_all.for_each(|_: u32, id: u32| {
        let statistics: LuaTable = spweapon_data_statistics
            .get(id)
            .with_context(context!("spweapon_data_statistics with id {id}"))?;

        let base_id: Option<u32> = statistics
            .get("base")
            .with_context(context!("base of spweapon_data_statistics with id {id}"))?;
        let base_id = base_id.unwrap_or(id);

        groups
            .entry(base_id)
            .and_modify(|e| {
                if *e < id {
                    *e = id
                }
            })
            .or_insert_with(|| {
                action.inc_amount();
                id
            });

        Ok(())
    })?;

    let total = action.amount();
    action.finish();

    let mut action = log::action!("Building augments.")
        .bounded_total(total)
        .start();

    let make_augment = |id| {
        let statistics: LuaTable = spweapon_data_statistics
            .get(id)
            .with_context(context!("spweapon_data_statistics with id {id}"))?;
        let data = AugmentSet { id, statistics };
        let augment = parse::augment::load_augment(lua, &data)?;
        action.inc_amount();
        Ok::<_, LuaError>(augment)
    };

    let mut augments = groups
        .into_values()
        .map(make_augment)
        .try_collect_fixed_array()?;

    action.finish();

    augments.sort_unstable_by_key(|t| t.augment_id);
    Ok(augments)
}

fn load_juustagram_chats(lua: &Lua, pg: &LuaTable) -> anyhow::Result<FixedArray<juustagram::Chat>> {
    let activity_ins_chat_group: LuaTable = pg
        .get("activity_ins_chat_group")
        .context("global pg.activity_ins_chat_group")?;
    let activity_ins_chat_group_all: LuaTable = activity_ins_chat_group
        .get("all")
        .context("global pg.activity_ins_chat_group.all")?;

    let mut chats = Vec::new();

    let total = activity_ins_chat_group_all.len()?;
    let mut action = log::action!("Building Juustagram chats.")
        .bounded_total(total.try_into()?)
        .start();

    activity_ins_chat_group_all.for_each(|_: u32, id: u32| {
        let chat: LuaValue = lua
            .globals()
            .call_function("get_juustagram_chat", id)
            .with_context(context!("activity_ins_chat_group with id {id}"))?;
        chats.push(lua.from_value(chat)?);
        action.inc_amount();
        Ok(())
    })?;

    action.finish();
    Ok(chats.into_fixed())
}

fn load_special_secretaries(
    lua: &Lua,
    pg: &LuaTable,
    server: GameServer,
) -> anyhow::Result<FixedArray<SpecialSecretary>> {
    let secretary_special_ship: LuaTable = pg
        .get("secretary_special_ship")
        .context("global pg.secretary_special_ship")?;
    let secretary_special_ship_all: LuaTable = secretary_special_ship
        .get("all")
        .context("global pg.secretary_special_ship.all")?;

    let mut action = log::action!("Finding special secretaries.")
        .unbounded()
        .suffix("..")
        .start();

    let mut ships = Vec::new();
    secretary_special_ship_all.for_each(|_: u32, id: u32| {
        let template: LuaTable = secretary_special_ship
            .get(id)
            .with_context(context!("secretary_special_ship with id {id}"))?;

        let kind: u32 = template
            .get("type")
            .with_context(context!("type of secretary_special_ship with id {id}"))?;

        if kind != 0 {
            action.inc_amount();
            ships.push(template);
        }

        Ok(())
    })?;

    let total = action.amount();
    action.finish();

    let mut action = log::action!("Building special secretaries.")
        .bounded_total(total)
        .start();

    let make_secretary = |data| {
        let equip = parse::secretary::load_special_secretary(lua, &data, server)?;
        action.inc_amount();
        Ok::<_, LuaError>(equip)
    };

    let mut ships = ships
        .into_iter()
        .map(make_secretary)
        .try_collect_fixed_array()?;

    action.finish();

    ships.sort_unstable_by_key(|t| t.id);
    Ok(ships)
}

fn fix_up_retrofitted_data(ship: &mut Retrofit, set: &ShipSet<'_>) -> LuaResult<()> {
    let buff_list_display: Vec<u32> = set.template.get("buff_list_display")?;
    ship.base.skills.sort_by_key(|s| {
        buff_list_display
            .iter()
            .enumerate()
            .find(|i| *i.1 == s.buff_id)
            .map(|i| i.0)
            .unwrap_or_default()
    });

    Ok(())
}

fn merge_out_data(main: &mut DefinitionData, next: DefinitionData) {
    macro_rules! eq {
        ($fn:ident, $Ty:ty, $($key:ident).*) => {
            fn $fn(a: &$Ty, b: &$Ty) -> bool {
                a.$($key).* == b.$($key).*
            }
        };
    }

    eq!(ship_eq, Ship, base.group_id);
    eq!(retrofit_eq, Retrofit, base.default_skin_id);
    eq!(skin_eq, Skin, skin_id);
    eq!(augment_eq, Augment, augment_id);
    eq!(equip_eq, Equip, equip_id);
    eq!(chat_eq, juustagram::Chat, chat_id);
    eq!(special_secretary_eq, SpecialSecretary, id);

    let action = log::action!("Merging data.").start();

    let update_skin = move |r: &mut Skin, n: Skin| {
        r.words.extend_from_array(n.words);
        r.words_extra.extend_from_array(n.words_extra);
    };

    let update_ship = move |r: &mut Ship, n: Ship| {
        add_missing(&mut r.retrofits, n.retrofits, retrofit_eq);
        add_or_update(&mut r.skins, n.skins, skin_eq, update_skin);
    };

    let update_secretary = move |r: &mut SpecialSecretary, n: SpecialSecretary| {
        r.words.extend_from_array(n.words);
    };

    add_or_update(&mut main.ships, next.ships, ship_eq, update_ship);
    add_missing(&mut main.augments, next.augments, augment_eq);
    add_missing(&mut main.equips, next.equips, equip_eq);
    add_missing(&mut main.juustagram_chats, next.juustagram_chats, chat_eq);
    add_or_update(
        &mut main.special_secretaries,
        next.special_secretaries,
        special_secretary_eq,
        update_secretary,
    );

    action.finish();
}

fn add_missing<T>(main: &mut FixedArray<T>, next: FixedArray<T>, matches: impl Fn(&T, &T) -> bool) {
    add_or_update(main, next, matches, |_, _| {});
}

fn add_or_update<T>(
    main: &mut FixedArray<T>,
    next: FixedArray<T>,
    matches: impl Fn(&T, &T) -> bool,
    on_match: impl Fn(&mut T, T),
) {
    let mut m = take(main).into_vec();
    for new in next {
        if let Some(old) = m.iter_mut().find(|old| matches(old, &new)) {
            on_match(old, new);
        } else {
            m.push(new);
        }
    }

    *main = m.into_fixed();
}

fn guess_server(path: &str) -> Option<GameServer> {
    let path = Path::new(path);
    let dir = path.components().next_back()?;

    let Component::Normal(dir) = dir else {
        return None;
    };

    dir.to_str()?.parse().ok()
}
