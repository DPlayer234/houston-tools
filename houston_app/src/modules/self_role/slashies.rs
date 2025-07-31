use super::config::{RoleEntry, RoleGroup};
use crate::buttons::prelude::*;
use crate::fmt::Join;
use crate::helper::contains_ignore_ascii_case;
use crate::slashies::prelude::*;

/// Add or remove a free role.
#[chat_command(name = "self-role", contexts = "Guild", integration_types = "Guild")]
pub async fn self_role(
    ctx: Context<'_>,
    /// The role to add/remove.
    #[autocomplete = "autocomplete_role"]
    role: u64,
) -> Result {
    let data = ctx.data_ref();
    let member = ctx.member().context("requires guild")?;
    let (group, role) =
        find_role_group(ctx, role).ok_or(HArgError::new_const("Unknown claimable role."))?;

    ctx.defer(true).await?;

    let description = if member.roles.contains(&role.id) {
        // if you already have the role, unconditionally remove it
        member
            .remove_role(ctx.http(), role.id, Some("removed via /self-role"))
            .await?;
        format!("Removed {}.", role.id.mention())
    } else {
        // if there is a limit, make sure the user isn't already at/above the limit
        check_role_group_limit(member, group)?;

        member
            .add_role(ctx.http(), role.id, Some("claimed via /self-role"))
            .await?;
        format!("Added {}.", role.id.mention())
    };

    let embed = CreateEmbed::new()
        .description(description)
        .color(data.config().embed_color);

    let reply = CreateReply::new()
        .embed(embed)
        .allowed_mentions(CreateAllowedMentions::new());

    ctx.send(reply).await?;
    Ok(())
}

/// Returns [`Err`] if the member cannot claim more group roles.
fn check_role_group_limit(member: &Member, group: &RoleGroup) -> Result {
    let Some(limit) = group.limit else {
        return Ok(());
    };

    let owned_in_group = member
        .roles
        .iter()
        .filter(|&&id| group.roles.iter().any(move |role| role.id == id))
        .count();

    if usize::from(limit.get()) <= owned_in_group {
        let roles = Join::AND.display_as(&group.roles, |r| r.id.mention());
        let message = format!("May only have **{limit}** of {roles}.");
        Err(HArgError::new(message).into())
    } else {
        Ok(())
    }
}

fn find_role_group(ctx: Context<'_>, mut index: u64) -> Option<(&RoleGroup, &RoleEntry)> {
    let guild_id = ctx.guild_id()?;
    let config = ctx.data_ref().config().self_role.get(&guild_id)?;

    for group in &config.groups {
        let len = u64::from(group.roles.len());
        if index < len {
            let index = u8::try_from(index).ok()?;
            return Some((group, &group.roles[index]));
        }

        index -= len;
    }

    None
}

async fn autocomplete_role<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    // get the config for this guild, return empty if none
    if let Some(guild_id) = ctx.guild_id()
        && let Some(guild_config) = ctx.data_ref().config().self_role.get(&guild_id)
    {
        // flatten the role groups and assign indices to them
        let choices: Vec<_> = (0u64..)
            .zip(guild_config.groups.iter().flat_map(|g| &g.roles))
            .filter(|(_, r)| contains_ignore_ascii_case(&r.name, partial))
            .map(|(index, r)| AutocompleteChoice::new(&r.name, AutocompleteValue::Integer(index)))
            .collect();

        CreateAutocompleteResponse::new().set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
