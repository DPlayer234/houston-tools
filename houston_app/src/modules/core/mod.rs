use super::prelude::*;

pub mod buttons;
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, _config: &HBotConfig) -> bool {
        true
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [
            slashies::bot_stats::bot_stats(),
            slashies::coin::coin(),
            slashies::dice::dice(),
            slashies::calc::calc(),
            slashies::quote::quote(),
            slashies::timestamp::timestamp(),
            slashies::who::who(),
            slashies::who::who_context(),
            slashies::upload::upload(),
        ]
    }

    fn buttons(&self, _config: &HBotConfig) -> impl IntoIterator<Item = ButtonAction> {
        [
            buttons::Delete::action(),
            buttons::Noop::action(),
            buttons::ToPage::action(),
        ]
    }

    fn event_handler(self) -> Option<Box<dyn EventHandler>> {
        Some(Box::new(self))
    }
}

super::impl_handler!(Module, |_, ctx| match _ {
    FullEvent::Ready { data_about_bot, .. } => ready(ctx, data_about_bot),
});

async fn ready(ctx: &Context, ready: &Ready) {
    use std::num::NonZero;

    let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
    log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

    let data = ctx.data_ref::<HContextData>();
    if let Err(why) = data.ready(&ctx.http).await {
        log::error!("Failure in ready: {why:?}");
    }
}
