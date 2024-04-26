//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::ship::*;

use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode, PartialEq, Eq)]
pub struct ViewShip {
    pub ship_id: u32,
    pub affinity: ViewAffinity,
    pub retrofit: Option<usize>
}

#[derive(Debug, Clone, Copy, Default, bitcode::Encode, bitcode::Decode, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViewAffinity {
    #[default] Default,
    Love,
    Oath
}

impl From<ViewShip> for ButtonArgs {
    fn from(value: ViewShip) -> Self {
        ButtonArgs::ViewShip(value)
    }
}

impl ViewShip {
    pub fn new_with_ship_id(ship_id: u32) -> Self {
        Self { ship_id, affinity: ViewAffinity::Default, retrofit: None }
    }

    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let rarity = ship.rarity.data();
        let hull_type = ship.hull_type.data();

        let description = format!(
            "{:★<star_pad$} {}\n\
            {}\n\
            {} ({})",
            '★', rarity.name,
            ship.faction.data().name,
            hull_type.name, hull_type.designation,
            star_pad = usize::from(ship.stars)
        );

        let stats = match self.affinity {
            ViewAffinity::Default => ship.stats.clone(),
            ViewAffinity::Love => ship.stats.multiply(1.06),
            ViewAffinity::Oath => ship.stats.multiply(1.12),
        };

        let embed = CreateEmbed::new()
            .title(ship.name.as_ref())
            .description(description)
            .color(rarity.color_rgb)
            .fields(if hull_type.team_type != TeamType::Submarine { get_surface_stats(stats) } else { get_submarine_stats(stats) });

        let mut rows = vec![
            CreateActionRow::Buttons(vec![
                self.button_with_affinity(ViewAffinity::Default)
                    .emoji('💙'),
                self.button_with_affinity(ViewAffinity::Love)
                    .emoji('❤'),
                self.button_with_affinity(ViewAffinity::Oath)
                    .emoji('💗'),
            ])
        ];

        let base_button = self.button_with_retrofit(None)
            .label("Base")
            .style(ButtonStyle::Secondary);

        match base_ship.unwrap_or(ship).retrofits.len() {
            0 => {},
            1 => {
                rows.push(CreateActionRow::Buttons(vec![
                    base_button,
                    self.button_with_retrofit(Some(0))
                        .label("Retrofit")
                        .style(ButtonStyle::Secondary)
                ]));
            },
            _ => {
                rows.push(CreateActionRow::Buttons(
                    Some(base_button)
                        .into_iter()
                        .chain(ship.retrofits.iter().enumerate()
                            .map(|(index, retro)| {
                                self.button_with_retrofit(Some(index))
                                    .label(format!("Retrofit ({})", retro.hull_type.data().team_type.data().name))
                                    .style(ButtonStyle::Secondary)
                            }))
                        .collect()
                ));
            }
        };

        create.embed(embed).components(rows)
    }

    fn button_with_affinity(&self, affinity: ViewAffinity) -> CreateButton {
        self.new_button(utils::field!(Self: affinity), affinity, || Sentinel::new(0, affinity as usize))
    }

    fn button_with_retrofit(&self, retrofit: Option<usize>) -> CreateButton {
        self.new_button(utils::field!(Self: retrofit), retrofit, || Sentinel::new(1, retrofit.unwrap_or(usize::MAX)))
    }
}

fn get_surface_stats(stats: ShipStats) -> [(&'static str, String, bool); 3] {
    fn f(n: f32) -> u32 { n.floor() as u32 }
    [
        ("Stats", format!("**`HP:`**`{: >5}`\n**`FP:`**`{: >5}`\n**`AA:`**`{: >5}`\n**`ASW:`**`{: >4}`\n**`LCK:`**`{: >4}`", f(stats.hp), f(stats.fp), f(stats.aa), f(stats.asw), f(stats.lck)), true),
        ("\u{200b}", format!("**`{}`**\n**`TRP:`**`{: >4}`\n**`AVI:`**`{: >4}`\n**`SPD:`**`{: >4}`\n**`Cost:`**`{: >3}`", stats.armor.data().name, f(stats.trp), f(stats.avi), f(stats.spd), stats.cost), true),
        ("\u{200b}", format!("**`RLD:`**`{: >4}`\n**`EVA:`**`{: >4}`\n**`ACC:`**`{: >4}`", f(stats.rld), f(stats.eva), f(stats.acc)), true),
    ]
}

fn get_submarine_stats(stats: ShipStats) -> [(&'static str, String, bool); 3] {
    fn f(n: f32) -> u32 { n.floor() as u32 }
    [
        ("Stats", format!("**`HP:`**`{: >5}`\n**`FP:`**`{: >5}`\n**`AA:`**`{: >5}`\n**`OXY:`**`{: >4}`\n**`LCK:`**`{: >4}`", f(stats.hp), f(stats.fp), f(stats.aa), stats.oxy, f(stats.lck)), true),
        ("\u{200b}", format!("**`{}`**\n**`TRP:`**`{: >4}`\n**`AVI:`**`{: >4}`\n**`AMO:`**`{: >4}`\n**`Cost:`**`{: >3}`", stats.armor.data().name, f(stats.trp), f(stats.avi), stats.amo, stats.cost), true),
        ("\u{200b}", format!("**`RLD:`**`{: >4}`\n**`EVA:`**`{: >4}`\n**`ACC:`**`{: >4}`\n**`SPD:`**`{: >4}`", f(stats.rld), f(stats.eva), f(stats.acc), f(stats.spd)), true),
    ]
}

impl ButtonArgsModify for ViewShip {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let ship = data.azur_lane.ship_by_id(self.ship_id).ok_or(ShipParseError)?;
        Ok(match self.retrofit.and_then(|index| ship.retrofits.get(index)) {
            None => self.modify_with_ship(create, ship, None),
            Some(retrofit) => self.modify_with_ship(create, retrofit, Some(ship))
        })
    }
}
