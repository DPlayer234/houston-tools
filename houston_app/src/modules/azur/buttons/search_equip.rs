use azur_lane::equip::*;
use azur_lane::Faction;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::{Config, GameData};
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter,
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

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        config: &'a Config,
        mut iter: impl Iterator<Item = &'a Equip>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for equip in iter.by_ref().take(PAGE_SIZE) {
            writeln_str!(
                desc,
                "- **{}** [{} {} {}]",
                equip.name,
                equip.rarity.name(),
                equip.faction.prefix().unwrap_or("Col."),
                equip.kind.name(),
            );

            let view_equip = super::equip::View::new(equip.equip_id).back(self.to_custom_data());
            options.push(CreateSelectMenuOption::new(
                &equip.name,
                view_equip.to_custom_id(),
            ));
        }

        let rows = super::pagination!(self, options, iter, "View equipment...");

        let author = CreateEmbedAuthor::new("Equipments").url(&*config.wiki_urls.equipment_list);

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        Ok(CreateReply::new().embed(embed).components(rows))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let config = data.config().azur()?;
        let filtered = self
            .filter
            .iterate(config.game_data())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, config, filtered)
    }
}

impl ButtonMessage for View {
    fn edit_reply(self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>> {
        self.create(ctx.data).map(EditReply::from)
    }

    fn edit_modal_reply(mut self, ctx: ModalContext<'_>) -> Result<EditReply<'_>> {
        ToPage::set_page_from(&mut self.page, ctx.interaction);
        self.create(ctx.data).map(EditReply::from)
    }
}

impl Filter {
    fn iterate<'a>(&self, azur: &'a GameData) -> Box<dyn Iterator<Item = &'a Equip> + 'a> {
        match &self.name {
            Some(name) => self.apply_filter(azur.equips_by_prefix(name.as_str())),
            None => self.apply_filter(azur.equips().iter()),
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
                    iter: impl Iterator<Item = &'a Equip> + 'a,
                ) -> Box<dyn Iterator<Item = &'a Equip> + 'a> {
                    match f.$field {
                        Some(filter) => $next(f, iter.filter(move |s| s.$field == filter)),
                        None => $next(f, iter),
                    }
                }
            };
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: kind => next_rarity);
        def_and_filter!(next_rarity: rarity => finish);

        fn finish<'a>(
            _f: &Filter,
            iter: impl Iterator<Item = &'a Equip> + 'a,
        ) -> Box<dyn Iterator<Item = &'a Equip> + 'a> {
            Box::new(iter)
        }

        next_faction(self, iter)
    }
}
