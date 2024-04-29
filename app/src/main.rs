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

    // SAFETY: No other code running that accesses this yet.
    unsafe { utils::time::mark_startup_time(); }

    println!("Starting...");

    let start = std::time::Instant::now();

    let bot_data = Arc::new(HBotData::new(|| {
        let data_path = std::env::var("AZUR_LANE_DATA").unwrap_or_else(|_| "houston_azur_lane_data.json".to_owned());
        let f = std::fs::File::open(data_path).expect("Failed to read Azur Lane data.");
        simd_json::from_reader(f).expect("Failed to parse Azur Lane data.")
    }));

    let loader = tokio::task::spawn({
        let bot_data = Arc::clone(&bot_data);
        async move {
            println!("Loading Azur Lane data...");
            bot_data.force_init();
            println!("Loaded Azur Lane data. ({:.2?})", start.elapsed());
        }
    });

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
                    
                    let discriminator = ready.user.discriminator.map_or(0u16, NonZeroU16::get);
                    println!("Logged in as: {}#{:04} ({:.2?})", ready.user.name, discriminator, start.elapsed());

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
    loader.await.unwrap();
}

async fn create_commands(ctx: &Context, framework: &poise::framework::Framework<Arc<HBotData>, HError>) -> HResult {
    let cmds = poise_command_builder::build_commands(&framework.options().commands);
    let res = ctx.http().create_global_commands(&cmds).await;
    if res.is_err() { println!("{res:?}"); }
    res?;

    Ok(())
}
