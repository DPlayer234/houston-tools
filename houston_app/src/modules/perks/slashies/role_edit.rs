use std::str::FromStr;

use crate::modules::perks::items::Item;
use crate::modules::perks::model::*;
use crate::slashies::prelude::*;

// Note: The description is set by the loading code.
/// Edit your unique role.
#[chat_command(name = "role-edit", contexts = "Guild", integration_types = "Guild")]
pub async fn role_edit(
    ctx: Context<'_>,
    /// The new role name.
    #[min_length = 2]
    #[max_length = 100]
    name: &str,
    /// The new role color as an RGB hex code.
    #[min_length = 6]
    #[max_length = 6]
    color: Option<HexColor>,
) -> Result {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    ctx.defer_as(Ephemeral).await?;

    let filter = UniqueRole::filter()
        .guild(guild_id)
        .user(ctx.user().id)
        .into_document()?;

    let unique = UniqueRole::collection(db)
        .find_one(filter)
        .await?
        .ok_or(HArgError::new_const("You don't have a unique role."))?;

    let mut edit = EditRole::new()
        .name(name)
        .audit_log_reason("use of role-edit command");

    if let Some(HexColor(color)) = color {
        edit = edit.colour(color);
    }

    Wallet::collection(db)
        .take_items(guild_id, ctx.user().id, &[(Item::RoleEdit, 1)], perks)
        .await?;

    match guild_id.edit_role(ctx.http(), unique.role, edit).await {
        Ok(role) => {
            let description = format!(
                "Your role is now: {}\n-# Used 1 {}.",
                role.mention(),
                Item::RoleEdit.info(perks).name,
            );

            let embed = CreateEmbed::new()
                .color(data.config().embed_color)
                .description(description);

            ctx.send(CreateReply::new().embed(embed)).await?;
        },
        Err(_) => {
            Wallet::collection(db)
                .add_items(guild_id, ctx.user().id, &[(Item::RoleEdit, 1)])
                .await?;

            let embed = CreateEmbed::new()
                .color(ERROR_EMBED_COLOR)
                .description("Can't edit the role.");

            ctx.send(CreateReply::new().embed(embed)).await?;
        },
    }

    Ok(())
}

struct HexColor(Color);

#[derive(Debug, thiserror::Error)]
#[error("The color is an invalid hex code.")]
struct NotHex;

impl FromStr for HexColor {
    type Err = NotHex;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str_radix(s, 16)
            .map(|u| Self(Color::new(u)))
            .map_err(|_| NotHex)
    }
}

houston_cmd::impl_slash_arg_via_from_str!(HexColor);
