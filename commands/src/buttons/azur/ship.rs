use std::fmt::Write;
//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::ship::*;
use azur_lane::equip::*;
use utils::{Discard, join};

use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
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
    pub fn with_ship_id(ship_id: u32) -> Self {
        Self { ship_id, level: 120, affinity: ViewAffinity::Love, retrofit: None }
    }

    pub fn modify_with_ship(self, data: &HBotData, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);
        let rarity = ship.rarity.data();
        let hull_type = ship.hull_type.data();

        let description = format!(
            "[{}] {:★<star_pad$}\n[{}] {} {}",
            rarity.name, '★', hull_type.designation, ship.faction.data().name, hull_type.name,
            star_pad = usize::from(ship.stars)
        );

        let embed = CreateEmbed::new()
            .author(super::get_ship_url(base_ship))
            .description(description)
            .color(rarity.color_rgb)
            .fields(self.get_stats_field(ship))
            .fields(self.get_equip_field(ship))
            .fields(self.get_skills_field(ship));

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

        {
            let self_custom_id = self.clone().to_custom_id();
            let skills = (ship.skills.len() != 0).then(|| {
                let source = super::skill::ViewSkillSource::Ship(self.ship_id, self.retrofit);
                let view_skill = super::skill::ViewSkill::with_back(source, self_custom_id.clone());
                CreateButton::new(view_skill.to_custom_id())
                    .label("Skills")
                    .style(ButtonStyle::Secondary)
            });

            let augment = data.azur_lane().augment_by_ship_id(ship.group_id).map(|augment| {
                let view_augment = super::augment::ViewAugment::with_back(augment.augment_id, self_custom_id.clone());
                CreateButton::new(view_augment.to_custom_id())
                    .label("Unique Augment")
                    .style(ButtonStyle::Secondary)
            });

            let lines = Some({
                let view_lines = super::lines::ViewLines::with_back(self.ship_id, self_custom_id.clone());
                CreateButton::new(view_lines.to_custom_id())
                    .label("Lines")
                    .style(ButtonStyle::Secondary)
            });

            let row: Vec<_> = skills.into_iter().chain(augment).chain(lines).collect();
            if !row.is_empty() {
                rows.push(CreateActionRow::Buttons(row));
            }
        }

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
                        .chain(
                            base_ship.retrofits.iter().enumerate()
                                .filter_map(|(index, retro)| {
                                    let index = u8::try_from(index).ok()?;
                                    let result = self.button_with_retrofit(Some(index))
                                        .label(format!("Retrofit ({})", retro.hull_type.data().team_type.data().name))
                                        .style(ButtonStyle::Secondary);
                                    Some(result)
                                })
                        )
                        .collect()
                ));
            }
        };

        create.embed(embed).components(rows)
    }

    fn button_with_level(&self, level: u8) -> CreateButton {
        self.new_button(utils::field!(Self: level), level, || Sentinel::new(0, u32::from(level)))
    }

    fn button_with_affinity(&self, affinity: ViewAffinity) -> CreateButton {
        self.new_button(utils::field!(Self: affinity), affinity, || Sentinel::new(1, affinity as u32))
    }

    fn button_with_retrofit(&self, retrofit: Option<u8>) -> CreateButton {
        self.new_button(utils::field!(Self: retrofit), retrofit, || Sentinel::new(2, retrofit.map(u32::from).unwrap_or(u32::MAX)))
    }

    fn get_stats_field(&self, ship: &ShipData) -> [SimpleEmbedFieldCreate; 1] {
        let stats = &ship.stats;
        let affinity = self.affinity.to_mult();
    
        fn f(n: f32) -> u32 { n.floor() as u32 }
        macro_rules! s {
            ($val:expr) => {{ f($val.calc(u32::from(self.level), affinity)) }};
        }
        
        if ship.hull_type.data().team_type != TeamType::Submarine {
            [(
                "Stats",
                format!(
                    "\
                    **`HP:`**`{: >5}` \u{2E31} **`{: <7}`**` ` \u{2E31} **`RLD:`**`{: >4}`\n\
                    **`FP:`**`{: >5}` \u{2E31} **`TRP:`**`{: >4}` \u{2E31} **`EVA:`**`{: >4}`\n\
                    **`AA:`**`{: >5}` \u{2E31} **`AVI:`**`{: >4}` \u{2E31} **`ACC:`**`{: >4}`\n\
                    **`ASW:`**`{: >4}` \u{2E31} **`SPD:`**`{: >4}`\n\
                    **`LCK:`**`{: >4}` \u{2E31} **`Cost:`**`{: >3}`
                    ",
                    s!(stats.hp), stats.armor.data().name, s!(stats.rld),
                    s!(stats.fp), s!(stats.trp), s!(stats.eva),
                    s!(stats.aa), s!(stats.avi), s!(stats.acc),
                    s!(stats.asw), f(stats.spd),
                    f(stats.lck), stats.cost
                ),
                false
            )]
        } else {
            [(
                "Stats",
                format!(
                    "\
                    **`HP:`**`{: >5}` \u{2E31} **`{: <7}`**` ` \u{2E31} **`RLD:`**`{: >4}`\n\
                    **`FP:`**`{: >5}` \u{2E31} **`TRP:`**`{: >4}` \u{2E31} **`EVA:`**`{: >4}`\n\
                    **`AA:`**`{: >5}` \u{2E31} **`AVI:`**`{: >4}` \u{2E31} **`ACC:`**`{: >4}`\n\
                    **`OXY:`**`{: >4}` \u{2E31} **`AMO:`**`{: >4}` \u{2E31} **`SPD:`**`{: >4}`\n\
                    **`LCK:`**`{: >4}` \u{2E31} **`Cost:`**`{: >3}`
                    ",
                    s!(stats.hp), stats.armor.data().name, s!(stats.rld),
                    s!(stats.fp), s!(stats.trp), s!(stats.eva),
                    s!(stats.aa), s!(stats.avi), s!(stats.acc),
                    stats.oxy, stats.amo, f(stats.spd),
                    f(stats.lck), stats.cost
                ),
                false
            )]
        }
    }

    fn get_equip_field(&self, ship: &ShipData) -> [SimpleEmbedFieldCreate; 1] {
        let slots = ship.equip_slots.iter()
            .filter_map(|e| e.mount.as_ref().map(|m| (&e.allowed, m)));

        let mut text = String::new();
        for (allowed, mount) in slots {
            if !text.is_empty() { text.push('\n'); }

            write!(text, "**`{: >3.0}%`**`x{}` ", mount.efficiency * 100f32, mount.mounts).discard();
            
            for (index, &kind) in allowed.iter().enumerate() {
                if index != 0 { text.push('/'); }
                text.push_str(to_equip_slot_display(kind));
            }

            if mount.preload != 0 {
                write!(text, " `PRE x{}`", mount.preload).discard();
            }

            if mount.parallel > 1 {
                text.push_str(" `PAR`");
            }
        }

        [("Equipment", text, true)]
    }

    fn get_skills_field(&self, ship: &ShipData) -> Option<SimpleEmbedFieldCreate> {
        match ship.skills.len() {
            0 => None,
            _ => {
                let mut text = String::new();
                for s in ship.skills.iter() {
                    if !text.is_empty() { text.push('\n'); }
                    write!(text, "{} **{}**", s.category.data().emoji, s.name).discard();
                }
                Some(("Skills", text, true))
            }
        }
    }
}

impl ButtonArgsModify for ViewShip {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let ship = data.azur_lane().ship_by_id(self.ship_id).ok_or(ShipParseError)?;
        Ok(match self.retrofit.and_then(|index| ship.retrofits.get(usize::from(index))) {
            None => self.modify_with_ship(data, create, ship, None),
            Some(retrofit) => self.modify_with_ship(data,create, retrofit, Some(ship))
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

fn to_equip_slot_display(kind: EquipKind) -> &'static str {
    match kind {
        EquipKind::DestroyerGun => join!("[DD](", config::equip::DD_GUN_LIST_URL, ")"),
        EquipKind::LightCruiserGun => join!("[CL](", config::equip::CL_GUN_LIST_URL, ")"),
        EquipKind::HeavyCruiserGun => join!("[CA](", config::equip::CA_GUN_LIST_URL, ")"),
        EquipKind::LargeCruiserGun => join!("[CB](", config::equip::CB_GUN_LIST_URL, ")"),
        EquipKind::BattleshipGun => join!("[BB](", config::equip::BB_GUN_LIST_URL, ")"),
        EquipKind::SurfaceTorpedo => join!("[Torpedo](", config::equip::SURFACE_TORPEDO_LIST_URL, ")"),
        EquipKind::SubmarineTorpedo => join!("[Torpedo](", config::equip::SUB_TORPEDO_LIST_URL, ")"),
        EquipKind::AntiAirGun => join!("[AA](", config::equip::AA_GUN_LIST_URL, ")"),
        EquipKind::FuzeAntiAirGun => join!("[AA (Fuze)](", config::equip::FUZE_AA_GUN_LIST_URL, ")"),
        EquipKind::Fighter => join!("[Fighter](", config::equip::FIGHTER_LIST_URL, ")"),
        EquipKind::DiveBomber => join!("[Dive Bomber](", config::equip::DIVE_BOMBER_LIST_URL, ")"),
        EquipKind::TorpedoBomber => join!("[Torpedo Bomber](", config::equip::TORPEDO_BOMBER_LIST_URL, ")"),
        EquipKind::SeaPlane => join!("[Seaplane](", config::equip::SEAPLANE_LIST_URL, ")"),
        EquipKind::AntiSubWeapon => join!("[ASW](", config::equip::ANTI_SUB_LIST_URL, ")"),
        EquipKind::AntiSubAircraft => join!("[ASW Aircraft](", config::equip::ANTI_SUB_LIST_URL, ")"),
        EquipKind::Helicopter => join!("[Helicopter](", config::equip::AUXILIARY_LIST_URL, ")"),
        EquipKind::Missile => join!("[Missile](", config::equip::SURFACE_TORPEDO_LIST_URL, ")"),
        EquipKind::Cargo => join!("[Cargo](", config::equip::CARGO_LIST_URL, ")"),
        EquipKind::Auxiliary => join!("[Auxiliary](", config::equip::AUXILIARY_LIST_URL, ")"),
    }
}
