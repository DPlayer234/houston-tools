# Houston Tools

Small Discord bot intended to be self-hosted.

Optionally loads Azur Lane game data collected by the Azur Lane Data Collector in this repo.

> [!WARNING]
> This branch uses serenity and poise "next" versions.

## Setup

The minimum setup requires setting the environment variable `DISCORD__TOKEN` to your Discord bot token (note: 2 underscores).

Upon startup, it will register its commands globally.

Configuration is supported either via environent variables or a file named `houston_app.toml` in the working directory. The TOML config has this structure:

```toml
[discord]
# this is the same as the DISCORD__TOKEN env variable. only one must be specified.
token = "..."

[bot]
# relative or absolute path to the data produced by the Azur Lane Data Collector.
# optional. when not present, disables the Azur Lane module.
azur_lane_data = "azur_lane_data"

# optional section to configure the terminal output.
# the logger writes to `stderr`.
[log]
# sets whether the logger should use colors.
# optional. attempts to auto-detect whether the output supports colors.
color = true

# sets the default minimum log level.
# optional. defaults to "warn", with "trace" for just the app itself.
default = "warn"

# may specify additional keys matching module names to filter their messages.
# log levels may be "trace", "debug", "info", "warn", or "error".
```

## Commands

Here is a quick overview of the supported commands:

| Command      | Description |
|:------------ |:----------- |
| calc         | Evaluates a mathematical equation. |
| config       | Provides (temporary) configuration for this app. |
| config hide  | Configures whether responses to your commands are hidden from other users. |
| coin         | Flips a coin. |
| dice         | Rolls some dice. |
| timestamp    | Provides methods for localized timestamps. |
| timestamp in | Gets a timestamp offset from the current time. |
| timestamp at | Gets a timestamp at the specified time. |
| timestamp of | Gets the creation timestamp from a Discord snowflake. |
| upload       | Uploads a file to an ephemeral message. Allows sharing if you are logged into multiple devices. |
| who          | Returns basic information about the provided user. |

The following commands are supported in context menus:

| Command      | Description |
|:------------ |:----------- |
| Get as Quote | (Message) Copies a format that is appropriate to use as a quote to crosspost. |
| User Info    | (User) Equivalent to `/who`. |

Additionally, when Azur Lane data is loaded, the azur command becomes available. Commands accepting names support fuzzy autocomplete.

| Command             | Description |
|:------------------- |:----------- |
| azur                | Information about mobile game Azur Lane. |
| azur ship           | Shows information about a ship. |
| azur search-ship    | Searches for ships. |
| azur equip          | Shows information about equipment. |
| azur search-equip   | Searches for equipment. |
| azur augment        | Shows information about an augment module. |
| azur search-augment | Searches for augment modules. |
| azur reload-time    | Calculates the actual reload time for a weapon. |

## Features requiring a database

The following features are optional and require a MongoDB database. Configure its URI in the config, f.e.:

```toml
[bot]
mongodb_uri = "mongodb://localhost/houston-tools"
```

### Starboard

Starboard will forward messages with a certain amount of reactions to another channel.
Furthermore, for each board, it will track a leaderboard score.

Starboard must be configured:

```toml
[[bot.starboard]]
guild = 1293210831923974204
channel = 1305620816272166962
emoji = "⭐"
reacts = 3
notices = [
    "An amazing post, {user}!",
    "{user}, the stars aligned.",
]

[[bot.starboard]]
guild = 1293210831923974204
channel = 1305620834450407606
emoji = "💀"
reacts = 3
notices = [
    "What a stinker, {user}!",
    "{user}, please stop.",
]
```

The following commands will be enabled:

| Command             | Description |
|:------------------- |:----------- |
| starboard top       | Shows a board's top users. |
| starboard top-posts | Shows the most-reacted posts in a board. |

# Azur Lane Data Collector

> [!WARNING]
> The collector *runs* the game scripts. As should be common sense, do not run untrusted code.

This is a command line tool that loads Azur Lane game scripts and outputs data to be used and displayed by the Discord bot.

## Use

```
  -i, --inputs <INPUTS>...  The path that the game scripts live in
  -o, --out <OUT>           The output directory
      --assets <ASSETS>     The path that holds the game assets
  -m, --minimize            Minimize the output JSON file
      --color <COLOR>       Override whether this program outputs color [possible values: true, false]
  -h, --help                Print help
```

`--inputs` is required. `--out` defaults to `azur_lane_data`.

`--inputs` specifies a path to decompiled game scripts, including unpacked `sharecfgdata`.
It is expected that `sharecfgdata/<asset-type>.lua` will load all entries when executed.

If `--assets` is specified, it will look for a folder within it named `shipmodels` that is searched for Unity asset bundles for extracting chibi images of the ships.
In essence, if you copy the `shipmodels` folder from the game's data and point to the parent directory, it should work.
If it is not specified, this step is skipped.

## Lua

Currently the collector defaults to using Lua 5.4 rather than LuaJIT. This is in part due to unpacked `sharecfgdata` files commonly being a merged decompilation output that cannot be loaded by LuaJIT due to too many constants.

If you want to switch to a different Lua edition, edit the enabled features of the `mlua` dependency. This isn't _perfectly_ supported, but if it compiles, it should be fine.

## Multiple Inputs

If you specify multiple input directories, the data is "merged". That is, ships, equipment, retrofits, and skins will added to earlier sets of data.
The first set that contains a certain entry will take priority.

## Terminal Output

The program will print its terminal output to _stderr_, attempting to use ANSI escapes to improve the output.
There is no _stdout_ output.

By default, the program will _try_ to detect whether the output supports colors. Notably, non-terminal outputs and environment that specify `NO_COLOR` will not have color.

In case the detection ends up being wrong, you may pass `--color=true` or `--color=false` to override the detection.

# Build

This is a standard Rust workspace. If you are already familiar with Cargo and the Rust toolchain, you should not need any further instructions.

Install the stable Rust toolchain if you haven't already, then invoke cargo for release builds:
```
cargo build --release
```

Alternatively, you can run the executables directly as:
```
cargo run --bin houston_app
cargo run --bin azur_lane_data_collector -- --inputs ...
```

## Release Options

As present in this repository, the release builds specify some additional options:

- [Fat LTO is enabled.](https://doc.rust-lang.org/rustc/codegen-options/index.html#lto) Compilation may be slow, but the output should be better.
- [Panics will abort](https://doc.rust-lang.org/rustc/codegen-options/index.html#panic) rather than unwind.

Edit the workspace's Cargo.toml if you prefer other behavior.

# License

MIT, see LICENSE.
