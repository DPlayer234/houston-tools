# Houston Tools

Small Discord bot intended to be self-hosted.

Optionally loads Azur Lane game data collected by the Azur Lane Data Collector in this repo.

This bot uses serenity "next" versions, so there may be sudden internal changes when those update.

## Setup

The minimum setup requires setting the environment variable `DISCORD__TOKEN` to your Discord bot token (note: 2 underscores).

Upon startup, it will register its commands globally.

Configuration is supported either via environment variables or a file named `houston_app.toml` in the working directory. The TOML config has this structure:

```toml
[discord]
# this is the same as the `DISCORD__TOKEN` environment variable.
# if both are specified, the environment variable takes priority.
token = "..."

[bot]
# optional. defaults to 0xDDA0DD
# sets the color used for most embeds.
embed_color = 0xDDA0DD
```

Going forward, only the TOML config will be explained because the configuration via environment variables is very limited.

To configure logging, see the Logging section further down.

Additionally, based on the environment variable `HOUSTON_PROFILE`, it will also try to load `houston_app.$(HOUSTON_PROFILE).toml`. Its properties will take priority over the main config file. If the environment variable isn't set, it is considered to be `release`, so it will try `houston_app.release.toml`.

> [!IMPORTANT]
> While tables are merged across the loaded configuration files, when an array is encountered, the array in the profile's config will override the same array in the base config entirely. For example, if you specify starboards in `houston_app.toml` **and** `houston_toml.release.toml`, only the starboards in `houston_toml.release.toml` will be enabled.

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

The bot also has a couple of minigames:

| Command                      | Description |
|:---------------------------- |:----------- |
| minigame tic-tac-toe         | Play tic-tac-toe with someone else. |
| minigame rock-paper-scissors | Play rock-paper-scissors with someone else. |
| minigame chess               | Play, uh, "chess" with someone else. |

## Features requiring a database

The following features are optional and require a MongoDB database. Configure its URI in the config, f.e.:

```toml
[bot]
mongodb_uri = "mongodb://localhost/houston-tools"
```

Note that the default database name is required as part of the URI and will be the database used.

### Starboard

Starboard will forward messages with a certain amount of reactions to another channel.
Furthermore, for each board, it will track a leaderboard score.

Starboard must be configured:

```toml
# the first numeric key here is the guild id.
[[bot.starboard.1293210831923974204.boards]]
# the id is important!
id = 1
name = "starboard"
channel = 1305620816272166962
emoji = "⭐"
reacts = 3
notices = [
    "An amazing post, {user}!",
    "{user}, the stars aligned.",
]

[[bot.starboard.1293210831923974204.boards]]
id = 2
name = "ripboard"
channel = 1305620834450407606
emoji = "wowie:1305835613790146631"
reacts = 3
notices = [
    "What a stinker, {user}!",
    "{user}, please stop.",
]
```

The board ID is used database-side to identify the board globally. As such, the board ID must be unique _globally_, not just per guild. Moving a board to another channel or emoji may have side effects but it won't break the scores.

The board emoji must either be a unicode emoji or "&lt;name&gt;:&lt;id&gt;", i.e. "wowie:1305835613790146631". The board emojis must be unique per guild. Unicode emojis are matched exactly, while custom emojis are matched by ID. The bot must be able to post to the channel.

The board channel is not required to be unique and multiple boards may use the same channel.

Also note that messages in nsfw channels are still tracked for sfw board channels. In this case, a small embed with a message link will be posted instead of a forward. If the board channel itself is nsfw, it will always be a forward.

The name is purely cosmetic and may be displayed in places where a channel name may be expected but channel mentions aren't valid.

The order that the boards are declared in will be used for display, like in the overview or during auto-complete of board names.

The following commands will be enabled:

| Command             | Description |
|:------------------- |:----------- |
| starboard top       | Shows a board's top users. |
| starboard top-posts | Shows the most-reacted posts in a board. |
| starboard overview  | Shows an overview of all boards. |

### Perks

Perks enables a currency system and a store to buy perks with.

This comes with the following configuration:

```toml
[bot.perks]
# optional. sets the display name of the currency
cash_name = "$"
# optional. the minimum time between perk checks. defaults to 3 minutes.
# the default is usually fine and you shouldn't need to adjust it.
check_interval = "00:03:00"

[[bot.starboard.1293210831923974204.boards.1]]
...
# in addition to the other options, you can also specify these on starboards:
# cash_gain: users will get as much currency per vote as specified here.
# only relevant once the message is pinned. at that point, cash for all votes is added.
cash_gain = 2
# additional gain for a pin.
cash_pin_gain = 10

# collectible enables an item with no inherent purpose.
# it can be repeatedly bought in the perk store.
[bot.perks.collectible]
name = "Crab Plushy 🦀"
description = "Necessary for every rustacean."
cost = 4

# you may also set prize roles for owning the collectible
[bot.perks.collectible.1293210831923974204]
# the notice part is optional
# if set, you need both the channel and text
notice.channel = 1293210831923974207
notice.text = "Look, look! {user} reached {role}!"
# these are pairs of (needed, role_id)
# only checked on purchase
prize_roles = [
    [20, 1309970796516610119],
    [40, 1309970817882259498],
    [80, 1309970846491742339],
    [160, 1309970845531246633],
]

# rainbow enables rainbow roles.
# the color cycle is dependent on `check_interval`
# the check interval must not be below (00:02:00) if rainbow role is enabled or you may hit rate limits.
[bot.perks.rainbow]
cost = 20
# duration is specified as HH:MM:SS.
duration = "24:00:00"

# configures a role for a server.
# the bot must have "Manage Roles" and have its role placed above it for this to work correctly.
# the perk will only be purchasable in servers configured here.
1293210831923974204.role = 1305905884807041124

# pushpin enables an item that lets someone pin/unpin a message
# to use the item, they need to use the context menu commands
[bot.perks.pushpin]
name = "Pushpin"
description = "Allow pinning or unpinning a message."
cost = 40

# role_edit enables an item that lets someone edit their unique role
# to change the role, they need to use /role-edit while owning this item
# to set the role, an admin must use `/perk-admin unique-role`
[bot.perks.role_edit]
name = "Orb of Change"
description = "Allows editing your unique role color and/or name."
cost = 10

# birthday enables birthday reminders and "gifts"
# when someone's birthday begins, in every configured guild, they will get
# the birthday role and the configured gift, as well a message from the bot.
[bot.perks.birthday]
# optional. defaults to 24 hours. should always be at least 24 h.
duration = "24:00:00"

# define some regions here. the first is considered the default.
# this allows users to pick when they want their birthday to start.
# note that the index into this is stored in the database.
[[bot.perks.birthday.regions]]
name = "Europe/Africa (UTC+0)"
time_offset = "0:00:00"

[[bot.perks.birthday.regions]]
name = "Americas (UTC-8)"
time_offset = "-8:00:00"

[[bot.perks.birthday.regions]]
name = "SEA (UTC+7)"
time_offset = "7:00:00"

# also configure a server
[bot.perks.birthday.1293210831923974204]
role = 1316802158070595725
notice.channel = 1293210831923974207
notice.text = "Happy birthday, {user}!"
# the gifts are pairs of (Item, amount).
# valid items are: Cash, Pushpin, RoleEdit, Collectible
gifts = [
    ["Cash", 500],
]
```

The following commands will be enabled:

| Command                | Description |
|:---------------------- |:----------- |
| birthday add           | Add your birthday. |
| birthday check         | Checks your set birthday. |
| birthday time-zone     | Sets your birthday time zone. |
| perk-admin enable      | Enables a perk for a member. |
| perk-admin disable     | Disables a perk for a member. |
| perk-admin list        | List active perks of a member. |
| perk-admin give        | Gives a user items. |
| perk-admin unique-role | Sets a user's unique role. Can be omitted to delete the association. |
| role-edit              | Edit your unique role. |
| shop                   | View the server shop. |
| wallet                 | View your server wallet. |

The following commands are supported in context menus:

| Command            | Description |
|:------------------ |:----------- |
| Use Pushpin: Pin   | (Message) Pin this message. |
| Use Pushpin: Unpin | (Message) Unpin this message. |

Commands are only available when the corresponding perk is enabled.

### Rep

Aka "reputation". This is purely a counter shown on the members' server profiles. Every server member may give one point to another member once per day (or any other interval you'd like). In addition, members receiving reputation can also get cash.

Reputation can only be given to users in the server, but not bots.

This comes with the following configuration:

```toml
[bot.rep]
# optional. defaults to 20 hours.
# the cooldown between `/rep` uses per user and server.
cooldown = "20:00:00"
# required. the cash gain on gaining rep.
cash = 10
```

The following commands will be enabled:

| Command | Description |
|:------- |:----------- |
| /rep    | Gives a reputation point to another server member. |

The following commands are supported in context menus:

| Command | Description |
|:------- |:----------- |
| Rep+    | (User) Equivalent to `/rep`. |

### Server Profile

This feature is enabled if at least one of starboard, rep, or perks is enabled.

The following commands will be enabled:

| Command        | Description |
|:-------------- |:----------- |
| profile        | View a member's server profile. |

The following commands are supported in context menus:

| Command        | Description |
|:-------------- |:----------- |
| Server Profile | (User) Equivalent to `/profile`. |

## Media React

Media-react has the bot automatically react to messages in certain channels. The intent is to react to "media" posts, such as images or videos, in combination with the starboard. Only normal messages, replies, and forwards from users (not bots) will ever be reacted to.

```toml
# the numeric key is the channel id
# the channel _may_ be a text channel, thread, or forum
[bot.media_react.1305620816272166962]
# the emojis to add
emojis = ["⭐"]
# optional. default to true.
# if true, this configuration will also apply to threads in this channel.
with_threads = true

# alternatively, you may define extended information for each reaction
[[bot.media_react.1305620834450407606.emojis]]
# the emoji. same as if this section was just a string
emoji = "wowie:1305835613790146631"

# optional. defines the condition for messages to get this reaction.
# possible values are:
# - "content" (default): includes a link or attachments
# - "always": reacts to all messages
# - "never": never reacts to messages
# `normal` defines the condition for regular messages
# `forward` defines the condition for messages forwarded to the channel
condition.normal = "content"
condition.forward = "content"
# you may also set both values at once like this:
#   condition = "content"
```

There may be up to 20 emojis per channel. The "emoji" value is declared in the same way as starboard emojis, that is each value must be unicode emoji or "&lt;name&gt;:&lt;id&gt;", i.e. "wowie:1305835613790146631".

Emojis are added in declaration order.

## Azur Lane

Provides access to data collected by the Azur Lane Data Collector via commands on the bot. To enable this, specify the directory with the collected data:

```toml
[bot.azur]
# relative or absolute path to the data produced by the Azur Lane Data Collector
# that is, this points to the folder with `main.json`
data_data = "azur_lane_data"
# optional. defaults to true
# if true, loads the data on startup.
# if false, loads it the first time it's needed.
early_load = true
```

The following commands will be enabled:

| Command                | Description |
|:---------------------- |:----------- |
| azur ship              | Shows information about a ship. |
| azur equip             | Shows information about equipment. |
| azur augment           | Shows information about an augment module. |
| azur special-secretary | Shows lines for a special secretary. |
| azur juustagram-chat   | View Juustagram chats. |
| azur reload-time       | Calculates the actual reload time for a weapon. |
| azur search ship       | Searches for ships. |
| azur search equip      | Searches for equipment. |
| azur search augment    | Searches for augment modules. |
| azur search special-secretary | Searches for special secretaries. |

## Logging

Logging can be configured via the configuration file. Broadly, this is done via the "log" section, which corresponds to a [log4rs](https://docs.rs/log4rs/1.3.0/log4rs/) configuration. The configuration isn't reloaded at runtime.

By default, it adds an appender with the name "default" and an encoder of kind "default". The "default" appender kind is a console logger. The "default" encoder kind provides the standard logging format for this application.

The only standard appender available is ["rolling_file"](https://docs.rs/log4rs/1.3.0/log4rs/append/rolling_file/struct.RollingFileAppenderDeserializer.html).

For the application specific loggers, the following things are available:

```toml
[log]
# optional. disabled by default.
# when set to true, sets a panic hook that will write panic messages to the logger.
# this is somewhat pointless for debug but _should_ be harmless for prod scenarios.
panic = false

[log.root]
# if you want to add loggers, you need to specify
# the default explicitly, if you want to keep it.
appenders = ["default", "webhook"]
# the level defaults to "warn"
# for app modules, it defaults to "trace"
level = "warn"

# this section is optional and the "default" appender is always present
[log.appenders.default]
# color detection is performed, but you may override that
color = true
# the `encoder` field is also supported

[log.appenders.webhook]
# the "webhook" kind logs batched messages to a discord webhook
kind = "webhook"
# you must specify an encoder - we use the default format here
encoder.kind = "default"
# you must also specify the webhook url
url = "https://discord.com/api/webhooks/<snip>/<snip>"
# optional (32). specifies the limit of queued-up records before discarding new ones
buffer_size = 32
# optional (10). specifies the maximum amount of records combined into one message
batch_size = 10
```

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
