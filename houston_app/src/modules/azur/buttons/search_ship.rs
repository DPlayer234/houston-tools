use azur_lane::ship::*;
use azur_lane::Faction;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::data::HAzurLane;
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub name: Option<String>,
    pub faction: Option<Faction>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<ShipRarity>,
    pub has_augment: Option<bool>
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    pub fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        iter: impl Iterator<Item = &'a ShipData>,
    ) -> CreateReply<'a> {
        let mut desc = String::new();
        let mut options = Vec::new();
        let mut has_next = false;

        for ship in iter {
            if options.len() >= PAGE_SIZE {
                has_next = true;
                break
            }

            let emoji = data.app_emojis().hull(ship.hull_type);

            writeln_str!(
                desc,
                "- {emoji} **{}** [{} {} {}]",
                ship.name, ship.rarity.name(), ship.faction.prefix().unwrap_or("Col."), ship.hull_type.designation(),
            );

            let view_ship = super::ship::View::new(ship.group_id).back(self.to_custom_data());
            options.push(CreateSelectMenuOption::new(&ship.name, view_ship.to_custom_id()).emoji(emoji.clone()));
        }

        let author = CreateEmbedAuthor::new("Ships")
            .url(config::azur_lane::SHIP_LIST_URL);

        if options.is_empty() {
            let embed = CreateEmbed::new()
                .author(author)
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            return CreateReply::new().embed(embed);
        }

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        let mut rows = Vec::new();
        if let Some(pagination) = ToPage::get_pagination_buttons(&mut self, utils::field_mut!(Self: page), has_next) {
            rows.push(pagination);
        }

        rows.push(super::create_string_select_menu_row(
            self.to_custom_id(),
            options,
            "View ship...",
        ));

        CreateReply::new().embed(embed).components(rows)
    }

    pub fn create(self, data: &HBotData) -> CreateReply<'_> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, filtered)
    }
}

impl ButtonMessage for View {
    fn edit_reply(mut self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>> {
        ToPage::load_page(&mut self.page, ctx.interaction);
        Ok(self.create(ctx.data).into())
    }
}

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a ShipData> + 'a> {
        match &self.name {
            Some(name) => self.apply_filter(data, data.ships_by_prefix(name.as_str())),
            None => self.apply_filter(data, data.ships().iter()),
        }
    }

    fn apply_filter<'a, I>(&self, data: &'a HAzurLane, iter: I) -> Box<dyn Iterator<Item = &'a ShipData> + 'a>
    where
        I: Iterator<Item = &'a ShipData> + 'a,
    {
        macro_rules! def_and_filter {
            ($fn_name:ident: $field:ident => $next:ident) => {
                fn $fn_name<'a>(
                    f: &Filter,
                    data: &'a HAzurLane,
                    iter: impl Iterator<Item = &'a ShipData> + 'a
                ) -> Box<dyn Iterator<Item = &'a ShipData> + 'a>
                {
                    match f.$field {
                        Some(filter) => $next(f, data, iter.filter(move |s| s.$field == filter)),
                        None => $next(f, data, iter)
                    }
                }
            }
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: hull_type => next_rarity);
        def_and_filter!(next_rarity: rarity => next_has_augment);

        fn next_has_augment<'a>(
            f: &Filter,
            data: &'a HAzurLane,
            iter: impl Iterator<Item = &'a ShipData> + 'a
        ) -> Box<dyn Iterator<Item = &'a ShipData> + 'a>
        {
            match f.has_augment {
                Some(filter) => Box::new(iter.filter(move |s| data.augments_by_ship_id(s.group_id).next().is_some() == filter)),
                None => Box::new(iter),
            }
        }

        next_faction(self, data, iter)
    }
}
