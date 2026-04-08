use utils_build::ensure;

fn main() {
    if precompile_lua_init().is_err() {
        std::process::exit(1);
    }

    utils_build::embed_windows_resources();
    utils_build::include_git_commit_hash();
}

fn precompile_lua_init() -> Result<(), ensure::PrintErr> {
    use std::path::Path;
    use std::{env, fs};

    use mlua::prelude::*;

    println!("cargo::rerun-if-changed=assets/lua_init.lua");

    let out_dir =
        env::var("OUT_DIR").map_err(|why| ensure::print_err!("cannot get $OUT_DIR: {why}"))?;
    let content = fs::read_to_string("assets/lua_init.lua")
        .map_err(|why| ensure::print_err!("cannot read lua_init: {why}"))?;

    let lua = Lua::new();
    let func = lua
        .load(content)
        .set_name("main")
        .set_mode(mlua::ChunkMode::Text)
        .into_function()
        .map_err(|why| ensure::print_err!("cannot compile lua_init: {why}"))?;

    let bytecode = func.dump(false);

    let out_file = Path::new(&out_dir).join("compiled-lua_init.lua");
    fs::write(out_file, bytecode)
        .map_err(|why| ensure::print_err!("cannot write compiled lua_init: {why}"))
}
