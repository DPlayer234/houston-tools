use azur_lane::Faction;
use azur_lane::equip::*;

use crate::buttons::*;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct View {
    page: u16,
    filter: Filter
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct Filter {
    pub name: Option<String>,
    pub faction: Option<Faction>,
    pub kind: Option<EquipKind>,
    pub rarity: Option<EquipRarity>,
}

impl From<View> for ButtonArgs {
    fn from(value: View) -> Self {
        ButtonArgs::ViewSearchEquip(value)
    }
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        View { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(self, create: CreateReply, iter: impl Iterator<Item = &'a Equip>) -> CreateReply {
        let mut desc = String::new();
        let mut options = Vec::new();
        let mut has_next = false;

        for equip in iter {
            if options.len() >= PAGE_SIZE {
                has_next = true;
                break
            }

            desc.push_str("- ");
            desc.push_str(&equip.name);
            desc.push('\n');

            let view_equip = super::equip::View::new(equip.equip_id);
            options.push(CreateSelectMenuOption::new(&equip.name, view_equip.to_custom_id()));
        }

        let embed = CreateEmbed::new()
            .title("Equipments")
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let options = CreateSelectMenuKind::String { options };
        let mut rows = vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new(self.clone().to_custom_id(), options).placeholder("View equipment..."))
        ];

        if self.page > 0 || has_next {
            rows.insert(0, CreateActionRow::Buttons(vec![
                if self.page > 0 {
                    self.new_button(utils::field!(Self: page), self.page - 1, || Sentinel::new(0, 1))
                } else {
                    CreateButton::new("#no-back").disabled(true)
                }.emoji('◀'),

                if has_next {
                    self.new_button(utils::field!(Self: page), self.page + 1, || Sentinel::new(0, 2))
                } else {
                    CreateButton::new("#no-forward").disabled(true)
                }.emoji('▶')
            ]));
        }

        create.embed(embed).components(rows)
    }
}

impl ButtonArgsModify for View {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        Ok(self.modify_with_iter(create, filtered))
    }
}

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a Equip> + 'a> {
        let predicate = self.predicate(data);
        match self.name {
            Some(ref name) => Box::new(data.equips_by_prefix(name.as_str()).filter(predicate)),
            None => Box::new(data.equip_list.iter().filter(predicate))
        }
    }

    fn predicate<'a>(&self, data: &'a HAzurLane) -> Box<dyn FnMut(&&Equip) -> bool + 'a> {
        macro_rules! def_and_filter {
            ($fn_name:ident: $field:ident => $next:ident) => {
                fn $fn_name<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&Equip) -> bool + 'a) -> Box<dyn FnMut(&&Equip) -> bool + 'a> {
                    match f.$field {
                        Some(filter) => $next(f, data, move |s| base(s) && s.$field == filter),
                        None => $next(f, data, base)
                    }
                }
            }
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: kind => next_rarity);
        def_and_filter!(next_rarity: rarity => finish);

        fn finish<'a>(_f: &Filter, _data: &'a HAzurLane, base: impl FnMut(&&Equip) -> bool + 'a) -> Box<dyn FnMut(&&Equip) -> bool + 'a> {
            Box::new(base)
        }

        next_faction(self, data, |_| true)
    }
}
