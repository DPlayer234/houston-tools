use azur_lane::equip::*;
use azur_lane::Faction;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::data::HAzurLane;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub name: Option<String>,
    pub faction: Option<Faction>,
    pub kind: Option<EquipKind>,
    pub rarity: Option<EquipRarity>,
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(
        mut self,
        iter: impl Iterator<Item = &'a Equip>,
    ) -> CreateReply<'a> {
        let mut desc = String::new();
        let mut options = Vec::new();
        let mut has_next = false;

        for equip in iter {
            if options.len() >= PAGE_SIZE {
                has_next = true;
                break
            }

            writeln_str!(
                desc,
                "- **{}** [{} {} {}]",
                equip.name, equip.rarity.name(), equip.faction.prefix().unwrap_or("Col."), equip.kind.name(),
            );

            let view_equip = super::equip::View::new(equip.equip_id).new_message();
            options.push(CreateSelectMenuOption::new(&equip.name, view_equip.to_custom_id()));
        }

        let author = CreateEmbedAuthor::new("Equipments")
            .url(config::azur_lane::EQUIPMENT_LIST_URL);

        if options.is_empty() {
            let embed = CreateEmbed::new()
                .author(author)
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            return CreateReply::new().embed(embed);
        }

        let embed = CreateEmbed::new()
            .author(author)
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let mut rows = Vec::new();
        if let Some(pagination) = super::get_pagination_buttons(&mut self, utils::field_mut!(Self: page), has_next) {
            rows.push(pagination);
        }

        rows.push(super::create_string_select_menu_row(
            self.to_custom_id(),
            options,
            "View equipment...",
        ));

        CreateReply::new().embed(embed).components(rows)
    }

    pub fn modify(self, data: &HBotData) -> CreateReply<'_> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.modify_with_iter(filtered)
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        Ok(self.modify(ctx.data))
    }
}

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a Equip> + 'a> {
        match &self.name {
            Some(name) => self.apply_filter(data.equips_by_prefix(name.as_str())),
            None => self.apply_filter(data.equips().iter()),
        }
    }

    fn apply_filter<'a, I>(&self, iter: I) -> Box<dyn Iterator<Item = &'a Equip> + 'a>
    where
        I: Iterator<Item = &'a Equip> + 'a,
    {
        macro_rules! def_and_filter {
            ($fn_name:ident: $field:ident => $next:ident) => {
                fn $fn_name<'a>(
                    f: &Filter,
                    iter: impl Iterator<Item = &'a Equip> + 'a
                ) -> Box<dyn Iterator<Item = &'a Equip> + 'a>
                {
                    match f.$field {
                        Some(filter) => $next(f, iter.filter(move |s| s.$field == filter)),
                        None => $next(f, iter)
                    }
                }
            }
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: kind => next_rarity);
        def_and_filter!(next_rarity: rarity => finish);

        fn finish<'a>(
            _f: &Filter,
            iter: impl Iterator<Item = &'a Equip> + 'a
        ) -> Box<dyn Iterator<Item = &'a Equip> + 'a>
        {
            Box::new(iter)
        }

        next_faction(self, iter)
    }
}