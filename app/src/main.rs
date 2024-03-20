use serenity::model::prelude::*;
use serenity::prelude::*;
use commands::*;

mod poise_command_builder;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN env var expected.");
    let intents = GatewayIntents::empty();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::slashies::get_commands(),
            on_error: |err| Box::pin(commands::slashies::error_handler(err)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let cmds = poise_command_builder::build_commands(&framework.options().commands);
                let res = ctx.http().create_global_commands(&cmds).await;
                if res.is_err() { println!("{res:?}"); }
                res?;

                println!("Started!");
                Ok(HBotData::default())
            })
        })
        .build();

    commands::time::mark_startup_time();

    let mut client = Client::builder(token, intents)
        .framework(framework)
        .await.unwrap();

    client.start().await.unwrap();
}
