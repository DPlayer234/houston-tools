use std::mem::swap;
use std::ptr;

use serenity::prelude::*;

use crate::modules::{azur, core as core_mod, minigame, perks, starboard};
use crate::prelude::*;

mod context;
mod encoding;
#[cfg(test)]
mod test;

pub use context::{AnyContext, AnyInteraction, ButtonContext, ModalContext};

pub mod prelude {
    pub use bson_model::ModelDocument as _;

    pub use super::{ButtonArgsReply, ButtonContext, CustomData, ModalContext, ToCustomData};
    pub use crate::prelude::*;
}

/// Helper macro that repeats needed code for every [`ButtonArgs`] variant.
macro_rules! define_button_args {
    ($($(#[$attr:meta])* $name:ident($Ty:ty)),* $(,)?) => {
        /// The supported button interaction arguments.
        ///
        /// This is owned data that can be deserialized into.
        #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
        enum ButtonArgs {
            $(
                $(#[$attr])*
                $name($Ty),
            )*
        }

        /// The supported button interaction arguments.
        ///
        /// This is borrowed data that can be serialized.
        #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
        enum ButtonArgsRef<'a> {
            $(
                $(#[$attr])*
                $name(&'a $Ty),
            )*
        }

        $(
            impl<'a> From<&'a $Ty> for ButtonArgsRef<'a> {
                fn from(value: &'a $Ty) -> Self {
                    Self::$name(value)
                }
            }
        )*

        impl<'a> From<&'a ButtonArgs> for ButtonArgsRef<'a> {
            /// Borrows the inner data.
            fn from(value: &'a ButtonArgs) -> Self {
                match value {
                    $(
                        ButtonArgs::$name(v) => Self::$name(v),
                    )*
                }
            }
        }

        impl ButtonArgs {
            async fn reply(self, ctx: ButtonContext<'_>) -> Result {
                match self {
                    $(
                        Self::$name(args) => args.reply(ctx).await,
                    )*
                }
            }

            async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
                match self {
                    $(
                        Self::$name(args) => args.modal_reply(ctx).await,
                    )*
                }
            }
        }
    };
}

// to avoid unexpected effects for old buttons, don't insert new variants
// anywhere other than the bottom and don't reorder them!
define_button_args! {
    /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
    Noop(core_mod::buttons::Noop),
    /// Open the ship detail view.
    AzurShip(azur::buttons::ship::View),
    /// Open the augment detail view.
    AzurAugment(azur::buttons::augment::View),
    /// Open the skill detail view.
    AzurSkill(azur::buttons::skill::View),
    /// Open the ship lines detail view.
    AzurLines(azur::buttons::lines::View),
    /// Open the ship filter list view.
    AzurSearchShip(azur::buttons::search_ship::View),
    /// Open the ship shadow equip details.
    AzurShadowEquip(azur::buttons::shadow_equip::View),
    /// Open the equipment details.
    AzurEquip(azur::buttons::equip::View),
    /// Open the equipment search.
    AzurSearchEquip(azur::buttons::search_equip::View),
    /// Open the augment search.
    AzurSearchAugment(azur::buttons::search_augment::View),
    /// Open the perk store.
    PerksStore(perks::buttons::shop::View),
    /// Open the starboard top view.
    StarboardTop(starboard::buttons::top::View),
    /// Open the starboard top posts view.
    StarboardTopPosts(starboard::buttons::top_posts::View),
    /// Open the "go to page" modal.
    ToPage(core_mod::buttons::ToPage),
    /// Delete the source message.
    Delete(core_mod::buttons::Delete),
    /// Sets the birthday for the perks module.
    PerksBirthdaySet(perks::buttons::birthday::Set),
    /// Open a Juustagram chat.
    AzurJuustagramChat(azur::buttons::juustagram_chat::View),
    /// Open the Juustagram chat search.
    AzurSearchJuustagramChat(azur::buttons::search_juustagram_chat::View),
    /// Play the next tic-tac-toe turn.
    MinigameTicTacToe(minigame::buttons::tic_tac_toe::View),
    /// Choose your action for rock-paper-scissors.
    MinigameRockPaperScissors(minigame::buttons::rock_paper_scissors::View),
    /// Play the next "chess" turn.
    MinigameChess(minigame::buttons::chess::View),
    /// Open the special secretary view.
    AzurSpecialSecretary(azur::buttons::special_secretary::View),
    /// Open the special secretary search.
    AzurSearchSpecialSecretary(azur::buttons::search_special_secretary::View),
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    pub fn from_custom_id(id: &str) -> Result<Self> {
        encoding::from_custom_id(id)
    }
}

/// Event handler for custom button menus.
pub struct EventHandler;

crate::modules::impl_handler!(EventHandler, |_, ctx| match _ {
    FullEvent::InteractionCreate {
        interaction: Interaction::Component(interaction),
        ..
    } => handler::dispatch_component(ctx, interaction),
    FullEvent::InteractionCreate {
        interaction: Interaction::Modal(interaction),
        ..
    } => handler::dispatch_modal(ctx, interaction),
});

/// Event handler for custom button menus.
mod handler {
    use std::sync::atomic::AtomicBool;

    use super::*;
    use crate::fmt::discord::interaction_location;

    /// Dispatches component interactions.
    pub async fn dispatch_component(ctx: &Context, interaction: &ComponentInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = handle_component(ctx, interaction, &reply_state).await {
            handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            )
            .await
        }
    }

    /// Handles the component interaction dispatch.
    async fn handle_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        use ComponentInteractionDataKind as Kind;

        let custom_id: &str = match &interaction.data.kind {
            Kind::StringSelect { values } if values.len() == 1 => &values[0],
            Kind::Button => &interaction.data.custom_id,
            _ => anyhow::bail!("Invalid interaction."),
        };

        let args = ButtonArgs::from_custom_id(custom_id)?;
        log::info!(
            "[Button] {}, {}: {:?}",
            interaction_location(interaction.guild_id, interaction.channel.as_ref()),
            interaction.user.name,
            args
        );

        args.reply(ButtonContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        })
        .await
    }

    /// Dispatches modal interactions.
    pub async fn dispatch_modal(ctx: &Context, interaction: &ModalInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = handle_modal(ctx, interaction, &reply_state).await {
            handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            )
            .await
        }
    }

    /// Handles the modal interaction dispatch.
    async fn handle_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        let args = ButtonArgs::from_custom_id(&interaction.data.custom_id)?;
        log::info!(
            "[Modal] {}, {}: {:?}",
            interaction_location(interaction.guild_id, interaction.channel.as_ref()),
            interaction.user.name,
            args
        );

        args.modal_reply(ModalContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        })
        .await
    }

    #[cold]
    async fn handle_dispatch_error(
        ctx: &Context,
        interaction_id: InteractionId,
        interaction_token: &str,
        reply_state: bool,
        err: anyhow::Error,
    ) {
        if let Some(ser_err) = err.downcast_ref::<serenity::Error>() {
            // print both errors to preserve the stack trace, if present
            log::warn!("Discord interaction error: {ser_err:?} / {err:?}");
            return;
        }

        let err_text = match err.downcast::<HArgError>() {
            Ok(err) => err.msg,
            Err(err) => {
                log::warn!("Component error: {err:?}");
                format!("Button error: ```{err}```").into()
            },
        };

        let embed = CreateEmbed::new()
            .description(err_text)
            .color(ERROR_EMBED_COLOR);

        let reply = CreateReply::new().ephemeral(true).embed(embed);

        let res = if reply_state {
            let response = reply.into_interaction_followup();
            response
                .execute(&ctx.http, None, interaction_token)
                .await
                .map(|_| ())
        } else {
            let response = reply.into_interaction_response();
            let response = CreateInteractionResponse::Message(response);
            response
                .execute(&ctx.http, interaction_id, interaction_token)
                .await
        };

        if let Err(res) = res {
            log::warn!("Error sending component error: {res}");
        }
    }
}

/// Provides a way to convert an object into a component custom ID.
///
/// This is auto-implemented for every type held by [`ButtonArgs`].
pub trait ToCustomData {
    /// Converts this instance to a component custom ID.
    #[must_use]
    fn to_custom_id(&self) -> String;

    /// Converts this instance to custom data.
    #[must_use]
    fn to_custom_data(&self) -> CustomData;

    /// Creates a new button that would switch to a state where one field is
    /// changed.
    ///
    /// If the field value is the same, instead returns a disabled button with
    /// the sentinel value.
    fn new_button<'a, T, F, S>(&mut self, field: F, value: T, sentinel: S) -> CreateButton<'a>
    where
        T: PartialEq,
        F: Fn(&mut Self) -> &mut T,
        S: FnOnce(T) -> u16,
    {
        let field_ref = field(self);
        let disabled = *field_ref == value;
        if disabled {
            // This value is intended to be unique for a given object.
            // It isn't used in any way other than as a discriminator.
            let sentinel_key = ptr::from_ref(field_ref) as u16;

            let sentinel = core_mod::buttons::Noop::new(sentinel_key, sentinel(value));
            let custom_id = sentinel.to_custom_id();
            CreateButton::new(custom_id).disabled(true)
        } else {
            let custom_id = self.to_custom_id_with(field, value);
            CreateButton::new(custom_id)
        }
    }

    /// Creates a new select option that would switch to a state where one field
    /// is changed.
    fn new_select_option<'a, T, F>(
        &mut self,
        label: impl Into<Cow<'a, str>>,
        field: F,
        value: T,
    ) -> CreateSelectMenuOption<'a>
    where
        T: PartialEq,
        F: Fn(&mut Self) -> &mut T,
    {
        let default = *field(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id).default_selection(default)
    }

    /// Creates a custom ID with one field replaced.
    #[must_use]
    fn to_custom_id_with<T, F>(&mut self, field: F, mut value: T) -> String
    where
        F: Fn(&mut Self) -> &mut T,
    {
        // Swap new value into the field
        swap(field(self), &mut value);
        // Create the custom ID
        let custom_id = self.to_custom_id();
        // Move original value back into field, dropping the new value.
        *field(self) = value;

        custom_id
    }
}

impl<T> ToCustomData for T
where
    for<'a> &'a T: Into<ButtonArgsRef<'a>>,
{
    fn to_custom_id(&self) -> String {
        encoding::to_custom_id(self.into())
    }

    fn to_custom_data(&self) -> CustomData {
        CustomData::from_button_args(self.into())
    }
}

/// Provides a way for button arguments to reply to the interaction.
pub trait ButtonArgsReply: Sized + Send {
    /// Replies to the component interaction.
    async fn reply(self, ctx: ButtonContext<'_>) -> Result;

    /// Replies to the modal interaction.
    async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
        _ = ctx;
        anyhow::bail!("this button args type does not support modals");
    }
}

/// Represents custom data for another menu.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomData(encoding::Buf);

impl CustomData {
    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        encoding::encode_custom_id(&self.0)
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    fn from_button_args(args: ButtonArgsRef<'_>) -> Self {
        let mut buf = encoding::Buf::new();
        encoding::write_button_args(&mut buf, args);
        Self(buf)
    }
}

#[cfg(test)]
impl CustomData {
    /// Gets an empty value.
    pub const EMPTY: Self = Self(encoding::Buf::new_const());

    /// Converts this instance to [`ButtonArgs`].
    fn to_button_args(&self) -> Result<ButtonArgs> {
        encoding::read_button_args(&self.0)
    }
}

/// Compile-time helper to assert that types are [`Send`] as expected.
///
/// Only done so we get errors at an early point rather than a sporadic "future
/// is not send" elsewhere.
fn _assert_traits() {
    fn ok<T: Send>(_v: T) {}
    fn dummy<T>() -> T {
        unreachable!()
    }

    ok(dummy::<ButtonArgs>());
    ok(dummy::<ButtonArgs>().reply(dummy()));
    ok(dummy::<ButtonArgs>().modal_reply(dummy()));
    ok(dummy::<ButtonArgsRef<'_>>());
    ok(dummy::<CustomData>());

    ok(dummy::<ButtonContext<'_>>());
    ok(dummy::<ButtonContext<'_>>().acknowledge());
    ok(dummy::<ButtonContext<'_>>().defer_as(true));
    ok(dummy::<ButtonContext<'_>>().reply(dummy()));
    ok(dummy::<ButtonContext<'_>>().edit(dummy()));

    ok(dummy::<ModalContext<'_>>());
    ok(dummy::<ModalContext<'_>>().acknowledge());
    ok(dummy::<ModalContext<'_>>().defer_as(true));
    ok(dummy::<ModalContext<'_>>().reply(dummy()));
    ok(dummy::<ModalContext<'_>>().edit(dummy()));
}
