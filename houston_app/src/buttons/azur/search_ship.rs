use azur_lane::Faction;
use azur_lane::ship::*;
use utils::text::write_str::*;

use crate::buttons::*;

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

    pub fn modify_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        create: CreateReply<'a>,
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

            let view_ship = super::ship::View::new(ship.group_id).new_message();
            options.push(CreateSelectMenuOption::new(&ship.name, view_ship.to_custom_id()).emoji(emoji.clone()));
        }

        if options.is_empty() {
            let embed = CreateEmbed::new()
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            return create.embed(embed);
        }

        let embed = CreateEmbed::new()
            .title("Ships")
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let mut rows = Vec::new();
        if let Some(pagination) = super::get_pagination_buttons(&mut self, utils::field_mut!(Self: page), has_next) {
            rows.push(pagination);
        }

        let options = CreateSelectMenuKind::String { options: options.into() };
        rows.push(CreateActionRow::SelectMenu(CreateSelectMenu::new(self.to_custom_id(), options).placeholder("View ship...")));

        create.embed(embed).components(rows)
    }

    pub fn modify<'a>(self, data: &'a HBotData, create: CreateReply<'a>) -> CreateReply<'a> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.modify_with_iter(data, create, filtered)
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        Ok(self.modify(ctx.data, ctx.create_reply()))
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
