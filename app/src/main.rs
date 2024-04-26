use std::num::NonZeroU16;
use std::sync::Arc;
use serenity::model::prelude::*;
use serenity::prelude::*;
use commands::*;

mod poise_command_builder;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN env var expected.");
    let intents = GatewayIntents::empty();

    unsafe { utils::time::mark_startup_time(); }

    println!("Starting...");

    let azur_lane_data = {
        let data_path = std::env::var("AZUR_LANE_DATA").unwrap_or_else(|_| "houston_azur_lane_data.json".to_owned());
        let f = std::fs::File::open(data_path).expect("Failed to read Azur Lane data.");
        serde_json::from_reader(f).expect("Failed to parse Azur Lane data.")
    };

    println!("Loaded Azur Lane data.");

    let bot_data = Arc::new(HBotData::new(azur_lane_data));

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: slashies::get_commands(),
            pre_command: |ctx| Box::pin(slashies::pre_command(ctx)),
            on_error: |err| Box::pin(slashies::error_handler(err)),
            ..Default::default()
        })
        .setup({
            let bot_data = Arc::clone(&bot_data);
            move |ctx, ready, framework| {
                Box::pin(async move {
                    create_commands(ctx, framework).await?;
                    print_ready(ready);
                    Ok(bot_data)
                })
            }
        })
        .build();

    let mut client = Client::builder(token, intents)
        .framework(framework)
        .event_handler(buttons::ButtonEventHandler::new(bot_data))
        .await.unwrap();

    client.start().await.unwrap();
}

async fn create_commands(ctx: &Context, framework: &poise::framework::Framework<Arc<HBotData>, HError>) -> HResult {
    let cmds = poise_command_builder::build_commands(&framework.options().commands);
    let res = ctx.http().create_global_commands(&cmds).await;
    if res.is_err() { println!("{res:?}"); }
    res?;

    Ok(())
}

fn print_ready(ready: &Ready) {
    let discriminator = ready.user.discriminator.map_or(0u16, NonZeroU16::get);
    println!("Started! Logged in as: {}#{:04}", ready.user.name, discriminator);
}
