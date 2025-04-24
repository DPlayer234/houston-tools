use azur_lane::Faction;
use azur_lane::equip::EquipKind;
use azur_lane::ship::HullType;

use super::buttons;
use crate::slashies::prelude::*;

mod autocomplete;
mod choices;
mod find;

use choices::{Ch, EAugmentRarity, EEquipRarity, EShipRarity};

/// Information about mobile game Azur Lane.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub mod azur {
    /// Shows information about a ship.
    #[sub_command]
    async fn ship(
        ctx: Context<'_>,
        /// The ship's name. This supports auto completion.
        #[autocomplete = "autocomplete::ship_name"]
        name: &str,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        defer_unloaded(ctx, ephemeral).await?;

        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let ship = find::ship(azur.game_data(), name)?;

        let view = buttons::ship::View::new(ship.group_id);
        ctx.send(
            view.create_with_ship(data, azur, ship, None)
                .ephemeral(ephemeral.into_ephemeral()),
        )
        .await?;
        Ok(())
    }

    /// Shows information about equipment.
    #[sub_command]
    async fn equip(
        ctx: Context<'_>,
        /// The equipment name. This supports auto completion.
        #[autocomplete = "autocomplete::equip_name"]
        name: &str,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        defer_unloaded(ctx, ephemeral).await?;

        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let equip = find::equip(azur.game_data(), name)?;

        let view = buttons::equip::View::new(equip.equip_id);
        ctx.send(
            view.create_with_equip(equip)
                .ephemeral(ephemeral.into_ephemeral()),
        )
        .await?;
        Ok(())
    }

    /// Shows information about an augment module.
    #[sub_command]
    async fn augment(
        ctx: Context<'_>,
        /// The equipment name. This supports auto completion.
        #[autocomplete = "autocomplete::augment_name"]
        name: &str,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        defer_unloaded(ctx, ephemeral).await?;

        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let augment = find::augment(azur.game_data(), name)?;

        let view = buttons::augment::View::new(augment.augment_id);
        ctx.send(
            view.create_with_augment(azur, augment)
                .ephemeral(ephemeral.into_ephemeral()),
        )
        .await?;
        Ok(())
    }

    /// Shows lines for a special secretary.
    #[sub_command(name = "special-secretary")]
    async fn special_secretary(
        ctx: Context<'_>,
        /// The equipment name. This supports auto completion.
        #[autocomplete = "autocomplete::special_secretary_name"]
        name: &str,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        defer_unloaded(ctx, ephemeral).await?;

        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let secretary = find::special_secretary(azur.game_data(), name)?;

        let view = buttons::special_secretary::View::new(secretary.id);
        ctx.send(
            view.create_with_sectary(data, secretary)
                .ephemeral(ephemeral.into_ephemeral()),
        )
        .await?;
        Ok(())
    }

    /// View Juustagram chats.
    #[sub_command(name = "juustagram-chat")]
    async fn juustagram_chat(
        ctx: Context<'_>,
        /// The ship's name. This supports auto completion.
        #[autocomplete = "autocomplete::ship_name_juustagram_chats"]
        ship: Option<&str>,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        use buttons::search_juustagram_chat::*;

        defer_unloaded(ctx, ephemeral).await?;

        let data = ctx.data_ref();
        let azur = data.config().azur()?;

        let filter = Filter {
            ship: match ship {
                Some(ship) => Some(find::ship(azur.game_data(), ship)?.group_id),
                None => None,
            },
        };

        let view = View::new(filter);
        ctx.send(view.create(data)?.ephemeral(ephemeral.into_ephemeral()))
            .await?;

        Ok(())
    }

    /// Calculates the actual reload time for a weapon.
    #[sub_command(name = "reload-time")]
    async fn reload_time(
        ctx: Context<'_>,
        /// The ship's RLD stat.
        #[min = 1]
        #[max = 999]
        rld: f64,
        /// The weapon's base FR in seconds.
        #[min = 0.0]
        #[max = 60.0]
        #[name = "weapon-fr"]
        weapon_reload: f64,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        let reload_time = (200.0 / (100.0 + rld)).sqrt() * weapon_reload;

        let description = format!(
            "-# **Base Weapon FR:** {weapon_reload:.2}s \u{2E31} **`RLD:`**`{rld: >4}`\n\
            **Final FR:** {reload_time:.2}s"
        );

        let embed = CreateEmbed::new()
            .color(ctx.data_ref().config().embed_color)
            .description(description);

        ctx.send(create_reply(ephemeral).embed(embed)).await?;
        Ok(())
    }

    /// Search for information.
    #[sub_command]
    mod search {
        /// Searches for ships.
        #[sub_command]
        async fn ship(
            ctx: Context<'_>,
            /// A name to search for.
            name: Option<&str>,
            /// The faction to select.
            #[autocomplete = "choices::faction"]
            faction: Option<Ch<Faction>>,
            /// The hull type to select.
            #[name = "hull-type"]
            #[autocomplete = "choices::hull_type"]
            hull_type: Option<Ch<HullType>>,
            /// The rarity to select.
            rarity: Option<EShipRarity>,
            /// Whether the ships have a unique augment.
            #[name = "has-augment"]
            has_augment: Option<bool>,
            /// Whether to show the response only to yourself.
            ephemeral: Option<bool>,
        ) -> Result {
            use buttons::search_ship::*;

            defer_unloaded(ctx, ephemeral).await?;
            let data = ctx.data_ref();

            let filter = Filter {
                name,
                faction: faction.map(Ch::into_inner),
                hull_type: hull_type.map(Ch::into_inner),
                rarity: rarity.map(EShipRarity::convert),
                has_augment,
            };

            let view = View::new(filter);
            ctx.send(view.create(data)?.ephemeral(ephemeral.into_ephemeral()))
                .await?;

            Ok(())
        }

        /// Searches for equipment.
        #[sub_command]
        async fn equip(
            ctx: Context<'_>,
            /// A name to search for.
            name: Option<&str>,
            /// The faction to select.
            #[autocomplete = "choices::faction"]
            faction: Option<Ch<Faction>>,
            /// The kind to select.
            #[autocomplete = "choices::equip_kind"]
            kind: Option<Ch<EquipKind>>,
            /// The rarity to select.
            rarity: Option<EEquipRarity>,
            /// Whether to show the response only to yourself.
            ephemeral: Option<bool>,
        ) -> Result {
            use buttons::search_equip::*;

            defer_unloaded(ctx, ephemeral).await?;
            let data = ctx.data_ref();

            let filter = Filter {
                name,
                faction: faction.map(Ch::into_inner),
                kind: kind.map(Ch::into_inner),
                rarity: rarity.map(EEquipRarity::convert),
            };

            let view = View::new(filter);
            ctx.send(view.create(data)?.ephemeral(ephemeral.into_ephemeral()))
                .await?;

            Ok(())
        }

        /// Searches for augment modules.
        #[sub_command]
        async fn augment(
            ctx: Context<'_>,
            /// A name to search for.
            name: Option<&str>,
            /// The allowed hull type.
            #[name = "hull-type"]
            #[autocomplete = "choices::hull_type"]
            hull_type: Option<Ch<HullType>>,
            /// The rarity to select.
            rarity: Option<EAugmentRarity>,
            /// The name of the ship it is uniquely for.
            #[autocomplete = "autocomplete::ship_name"]
            #[name = "for-ship"]
            for_ship: Option<&str>,
            /// Whether to show the response only to yourself.
            ephemeral: Option<bool>,
        ) -> Result {
            use buttons::search_augment::*;

            defer_unloaded(ctx, ephemeral).await?;

            let data = ctx.data_ref();
            let azur = data.config().azur()?;

            let unique_ship_id = match for_ship {
                Some(for_ship) => Some(find::ship(azur.game_data(), for_ship)?.group_id),
                None => None,
            };

            let filter = Filter {
                name,
                hull_type: hull_type.map(Ch::into_inner),
                rarity: rarity.map(EAugmentRarity::convert),
                unique_ship_id,
            };

            let view = View::new(filter);
            ctx.send(view.create(data)?.ephemeral(ephemeral.into_ephemeral()))
                .await?;

            Ok(())
        }

        /// Searches for special secretaries.
        #[sub_command(name = "special-secretary")]
        async fn special_secretary(
            ctx: Context<'_>,
            /// A name to search for.
            name: Option<&str>,
            /// Whether to show the response only to yourself.
            ephemeral: Option<bool>,
        ) -> Result {
            use buttons::search_special_secretary::*;

            defer_unloaded(ctx, ephemeral).await?;
            let data = ctx.data_ref();

            let filter = Filter { name };

            let view = View::new(filter);
            ctx.send(view.create(data)?.ephemeral(ephemeral.into_ephemeral()))
                .await?;

            Ok(())
        }
    }
}

async fn defer_unloaded(ctx: Context<'_>, ephemeral: Option<bool>) -> Result {
    let data = ctx.data_ref();
    let azur = data.config().azur_raw()?;
    if azur.needs_load() {
        ctx.defer_as(ephemeral).await?;
    }
    Ok(())
}
