use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use clap::Parser;
use mlua::prelude::*;
use azur_lane::*;
use azur_lane::ship::*;

mod macros;
mod convert_al;
mod enhance;
mod skill_loader;
mod model;

use model::*;

#[derive(Debug, Parser)]
struct Cli {
    /// The path that the game scripts live in.
    #[arg(short, long)]
    input: String,
    /// The output file name.
    #[arg(short, long)]
    out: Option<String>,
    /// Minimize the output JSON file.
    #[arg(short, long)]
    minimize: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let lua = Lua::new();

    lua.globals().raw_set("AZUR_LANE_DATA_PATH", cli.input)?;
    lua.load(include_str!("assets/lua_init.lua"))
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .exec()?;

    let name_overrides: HashMap<u32, String> = serde_json::from_str(include_str!("assets/name_overrides.json"))?;

    println!("Init done.");

    // General:
    let pg: LuaTable = context!(lua.globals().get("pg"); "global pg")?;
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

    let mut groups = HashMap::new();
    ship_data_template_all.for_each(|_: u32, id: u32| {
        if id >= 900000 && id <= 900999 {
            return Ok(())
        }

        let template: LuaTable = context!(ship_data_template.get(id); "ship_data_template with id {id}")?;
        let statistics: LuaTable = context!(ship_data_statistics.get(id); "ship_data_statistics with id {id}")?;
        
        let group_id: u32 = context!(template.get("group_type"); "group_type of ship_data_template with id {id}")?;
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

        let ship = ShipSet {
            id,
            template,
            statistics,
            strengthen,
            retrofit_data: retrofit
        };

        groups.entry(group_id)
            .or_insert_with(|| Group { id: group_id, tables: Vec::new() })
            .tables.push(ship);

        Ok(())
    })?;

    let mut candidates = Vec::new();
    for group in groups.into_values() {
        let mlb_max_id = group.id * 10 + 4;
        let Some(mlb) = group.tables.iter().filter(|t| t.id <= mlb_max_id).max_by_key(|t| t.id) else {
            Err(LuaError::external(DataError::NoMlb).with_context(|_| format!("no mlb for ship with id {}", group.id)))?
        };

        let retrofits = group.tables.iter().filter(|t| t.id > mlb.id).cloned().collect();

        candidates.push(ShipCandidate {
            id: group.id,
            mlb: mlb.clone(),
            retrofits,
            retrofit_data: mlb.retrofit_data.clone()
        });
    }

    candidates.sort_by_key(|t| t.id);
    println!("candidates: {}", candidates.len());

    let mut ships = Vec::new();
    for candidate in candidates {
        let mut mlb = candidate.mlb.to_ship_data(&lua)?;
        if let Some(name_override) = name_overrides.get(&mlb.group_id) {
            mlb.name = Arc::from(name_override.as_str());
        }
        
        let mut retrofits: Vec<ShipData> = Vec::new();
        if let Some(ref retrofit_data) = candidate.retrofit_data {
            for retrofit_set in candidate.retrofits {
                let mut retrofit = retrofit_set.to_ship_data(&lua)?;
                enhance::retrofit::apply_retrofit(&lua, &mut retrofit, &retrofit_data)?;
    
                fix_up_retrofitted_data(&mut retrofit, &retrofit_set)?;
                retrofits.push(retrofit);
            }

            if retrofits.is_empty() {
                let mut retrofit = mlb.clone();
                enhance::retrofit::apply_retrofit(&lua, &mut retrofit, &retrofit_data)?;

                fix_up_retrofitted_data(&mut retrofit, &candidate.mlb)?;
                retrofits.push(retrofit); 
            }
        }

        mlb.retrofits = Arc::from(retrofits);
        ships.push(mlb);
    }

    println!("Writing output...");

    let out_path = cli.out.as_deref().unwrap_or("houston_azur_lane_data.json");
    let f = fs::File::create(out_path)?;
    let out_data = DefinitionData {
        ships
    };

    if cli.minimize {
        serde_json::to_writer(&f, &out_data)?;
    } else {
        serde_json::to_writer_pretty(&f, &out_data)?;
    }

    println!("Written {} bytes.", f.metadata()?.len());
    drop(f);

    Ok(())
}

fn fix_up_retrofitted_data(ship: &mut ShipData, set: &ShipSet) -> LuaResult<()> {
    let buff_list_display: Vec<u32> = set.template.get("buff_list_display")?;
    let mut skills = ship.skills.to_vec();
    skills.sort_by_key(|s| buff_list_display.iter().enumerate().find(|i| *i.1 == s.buff_id).map(|i| i.0).unwrap_or_default());
    ship.skills = Arc::from(skills);

    Ok(())
}
