use azur_lane::equip::*;
use azur_lane::Faction;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::data::HAzurLane;
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

    pub fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
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

        super::pagination!(rows => self, options, iter);

        let author =
            CreateEmbedAuthor::new("Equipments").url(config::azur_lane::EQUIPMENT_LIST_URL);

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        rows.push(super::create_string_select_menu_row(
            self.to_custom_id(),
            options,
            "View equipment...",
        ));

        Ok(CreateReply::new().embed(embed).components(rows))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let filtered = self
            .filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, filtered)
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
