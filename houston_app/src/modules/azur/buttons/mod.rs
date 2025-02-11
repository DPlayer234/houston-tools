//! Some small guidelines for the API of buttons here:
//!
//! - Accept `LoadedConfig` instead of re-grabbing it manually from the bot data
//!   unless there is no benefit to it.
//! - Private methods should avoid doing duplicate work and be infallible if
//!   possible -- pass the Config through as an additional parameter if needed
//! - Builder-style methods >> [`Option`] parameter on `new` function

use azur_lane::ship::HullType;

use crate::buttons::prelude::*;

pub mod augment;
pub mod equip;
pub mod juustagram_chat;
pub mod lines;
pub mod search_augment;
pub mod search_equip;
pub mod search_juustagram_chat;
pub mod search_ship;
pub mod search_special_secretary;
pub mod shadow_equip;
pub mod ship;
pub mod skill;
pub mod special_secretary;

#[derive(Debug, thiserror::Error)]
enum AzurParseError {
    #[error("unknown ship")]
    Ship,
    #[error("unknown equip")]
    Equip,
    #[error("unknown augment")]
    Augment,
    #[error("unknown special secretary")]
    SpecialSecretary,
    #[error("unknown juustagram chat")]
    JuustagramChat,
}

fn get_ship_preview_name<'a>(ctx: &ButtonContext<'a>) -> Option<&'a str> {
    let embed = ctx.interaction.message.embeds.first()?;
    get_thumbnail_filename(embed)
}

fn get_thumbnail_filename(embed: &Embed) -> Option<&str> {
    let thumb = embed.thumbnail.as_ref()?;
    let (_, name) = thumb.url.rsplit_once('/')?;
    Some(name.split_once('.').map_or(name, |a| a.0))
}

pub fn hull_emoji(hull_type: HullType, data: &HBotData) -> &ReactionType {
    let e = data.app_emojis();
    match hull_type {
        HullType::Unknown => e.fallback(),
        HullType::Destroyer => e.hull_dd(),
        HullType::LightCruiser => e.hull_cl(),
        HullType::HeavyCruiser => e.hull_ca(),
        HullType::Battlecruiser => e.hull_bc(),
        HullType::Battleship => e.hull_bb(),
        HullType::LightCarrier => e.hull_cvl(),
        HullType::AircraftCarrier => e.hull_cv(),
        HullType::Submarine => e.hull_ss(),
        HullType::AviationBattleship => e.hull_bbv(),
        HullType::RepairShip => e.hull_ar(),
        HullType::Monitor => e.hull_bm(),
        HullType::AviationSubmarine => e.hull_ssv(),
        HullType::LargeCruiser => e.hull_cb(),
        HullType::MunitionShip => e.hull_ae(),
        HullType::MissileDestroyerV => e.hull_ddgv(),
        HullType::MissileDestroyerM => e.hull_ddgm(),
        HullType::FrigateS => e.hull_ixs(),
        HullType::FrigateV => e.hull_ixv(),
        HullType::FrigateM => e.hull_ixm(),
    }
}

macro_rules! pagination {
    ($obj:expr, $options:expr, $iter:expr, $label:expr) => {{
        if $options.is_empty() {
            return $crate::modules::azur::buttons::pagination_impl::no_results($obj.page);
        }

        $crate::modules::azur::buttons::pagination_impl::rows_setup(
            &mut $obj,
            $options.into(),
            $iter,
            $label.into(),
            |s| &mut s.page,
        )
    }};
}

pub(crate) use pagination;

mod pagination_impl {
    use crate::buttons::prelude::*;
    use crate::helper::discord::create_string_select_menu_row;
    use crate::modules::core::buttons::ToPage;

    const PAGE_SIZE: usize = 15;

    pub fn no_results<'new>(page: u16) -> Result<CreateReply<'new>> {
        if page == 0 {
            let embed = CreateEmbed::new()
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            Ok(CreateReply::new().embed(embed))
        } else {
            Err(HArgError::new("This page has no data.").into())
        }
    }

    pub fn rows_setup<'a, T, I, F>(
        obj: &mut T,
        options: Cow<'a, [CreateSelectMenuOption<'a>]>,
        iter: I,
        label: Cow<'a, str>,
        page: F,
    ) -> Vec<CreateActionRow<'a>>
    where
        T: ToCustomData,
        I: Iterator,
        F: Fn(&mut T) -> &mut u16,
    {
        let mut rows = Vec::new();

        #[allow(clippy::cast_possible_truncation)]
        let page_count = 1 + *page(obj) + iter.count().div_ceil(PAGE_SIZE) as u16;
        let pagination = ToPage::build_row(obj, page).exact_page_count(page_count);

        if let Some(pagination) = pagination.end() {
            rows.push(pagination);
        }

        rows.push(create_string_select_menu_row(
            obj.to_custom_id(),
            options,
            label,
        ));
        rows
    }
}
