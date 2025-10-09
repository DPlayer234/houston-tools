use std::{fmt, iter};

use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::helper::discord::unicode_emoji;
use crate::modules::azur::LoadedConfig;
use crate::modules::azur::config::WikiUrls;

/// View general ship details.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    ship_id: u32,
    #[builder(default = 120)]
    level: u8,
    #[builder(default = ViewAffinity::Love)]
    affinity: ViewAffinity,
    #[builder(default = None)]
    retrofit: Option<u8>,
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    back: Option<Nav<'v>>,
}

/// The affinity used to calculate stat values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ViewAffinity {
    Neutral,
    Love,
    Oath,
}

impl View<'_> {
    /// Modifies the create-reply with preresolved ship data.
    pub fn create_with_ship<'a>(
        self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
    ) -> CreateReply<'a> {
        let mut create = CreateReply::new();

        let thumbail_key = if let Some(skin) = ship.skin_by_id(ship.default_skin_id)
            && let Some(image_data) = azur.game_data().get_chibi_image(&skin.image_key)
        {
            let filename = format!("{}.webp", skin.image_key);
            create = create.attachment(CreateAttachment::bytes(image_data, filename));
            Some(skin.image_key.as_str())
        } else {
            None
        };

        create.components_v2(self.with_ship(data, azur, ship, ship, thumbail_key))
    }

    fn edit_with_ship<'a>(
        self,
        ctx: &ButtonContext<'a>,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: &'a ShipData,
    ) -> EditReply<'a> {
        let mut edit = EditReply::new();

        let thumbail_key = if let Some(skin) = base_ship.skin_by_id(ship.default_skin_id)
            && let Some(image_data) = azur.game_data().get_chibi_image(&skin.image_key)
        {
            if Some(skin.image_key.as_str()) != super::get_ship_preview_name(ctx) {
                edit = edit.new_attachment(CreateAttachment::bytes(
                    image_data,
                    format!("{}.webp", skin.image_key),
                ));
            }
            Some(skin.image_key.as_str())
        } else {
            edit = edit.clear_attachments();
            None
        };

        edit.components_v2(self.with_ship(ctx.data, azur, ship, base_ship, thumbail_key))
    }

    fn with_ship<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: &'a ShipData,
        thumbnail_key: Option<&str>,
    ) -> CreateComponents<'a> {
        let mut components = CreateComponents::new();

        components.push(self.get_header_field(data, ship, base_ship, thumbnail_key));
        components.extend(self.get_retro_state_row(base_ship));
        components.push(CreateSeparator::new(true));

        components.push(self.get_stats_field(ship));
        components.push(self.get_upgrade_row());
        components.push(CreateSeparator::new(true));

        components.push(self.get_equip_field(azur, ship));
        components.push(CreateSeparator::new(true));

        if let Some(skills_field) = self.get_skills_field(azur, ship) {
            components.push(skills_field);
            components.push(CreateSeparator::new(true));
        }

        if let Some(fleet_tech_field) = self.get_fleet_tech_field(data, base_ship) {
            components.push(fleet_tech_field);
            components.push(CreateSeparator::new(true));
        }

        components.push(self.get_nav_row(azur, base_ship));

        components![CreateContainer::new(components).accent_color(ship.rarity.color_rgb())]
    }

    fn get_nav_row<'a>(
        &self,
        azur: LoadedConfig<'a>,
        base_ship: &'a ShipData,
    ) -> CreateComponent<'a> {
        let view_lines = super::lines::View::builder()
            .ship_id(self.ship_id)
            .back(self.to_nav())
            .build();

        let lines_button = CreateButton::new(view_lines.to_custom_id())
            .label("Lines")
            .style(ButtonStyle::Secondary);

        let wiki_url = azur.wiki_urls().ship(base_ship);
        let wiki_button = CreateButton::new_link(wiki_url)
            .label("Wiki")
            .style(ButtonStyle::Secondary);

        let buttons = match &self.back {
            Some(back) => vec![
                CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
                    .label("Back"),
                lines_button,
                wiki_button,
            ],
            None => vec![lines_button, wiki_button],
        };

        CreateActionRow::buttons(buttons).into_component()
    }

    fn get_retro_state_row<'a>(&mut self, base_ship: &ShipData) -> Option<CreateComponent<'a>> {
        let base_button = || self.button_with_retrofit(None).label("Base");

        let buttons = match base_ship.retrofits.len() {
            0 => return None,
            1 => vec![
                { base_button }(),
                self.button_with_retrofit(Some(0)).label("Retrofit"),
            ],
            _ => iter::once({ base_button }())
                .chain(self.multi_retro_buttons(base_ship))
                .collect::<Vec<_>>(),
        };

        Some(CreateActionRow::buttons(buttons).into_component())
    }

    fn multi_retro_buttons<'a, 'b>(
        &'a mut self,
        base_ship: &'a ShipData,
    ) -> impl Iterator<Item = CreateButton<'b>> + 'a {
        (0..4u8).zip(&base_ship.retrofits).map(|(index, retro)| {
            // using team_type for the label since currently only DDGs have multiple
            // retro states and their main identifier is what fleet they go in
            self.button_with_retrofit(Some(index))
                .label(format!("Retrofit ({})", retro.hull_type.team_type().name()))
        })
    }

    fn get_header_field<'a>(
        &self,
        data: &'a HBotData,
        ship: &'a ShipData,
        base_ship: &'a ShipData,
        thumbnail_key: Option<&str>,
    ) -> CreateComponent<'a> {
        let content = format!(
            "## {}\n\
             [{}] {:★<star_pad$}\n{} {} {}",
            base_ship.name,
            ship.rarity.name(),
            '★',
            super::hull_emoji(ship.hull_type, data),
            ship.faction.name(),
            ship.hull_type.name(),
            star_pad = usize::from(ship.stars)
        );

        let content = CreateTextDisplay::new(content);

        if let Some(thumbnail_key) = thumbnail_key {
            let url = format!("attachment://{thumbnail_key}.webp");
            let media = CreateUnfurledMediaItem::new(url);
            let thumbnail = CreateThumbnail::new(media);

            CreateSection::new(
                section_components![content],
                CreateSectionAccessory::Thumbnail(thumbnail),
            )
            .into_component()
        } else {
            content.into_component()
        }
    }

    /// Creates the embed field that display the stats.
    fn get_stats_field<'a>(&self, ship: &ShipData) -> CreateComponent<'a> {
        let stats = &ship.stats;
        let level = u32::from(self.level);
        let affinity = self.affinity.to_mult();

        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        fn f(n: f64) -> u32 {
            n.floor() as u32
        }

        macro_rules! calc_all {
            ($($i:ident)*) => {
                $(
                    let $i = f(stats.$i.calc(level, affinity));
                )*
            };
        }

        calc_all!(hp rld fp trp eva aa avi acc);
        let spd = f(stats.spd);
        let lck = f(stats.lck);
        let cost = stats.cost;

        let armor_name = stats.armor.name();
        let armor_pad = 8usize.saturating_sub(stats.armor.name().len());

        let content = if ship.hull_type.team_type() != TeamType::Submarine {
            calc_all!(asw);
            format!(
                "### Stats\n\
                 **`HP:`**`{hp: >5}` \u{2E31} **`{armor_name}`**`{: <armor_pad$}` \u{2E31} **`RLD:`**`{rld: >4}`\n\
                 **`FP:`**`{fp: >5}` \u{2E31} **`TRP:`**`{trp: >4}` \u{2E31} **`EVA:`**`{eva: >4}`\n\
                 **`AA:`**`{aa: >5}` \u{2E31} **`AVI:`**`{avi: >4}` \u{2E31} **`ACC:`**`{acc: >4}`\n\
                 **`ASW:`**`{asw: >4}` \u{2E31} **`SPD:`**`{spd: >4}`\n\
                 **`LCK:`**`{lck: >4}` \u{2E31} **`Cost:`**`{cost: >3}`",
                "",
            )
        } else {
            let ShipStatBlock { oxy, amo, .. } = *stats;
            format!(
                "### Stats\n\
                 **`HP:`**`{hp: >5}` \u{2E31} **`{armor_name}`**`{: <armor_pad$}` \u{2E31} **`RLD:`**`{rld: >4}`\n\
                 **`FP:`**`{fp: >5}` \u{2E31} **`TRP:`**`{trp: >4}` \u{2E31} **`EVA:`**`{eva: >4}`\n\
                 **`AA:`**`{aa: >5}` \u{2E31} **`AVI:`**`{avi: >4}` \u{2E31} **`ACC:`**`{acc: >4}`\n\
                 **`OXY:`**`{oxy: >4}` \u{2E31} **`AMO:`**`{amo: >4}` \u{2E31} **`SPD:`**`{spd: >4}`\n\
                 **`LCK:`**`{lck: >4}` \u{2E31} **`Cost:`**`{cost: >3}`",
                "",
            )
        };

        CreateTextDisplay::new(content).into_component()
    }

    fn get_upgrade_row<'a>(&mut self) -> CreateComponent<'a> {
        CreateActionRow::buttons(vec![
            self.button_with_level(120).label("Lv.120"),
            self.button_with_level(125).label("Lv.125"),
            self.button_with_affinity(ViewAffinity::Love)
                .emoji(unicode_emoji("❤"))
                .label("100"),
            self.button_with_affinity(ViewAffinity::Oath)
                .emoji(unicode_emoji("💗"))
                .label("200"),
        ])
        .into_component()
    }

    /// Creates the embed field that displays the weapon equipment slots.
    fn get_equip_field<'a>(&self, azur: LoadedConfig<'a>, ship: &ShipData) -> CreateComponent<'a> {
        let slots = ship
            .equip_slots
            .iter()
            .filter_map(|e| e.mount.as_ref().map(|m| (&e.allowed, m)));

        let mut text = String::new();
        text.push_str("### Equipment");

        for (allowed, mount) in slots {
            if !text.is_empty() {
                text.push('\n');
            }

            let slots =
                Join::SLASH.display_as(allowed, |&kind| equip_slot_display(azur.wiki_urls(), kind));

            write!(text, "**`{: >3.0}%`**", mount.efficiency * 100f64);

            match (mount.mounts, mount.retriggers) {
                (m, 0) => write!(text, "`x{m}`"),
                (1, r) => write!(text, "`x{}`", r + 1),
                (m, r) => write!(text, "`x{}x{m}`", r + 1),
            }

            write!(text, " {slots}");

            if mount.preload != 0 {
                write!(text, " `PRE x{}`", mount.preload);
            }

            if mount.parallel > 1 {
                text.push_str(" `PAR`");
            }
        }

        for mount in &ship.shadow_equip {
            if !text.is_empty() {
                text.push('\n');
            }

            write!(
                text,
                "-# **`{: >3.0}%`** {}",
                mount.efficiency * 100f64,
                mount.name
            );
        }

        for equip in &ship.depth_charges {
            if !text.is_empty() {
                text.push('\n');
            }

            write!(text, "-# **`ASW:`** {}", equip.name);
        }

        let text = CreateTextDisplay::new(text);

        if ship.shadow_equip.is_empty() && ship.depth_charges.is_empty() {
            text.into_component()
        } else {
            let view = super::shadow_equip::View::new(self.clone());
            let button = CreateButton::new(view.to_custom_id())
                .label("Shadow Equip")
                .style(ButtonStyle::Secondary);

            CreateSection::new(
                section_components![text],
                CreateSectionAccessory::Button(button),
            )
            .into_component()
        }
    }

    /// Creates the embed field that displays the skill summary.
    fn get_skills_field<'a>(
        &self,
        azur: LoadedConfig<'a>,
        ship: &ShipData,
    ) -> Option<CreateComponent<'a>> {
        if ship.skills.is_empty() {
            return None;
        }

        // There isn't any way a unique augment can do anything if there are no skills
        // so we still skip the field if there are no skills but there is an augment.
        // ... Not that there are any ships without skills to begin with.
        // CMBK: do we need this at all?
        let mut text = String::new();
        text.push_str("### Skills\n");

        for s in &ship.skills {
            writeln!(text, "{} **{}**", s.category.emoji(), s.name);
        }

        if let Some(bonus) = ship.ultimate_bonus {
            writeln!(text, "> {}", bonus.description());
        }

        let augments = azur.game_data().augments_by_ship_id(ship.group_id);
        for augment in augments {
            writeln!(text, "-# UA: **{}**", augment.name);
        }

        let button = {
            use super::skill::{View, ViewSource};

            let view_skill = View::builder()
                .source(ViewSource::ship(self.ship_id, self.retrofit))
                .back(self.to_nav())
                .build();

            CreateButton::new(view_skill.to_custom_id())
                .label("Info")
                .style(ButtonStyle::Secondary)
        };

        Some(
            CreateSection::new(
                section_components![CreateTextDisplay::new(text)],
                CreateSectionAccessory::Button(button),
            )
            .into_component(),
        )
    }

    /// Creates the embed field that displays the fleet tech info.
    fn get_fleet_tech_field<'a>(
        &self,
        data: &HBotData,
        base_ship: &ShipData,
    ) -> Option<CreateComponent<'a>> {
        let fleet_tech = base_ship.fleet_tech.as_ref()?;

        fn stat_display(data: &HBotData, stats: &FleetTechStatBonus) -> impl fmt::Display {
            utils::text::from_fn(|f| {
                let hulls =
                    Join::EMPTY.display_as(&stats.hull_types, |h| super::hull_emoji(*h, data));
                let stat = stats.stat.name();
                let amount = stats.amount;

                write!(f, "{hulls} **`{stat}`**`+{amount}`")
            })
        }

        let text = format!(
            "**Tech:** {} \u{2E31} {}",
            stat_display(data, &fleet_tech.stats_get),
            stat_display(data, &fleet_tech.stats_level),
        );

        Some(CreateTextDisplay::new(text).into_component())
    }

    /// Gets a button that redirects to a different level.
    fn button_with_level<'a>(&mut self, level: u8) -> CreateButton<'a> {
        self.new_button(|s| &mut s.level, level, u8::into)
    }

    /// Gets a button that redirects to a different affinity.
    fn button_with_affinity<'a>(&mut self, affinity: ViewAffinity) -> CreateButton<'a> {
        self.new_button(|s| &mut s.affinity, affinity, |u| u as u16)
    }

    /// Creates a button that redirects to a retrofit state.
    fn button_with_retrofit<'a>(&mut self, retrofit: Option<u8>) -> CreateButton<'a> {
        self.new_button(
            |s| &mut s.retrofit,
            retrofit,
            |u| u.map_or(u16::MAX, u16::from),
        )
    }

    pub fn find_ship<'a>(
        &self,
        azur: LoadedConfig<'a>,
    ) -> Result<(&'a ShipData, Option<&'a ShipData>)> {
        let ship = azur
            .game_data()
            .ship_by_id(self.ship_id)
            .ok_or(AzurParseError::Ship)?;

        let retrofit = self
            .retrofit
            .and_then(|index| ship.retrofits.get(usize::from(index)));

        Ok((ship, retrofit))
    }
}

button_value!(for<'v> View<'v>, 1);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;

        let (ship, retrofit) = self.find_ship(azur)?;
        let edit = match retrofit {
            None => self.edit_with_ship(&ctx, azur, ship, ship),
            Some(retrofit) => self.edit_with_ship(&ctx, azur, retrofit, ship),
        };

        ctx.edit(edit).await
    }
}

impl ViewAffinity {
    /// Converts the affinity to a stat multiplier.
    fn to_mult(self) -> f64 {
        match self {
            Self::Neutral => 1.0,
            Self::Love => 1.06,
            Self::Oath => 1.12,
        }
    }
}

struct Slot<'a> {
    label: &'a str,
    url: &'a str,
}

impl<'a> Slot<'a> {
    fn new(label: &'a str, url: &'a str) -> Self {
        Self { label, url }
    }
}

impl fmt::Display for Slot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label, url } = self;
        write!(f, "[{label}]({url})")
    }
}

/// Converts the equip slot to a masked link to the appropriate wiki page.
fn equip_slot_display(w: &WikiUrls, kind: EquipKind) -> Slot<'_> {
    match kind {
        EquipKind::Unknown => Slot::new("Unknown", &w.equipment_list),
        EquipKind::DestroyerGun => Slot::new("DD Gun", &w.dd_gun_list),
        EquipKind::LightCruiserGun => Slot::new("CL Gun", &w.cl_gun_list),
        EquipKind::HeavyCruiserGun => Slot::new("CA Gun", &w.ca_gun_list),
        EquipKind::LargeCruiserGun => Slot::new("CB Gun", &w.cb_gun_list),
        EquipKind::BattleshipGun => Slot::new("BB Gun", &w.bb_gun_list),
        EquipKind::SurfaceTorpedo => Slot::new("Torpedo", &w.surface_torpedo_list),
        EquipKind::SubmarineTorpedo => Slot::new("Torpedo", &w.sub_torpedo_list),
        EquipKind::AntiAirGun => Slot::new("AA Gun", &w.aa_gun_list),
        EquipKind::FuzeAntiAirGun => Slot::new("AA Gun (Fuze)", &w.fuze_aa_gun_list),
        EquipKind::Fighter => Slot::new("Fighter", &w.fighter_list),
        EquipKind::DiveBomber => Slot::new("Dive Bomber", &w.dive_bomber_list),
        EquipKind::TorpedoBomber => Slot::new("Torpedo Bomber", &w.torpedo_bomber_list),
        EquipKind::SeaPlane => Slot::new("Seaplane", &w.seaplane_list),
        EquipKind::AntiSubWeapon => Slot::new("ASW", &w.anti_sub_list),
        EquipKind::AntiSubAircraft => Slot::new("ASW Aircraft", &w.anti_sub_list),
        EquipKind::Helicopter => Slot::new("Helicopter", &w.auxiliary_list),
        EquipKind::Missile => Slot::new("Missile", &w.surface_torpedo_list),
        EquipKind::Cargo => Slot::new("Cargo", &w.cargo_list),
        EquipKind::Auxiliary => Slot::new("Auxiliary", &w.auxiliary_list),
    }
}
