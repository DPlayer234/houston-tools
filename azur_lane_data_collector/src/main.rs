use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use clap::Parser;
use mlua::prelude::*;

use azur_lane::*;
use azur_lane::ship::*;

mod convert_al;
mod enhance;
mod macros;
mod model;
mod parse;

use model::*;

#[derive(Debug, Parser)]
struct Cli {
    /// The path that the game scripts live in.
    #[arg(short, long, num_args = 1.., required = true)]
    inputs: Vec<String>,
    /// The output directory.
    #[arg(short, long)]
    out: Option<String>,

    /// The path that holds the game assets.
    #[arg(long)]
    assets: Option<String>,

    /// Minimize the output JSON file.
    #[arg(short, long)]
    minimize: bool,

    #[arg(long)]
    test: bool
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let start = std::time::Instant::now();

    let out_data = {
        // Expect at least 1 input
        let mut out_data = load_definition(&cli.inputs[0], start)?;
        for input in cli.inputs.iter().skip(1) {
            println!("Loading more from '{}'...", input);
            let next = load_definition(input, start)?;
            merge_out_data(&mut out_data, next);
            println!("Merged data. ({:.2?})", start.elapsed());
        }

        out_data
    };

    let out_dir = cli.out.as_deref().unwrap_or("azur_lane_data");
    {
        println!("Writing output...");

        fs::create_dir_all(out_dir)?;
        let f = fs::File::create(Path::new(out_dir).join("main.json"))?;
        if cli.minimize {
            serde_json::to_writer(&f, &out_data)?;
        } else {
            serde_json::to_writer_pretty(&f, &out_data)?;
        }

        println!("Written {} bytes. ({:.2?})", f.metadata()?.len(), start.elapsed());
    }

    if let Some(assets) = cli.assets.as_deref() {
        // Extract and save chibis for all skins.
        fs::create_dir_all(Path::new(out_dir).join("chibi"))?;

        println!("Extracting chibis...");

        let mut extract_count = 0usize;
        let mut total_count = 0usize;
        let mut new_count = 0usize;

        for skin in out_data.ships.iter().flat_map(|s| s.skins.iter()) {
            total_count += 1;

            if let Some(image) = parse::image::load_chibi_image(assets, &skin.image_key)? {
                extract_count += 1;

                let path = utils::join_path![out_dir, "chibi", &skin.image_key; "webp"];
                if let Ok(mut f) = fs::OpenOptions::new().create_new(true).write(true).open(path) {
                    new_count += 1;

                    f.write_all(&image)?;
                }
            }
        }

        println!("Extracted chibis ({extract_count}/{total_count}); {new_count} new. {:.2?}", start.elapsed());
    }

    Ok(())
}

fn load_definition(input: &str, start: std::time::Instant) -> Result<DefinitionData, anyhow::Error> {
    let lua = Lua::new();

    lua.globals().raw_set("AZUR_LANE_DATA_PATH", input)?;
    lua.load(include_str!("../assets/lua_init.lua"))
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .exec()?;

    println!("Init done. ({:.2?})", start.elapsed());

    let pg: LuaTable = lua.globals().get("pg").context("global pg")?;

    let ships = {
        let ship_data_template: LuaTable = pg.get("ship_data_template").context("global pg.ship_data_template")?;
        let ship_data_template_all: LuaTable = ship_data_template.get("all").context("global pg.ship_data_template.all")?;
        let ship_data_statistics: LuaTable = pg.get("ship_data_statistics").context("global pg.ship_data_statistics")?;

        // Normal enhancement data (may be present even if not used for that ship):
        let ship_data_strengthen: LuaTable = pg.get("ship_data_strengthen").context("global pg.ship_data_strengthen")?;

        // Blueprint/Research ship data:
        let ship_data_blueprint: LuaTable = pg.get("ship_data_blueprint").context("global pg.ship_data_blueprint")?;
        let ship_strengthen_blueprint: LuaTable = pg.get("ship_strengthen_blueprint").context("global pg.ship_strengthen_blueprint")?;

        // META ship data:
        let ship_strengthen_meta: LuaTable = pg.get("ship_strengthen_meta").context("global pg.ship_strengthen_meta")?;
        let ship_meta_repair: LuaTable = pg.get("ship_meta_repair").context("global pg.ship_meta_repair")?;
        let ship_meta_repair_effect: LuaTable = pg.get("ship_meta_repair_effect").context("global pg.ship_meta_repair_effect")?;

        // Retrofit data:
        let ship_data_trans: LuaTable = pg.get("ship_data_trans").context("global pg.ship_data_trans")?;
        let transform_data_template: LuaTable = pg.get("transform_data_template").context("global pg.transform_data_template")?;

        // Skin/word data:
        let ship_skin_template: LuaTable = pg.get("ship_skin_template").context("global pg.ship_skin_template")?;
        let ship_skin_template_get_id_list_by_ship_group: LuaTable = ship_skin_template.get("get_id_list_by_ship_group").context("global pg.ship_skin_template.get_id_list_by_ship_group")?;
        let ship_skin_words: LuaTable = pg.get("ship_skin_words").context("global pg.ship_skin_words")?;
        let ship_skin_words_extra: LuaTable = pg.get("ship_skin_words_extra").context("global pg.ship_skin_words_extra")?;

        let mut groups = HashMap::new();
        ship_data_template_all.for_each(|_: u32, id: u32| {
            if (900000..=900999).contains(&id) {
                return Ok(())
            }

            let template: LuaTable = ship_data_template.get(id).with_context(context!("ship_data_template with id {id}"))?;
            let group_id: u32 = template.get("group_type").with_context(context!("group_type of ship_data_template with id {id}"))?;

            groups.entry(group_id)
                .or_insert_with(|| ShipGroup { id: group_id, members: Vec::new() })
                .members.push(id);

            Ok(())
        })?;

        println!("Ship groups: {} ({:.2?})", groups.len(), start.elapsed());

        let make_ship_set = |id: u32| -> LuaResult<ShipSet> {
            let template: LuaTable = ship_data_template.get(id).with_context(context!("!ship_data_template with id {id}"))?;
            let statistics: LuaTable = ship_data_statistics.get(id).with_context(context!("ship_data_statistics with id {id}"))?;

            let strengthen_id: u32 = template.get("strengthen_id").with_context(context!("strengthen_id of ship_data_template with id {id}"))?;
            let _: u32 = template.get("id").with_context(context!("id of ship_data_template with id {id}"))?;

            let enhance: Option<LuaTable> = ship_data_strengthen.get(strengthen_id).with_context(context!("ship_data_strengthen with {id}"))?;
            let blueprint: Option<LuaTable> = ship_data_blueprint.get(strengthen_id).with_context(context!("ship_data_blueprint with {id}"))?;
            let meta: Option<LuaTable> = ship_strengthen_meta.get(strengthen_id).with_context(context!("ship_strengthen_meta with {id}"))?;

            let strengthen = match (enhance, blueprint, meta) {
                (_, Some(data), _) => Strengthen::Blueprint(BlueprintStrengthen { data, effect_lookup: &ship_strengthen_blueprint }),
                (_, _, Some(data)) => Strengthen::META(MetaStrengthen { data, repair_lookup: &ship_meta_repair, repair_effect_lookup: &ship_meta_repair_effect }),
                (Some(data), _, _) => Strengthen::Normal(data),
                _ => Err(LuaError::external(DataError::NoStrengthen))?
            };

            let retrofit: Option<LuaTable> = ship_data_trans.get(strengthen_id).with_context(context!("ship_data_trans with {id}"))?;
            let retrofit = retrofit.map(|r| Retrofit { data: r, list_lookup: &transform_data_template });

            Ok(ShipSet {
                id,
                template,
                statistics,
                strengthen,
                retrofit_data: retrofit
            })
        };

        let config = &*CONFIG;
        let mut ships = groups.into_values().map(|group| {
            let members = group.members.into_iter()
                .map(make_ship_set)
                .collect::<LuaResult<Vec<_>>>()?;

            let mlb_max_id = group.id * 10 + 4;
            let Some(raw_mlb) = members.iter().filter(|t| t.id <= mlb_max_id).max_by_key(|t| t.id) else {
                Err(LuaError::external(DataError::NoMlb).context(format!("no mlb for ship with id {}", group.id)))?
            };

            let raw_retrofits: Vec<&ShipSet> = members.iter().filter(|t| t.id > raw_mlb.id).collect();

            let raw_skins: Vec<u32> = ship_skin_template_get_id_list_by_ship_group.get(group.id).with_context(context!("skin ids for ship with id {}", group.id))?;
            let raw_skins = raw_skins.into_iter().map(|skin_id| Ok(SkinSet {
                skin_id,
                template: ship_skin_template.get(skin_id).with_context(context!("skin template {} for ship {}", skin_id, group.id))?,
                words: ship_skin_words.get(skin_id).with_context(context!("skin words {} for ship {}", skin_id, group.id))?,
                words_extra: ship_skin_words_extra.get(skin_id).with_context(context!("skin words extra {} for ship {}", skin_id, group.id))?,
            })).collect::<LuaResult<Vec<_>>>()?;

            let mut mlb = parse::ship::load_ship_data(&lua, raw_mlb)?;
            if let Some(name_override) = config.name_overrides.get(&mlb.group_id) {
                mlb.name.clone_from(name_override);
            }

            if let Some(retrofit_data) = &raw_mlb.retrofit_data {
                for retrofit_set in raw_retrofits {
                    let mut retrofit = parse::ship::load_ship_data(&lua, retrofit_set)?;
                    enhance::retrofit::apply_retrofit(&lua, &mut retrofit, retrofit_data)?;

                    fix_up_retrofitted_data(&mut retrofit, retrofit_set)?;
                    mlb.retrofits.push(retrofit);
                }

                if mlb.retrofits.is_empty() {
                    let mut retrofit = mlb.clone();
                    enhance::retrofit::apply_retrofit(&lua, &mut retrofit, retrofit_data)?;

                    fix_up_retrofitted_data(&mut retrofit, raw_mlb)?;
                    mlb.retrofits.push(retrofit);
                }
            }

            for raw_skin in raw_skins {
                mlb.skins.push(parse::skin::load_skin(&raw_skin)?);
            }

            Ok(mlb)
        }).collect::<anyhow::Result<Vec<_>>>()?;

        println!("Built Ship data. ({:.2?})", start.elapsed());

        ships.sort_unstable_by_key(|t| t.group_id);
        ships
    };

    let equips = {
        let equip_data_template: LuaTable = pg.get("equip_data_template").context("global pg.equip_data_template")?;
        let equip_data_template_all: LuaTable = equip_data_template.get("all").context("global pg.equip_data_template.all")?;
        let equip_data_statistics: LuaTable = pg.get("equip_data_statistics").context("global pg.equip_data_statistics")?;

        let mut equips = Vec::new();
        equip_data_template_all.for_each(|_: u32, id: u32| {
            let template: LuaTable = equip_data_template.get(id).with_context(context!("equip_data_template with id {id}"))?;
            let statistics: LuaTable = equip_data_statistics.get(id).with_context(context!("equip_data_statistics with id {id}"))?;

            let next: u32 = template.get("next").with_context(context!("base of equip_data_template with id {id}"))?;
            let tech: u32 = statistics.get("tech").with_context(context!("tech of equip_data_statistics with id {id}"))?;
            if next == 0 && matches!(tech, 0 | 3..) {
                equips.push(id);
            }

            Ok(())
        })?;

        println!("Equips: {} ({:.2?})", equips.len(), start.elapsed());

        let mut equips = equips.into_iter().map(|id| {
            parse::skill::load_equip(&lua, id)
        }).collect::<LuaResult<Vec<_>>>()?;

        println!("Built Equip data. ({:.2?})", start.elapsed());

        equips.sort_unstable_by_key(|t| (t.faction, t.kind, t.equip_id));
        equips
    };

    let augments = {
        let spweapon_data_statistics: LuaTable = pg.get("spweapon_data_statistics").context("global pg.spweapon_data_statistics")?;
        let spweapon_data_statistics_all: LuaTable = spweapon_data_statistics.get("all").context("global pg.spweapon_data_statistics.all")?;

        let mut groups: HashMap<u32, u32> = HashMap::new();
        spweapon_data_statistics_all.for_each(|_: u32, id: u32| {
            let statistics: LuaTable = spweapon_data_statistics.get(id).with_context(context!("spweapon_data_statistics with id {id}"))?;

            let base_id: Option<u32> = statistics.get("base").with_context(context!("base of spweapon_data_statistics with id {id}"))?;
            let base_id = base_id.unwrap_or(id);

            groups.entry(base_id)
                .and_modify(|e| if *e < id { *e = id })
                .or_insert(id);

            Ok(())
        })?;

        println!("Augments: {} ({:.2?})", groups.len(), start.elapsed());

        let mut augments = groups.into_values().map(|id| {
            let statistics: LuaTable = spweapon_data_statistics.get(id).with_context(context!("spweapon_data_statistics with id {id}"))?;
            let data = AugmentSet { id, statistics };
            parse::augment::load_augment(&lua, &data)
        }).collect::<LuaResult<Vec<_>>>()?;

        println!("Built Augment data. ({:.2?})", start.elapsed());

        augments.sort_unstable_by_key(|t| t.augment_id);
        augments
    };

    // i have no idea why this deadlocks, but i don't care.
    // when the program exits, the memory will be cleaned up.
    if cfg!(debug_assertions) {
        drop(pg);
        std::mem::forget(lua);
    }

    Ok(DefinitionData {
        ships,
        equips,
        augments
    })
}

fn fix_up_retrofitted_data(ship: &mut ShipData, set: &ShipSet) -> LuaResult<()> {
    let buff_list_display: Vec<u32> = set.template.get("buff_list_display")?;
    ship.skills.sort_by_key(|s| {
        buff_list_display.iter().enumerate()
            .find(|i| *i.1 == s.buff_id)
            .map(|i| i.0)
            .unwrap_or_default()
    });

    Ok(())
}

fn merge_out_data(main: &mut DefinitionData, next: DefinitionData) {
    for next_ship in next.ships {
        if let Some(main_ship) = main.ships.iter_mut().find(|s| s.group_id == next_ship.group_id) {
            add_missing(&mut main_ship.retrofits, next_ship.retrofits, |a, b| a.default_skin_id == b.default_skin_id);
            add_missing(&mut main_ship.skins, next_ship.skins, |a, b| a.skin_id == b.skin_id);
        } else {
            main.ships.push(next_ship);
        }
    }

    add_missing(&mut main.augments, next.augments, |a, b| a.augment_id == b.augment_id);
    add_missing(&mut main.equips, next.equips, |a, b| a.equip_id == b.equip_id);
}

fn add_missing<T>(main: &mut Vec<T>, next: Vec<T>, matches: impl Fn(&T, &T) -> bool) {
    for new in next {
        if !main.iter().any(|old| matches(old, &new)) {
            main.push(new);
        }
    }
}
