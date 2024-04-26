//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::ship::*;

use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode, PartialEq, Eq)]
pub struct ViewShip {
    pub ship_id: u32,
    pub level: u8,
    pub affinity: ViewAffinity,
    pub retrofit: Option<u8>
}

#[derive(Debug, Clone, Copy, bitcode::Encode, bitcode::Decode, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViewAffinity {
    Neutral,
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
        Self { ship_id, level: 120, affinity: ViewAffinity::Love, retrofit: None }
    }

    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);
        let rarity = ship.rarity.data();
        let hull_type = ship.hull_type.data();

        let description = format!(
            "[{}] {:★<star_pad$}\n[{}] {} {}",
            rarity.name, '★', hull_type.designation, ship.faction.data().name, hull_type.name,
            star_pad = usize::from(ship.stars)
        );

        let embed = CreateEmbed::new()
            .title(base_ship.name.as_ref())
            .description(description)
            .color(rarity.color_rgb)
            .fields(get_stats(&self, ship));

        let mut rows = vec![
            CreateActionRow::Buttons(vec![
                self.button_with_level(100)
                    .label("Lv.100"),
                self.button_with_level(120)
                    .label("Lv.120"),
                self.button_with_level(125)
                    .label("Lv.125")
            ]),
            CreateActionRow::Buttons(vec![
                self.button_with_affinity(ViewAffinity::Neutral)
                    .emoji('💙').label("50"),
                self.button_with_affinity(ViewAffinity::Love)
                    .emoji('❤').label("100"),
                self.button_with_affinity(ViewAffinity::Oath)
                    .emoji('💗').label("200"),
            ])
        ];

        let base_button = self.button_with_retrofit(None)
            .label("Base")
            .style(ButtonStyle::Secondary);

        match base_ship.retrofits.len() {
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
                        .chain(base_ship.retrofits.iter().enumerate()
                            .filter_map(|(index, retro)| {
                                let index = u8::try_from(index).ok()?;
                                let result = self.button_with_retrofit(Some(index))
                                    .label(format!("Retrofit ({})", retro.hull_type.data().team_type.data().name))
                                    .style(ButtonStyle::Secondary);
                                Some(result)
                            }))
                        .collect()
                ));
            }
        };

        create.embed(embed).components(rows)
    }

    fn button_with_level(&self, level: u8) -> CreateButton {
        self.new_button(utils::field!(Self: level), level, || Sentinel::new(1, u32::from(level)))
    }

    fn button_with_affinity(&self, affinity: ViewAffinity) -> CreateButton {
        self.new_button(utils::field!(Self: affinity), affinity, || Sentinel::new(1, affinity as u32))
    }

    fn button_with_retrofit(&self, retrofit: Option<u8>) -> CreateButton {
        self.new_button(utils::field!(Self: retrofit), retrofit, || Sentinel::new(2, retrofit.map(u32::from).unwrap_or(u32::MAX)))
    }
}

fn get_stats(view: &ViewShip, ship: &ShipData) -> [(&'static str, String, bool); 3] {
    let stats = &ship.stats;
    let affinity = view.affinity.to_mult();

    fn f(n: f32) -> u32 { n.floor() as u32 }
    macro_rules! s {
        ($val:expr) => {{ f($val.calc(u32::from(view.level), affinity)) }};
    }

    if ship.hull_type.data().team_type != TeamType::Submarine {
        [
            ("Stats", format!("**`HP:`**`{: >5}`\n**`FP:`**`{: >5}`\n**`AA:`**`{: >5}`\n**`ASW:`**`{: >4}`\n**`LCK:`**`{: >4}`", s!(stats.hp), s!(stats.fp), s!(stats.aa), s!(stats.asw), f(stats.lck)), true),
            ("\u{200b}", format!("**`{}`**\n**`TRP:`**`{: >4}`\n**`AVI:`**`{: >4}`\n**`SPD:`**`{: >4}`\n**`Cost:`**`{: >3}`", stats.armor.data().name, s!(stats.trp), s!(stats.avi), f(stats.spd), stats.cost), true),
            ("\u{200b}", format!("**`RLD:`**`{: >4}`\n**`EVA:`**`{: >4}`\n**`ACC:`**`{: >4}`", s!(stats.rld), s!(stats.eva), s!(stats.acc)), true),
        ]
    } else {
        [
            ("Stats", format!("**`HP:`**`{: >5}`\n**`FP:`**`{: >5}`\n**`AA:`**`{: >5}`\n**`OXY:`**`{: >4}`\n**`LCK:`**`{: >4}`", s!(stats.hp), s!(stats.fp), s!(stats.aa), stats.oxy, f(stats.lck)), true),
            ("\u{200b}", format!("**`{}`**\n**`TRP:`**`{: >4}`\n**`AVI:`**`{: >4}`\n**`AMO:`**`{: >4}`\n**`Cost:`**`{: >3}`", stats.armor.data().name, s!(stats.trp), s!(stats.avi), stats.amo, stats.cost), true),
            ("\u{200b}", format!("**`RLD:`**`{: >4}`\n**`EVA:`**`{: >4}`\n**`ACC:`**`{: >4}`\n**`SPD:`**`{: >4}`", s!(stats.rld), s!(stats.eva), s!(stats.acc), f(stats.spd)), true),
        ]
    }
}

impl ButtonArgsModify for ViewShip {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let ship = data.azur_lane.ship_by_id(self.ship_id).ok_or(ShipParseError)?;
        Ok(match self.retrofit.and_then(|index| ship.retrofits.get(usize::from(index))) {
            None => self.modify_with_ship(create, ship, None),
            Some(retrofit) => self.modify_with_ship(create, retrofit, Some(ship))
        })
    }
}

impl ViewAffinity {
    fn to_mult(self) -> f32 {
        match self {
            ViewAffinity::Neutral => 1.0f32,
            ViewAffinity::Love => 1.06f32,
            ViewAffinity::Oath => 1.12f32,
        }
    }
}
