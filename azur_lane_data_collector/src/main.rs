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
    #[arg(short, long)]
    input: String,
    /// The output directory.
    #[arg(short, long)]
    out: Option<String>,

    /// The path that holds the game assets.
    #[arg(long)]
    assets: Option<String>,

    /// Minimize the output JSON file.
    #[arg(short, long)]
    minimize: bool
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let lua = Lua::new();

    let start = std::time::Instant::now();

    lua.globals().raw_set("AZUR_LANE_DATA_PATH", cli.input)?;
    lua.load(include_str!("assets/lua_init.lua"))
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .exec()?;

    println!("Init done. ({:.2?})", start.elapsed());

    // General:
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;

    let ships = {
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

        // Skin/word data:
        let ship_skin_template: LuaTable = context!(pg.get("ship_skin_template"); "global pg.ship_skin_template")?;
        let ship_skin_template_get_id_list_by_ship_group: LuaTable = context!(ship_skin_template.get("get_id_list_by_ship_group"); "global pg.ship_skin_template.get_id_list_by_ship_group")?;
        let ship_skin_words: LuaTable = context!(pg.get("ship_skin_words"); "global pg.ship_skin_words")?;
        let ship_skin_words_extra: LuaTable = context!(pg.get("ship_skin_words_extra"); "global pg.ship_skin_words_extra")?;

        let mut groups = HashMap::new();
        ship_data_template_all.for_each(|_: u32, id: u32| {
            if id >= 900000 && id <= 900999 {
                return Ok(())
            }

            let template: LuaTable = context!(ship_data_template.get(id); "ship_data_template with id {id}")?;
            let group_id: u32 = context!(template.get("group_type"); "group_type of ship_data_template with id {id}")?;

            groups.entry(group_id)
                .or_insert_with(|| ShipGroup { id: group_id, members: Vec::new() })
                .members.push(id);

            Ok(())
        })?;

        println!("Ship groups: {} ({:.2?})", groups.len(), start.elapsed());

        let make_ship_set = |id: u32| -> LuaResult<ShipSet> {
            let template: LuaTable = context!(ship_data_template.get(id); "!ship_data_template with id {id}")?;
            let statistics: LuaTable = context!(ship_data_statistics.get(id); "ship_data_statistics with id {id}")?;

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
                Err(LuaError::external(DataError::NoMlb).with_context(|_| format!("no mlb for ship with id {}", group.id)))?
            };

            let raw_retrofits: Vec<&ShipSet> = members.iter().filter(|t| t.id > raw_mlb.id).collect();

            let raw_skins: Vec<u32> = context!(ship_skin_template_get_id_list_by_ship_group.get(group.id); "skin ids for ship with id {}", group.id)?;
            let raw_skins = raw_skins.into_iter().map(|skin_id| Ok(SkinSet {
                skin_id,
                template: context!(ship_skin_template.get(skin_id); "skin template {} for ship {}", skin_id, group.id)?,
                words: context!(ship_skin_words.get(skin_id); "skin words {} for ship {}", skin_id, group.id)?,
                words_extra: context!(ship_skin_words_extra.get(skin_id); "skin words extra {} for ship {}", skin_id, group.id)?,
            })).collect::<LuaResult<Vec<_>>>()?;

            let mut mlb = parse::ship::load_ship_data(&lua, &raw_mlb)?;
            if let Some(name_override) = config.name_overrides.get(&mlb.group_id) {
                mlb.name = name_override.clone();
            }

            if let Some(ref retrofit_data) = raw_mlb.retrofit_data {
                for retrofit_set in raw_retrofits {
                    let mut retrofit = parse::ship::load_ship_data(&lua, &retrofit_set)?;
                    enhance::retrofit::apply_retrofit(&lua, &mut retrofit, retrofit_data)?;

                    fix_up_retrofitted_data(&mut retrofit, &retrofit_set)?;
                    mlb.retrofits.push(retrofit);
                }

                if mlb.retrofits.is_empty() {
                    let mut retrofit = mlb.clone();
                    enhance::retrofit::apply_retrofit(&lua, &mut retrofit, retrofit_data)?;

                    fix_up_retrofitted_data(&mut retrofit, &raw_mlb)?;
                    mlb.retrofits.push(retrofit);
                }
            }

            for raw_skin in raw_skins {
                mlb.skins.push(parse::skin::load_skin(&raw_skin)?);
            }

            Ok(mlb)
        }).collect::<anyhow::Result<Vec<_>>>()?;

        println!("Built Ship data. ({:.2?})", start.elapsed());

        ships.sort_by_key(|t| t.group_id);
        ships
    };

    let augments = {
        let spweapon_data_statistics: LuaTable = context!(pg.get("spweapon_data_statistics"); "global pg.spweapon_data_statistics")?;
        let spweapon_data_statistics_all: LuaTable = context!(spweapon_data_statistics.get("all"); "global pg.spweapon_data_statistics.all")?;

        let mut groups: HashMap<u32, u32> = HashMap::new();
        spweapon_data_statistics_all.for_each(|_: u32, id: u32| {
            let statistics: LuaTable = context!(spweapon_data_statistics.get(id); "spweapon_data_statistics with id {id}")?;

            let base_id: Option<u32> = context!(statistics.get("base"); "base of spweapon_data_statistics with id {id}")?;
            let base_id = base_id.unwrap_or(id);

            groups.entry(base_id)
                .and_modify(|e| { if *e < id { *e = id } })
                .or_insert(id);

            Ok(())
        })?;

        println!("Augments: {} ({:.2?})", groups.len(), start.elapsed());

        let mut augments = groups.into_values().map(|id| {
            let statistics: LuaTable = context!(spweapon_data_statistics.get(id); "spweapon_data_statistics with id {id}")?;
            let data = AugmentSet { id, statistics };
            parse::augment::load_augment(&lua, &data)
        }).collect::<LuaResult<Vec<_>>>()?;

        println!("Built Augment data. ({:.2?})", start.elapsed());

        augments.sort_by_key(|t| t.augment_id);
        augments
    };

    let out_dir = cli.out.as_deref().unwrap_or("azur_lane_data");
    if let Some(assets) = cli.assets.as_deref() {
        // Extract and save chibis for all skins.
        fs::create_dir_all(Path::new(out_dir).join("chibi"))?;

        println!("Extracting chibis...");

        let mut extract_count = 0usize;
        let mut total_count = 0usize;
        let mut new_count = 0usize;

        for skin in ships.iter().flat_map(|s| s.skins.iter()) {
            total_count += 1;

            if let Some(image) = parse::image::load_chibi_image(assets, &skin.image_key)? {
                extract_count += 1;

                let path = Path::new(out_dir).join("chibi").join(&skin.image_key);
                if let Ok(mut f) = fs::OpenOptions::new().create_new(true).write(true).open(path) {
                    new_count += 1;

                    f.write_all(&image)?;
                }
            }
        }

        println!("Extracted chibis ({extract_count}/{total_count}); {new_count} new. {:.2?}", start.elapsed());
    } else {
        fs::create_dir_all(out_dir)?;
    }

    println!("Writing output...");

    let f = fs::File::create(Path::new(out_dir).join("main.json"))?;
    let out_data = DefinitionData {
        ships,
        augments
    };

    if cli.minimize {
        serde_json::to_writer(&f, &out_data)?;
    } else {
        serde_json::to_writer_pretty(&f, &out_data)?;
    }

    println!("Written {} bytes. ({:.2?})", f.metadata()?.len(), start.elapsed());
    drop(f);

    Ok(())
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
