use std::fmt::Write;

use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::{Discard, join};

use crate::buttons::*;
use super::ShipParseError;

/// View general ship details.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub ship_id: u32,
    pub level: u8,
    pub affinity: ViewAffinity,
    pub retrofit: Option<u8>,
    mode: ButtonMessageMode,
}

/// The affinity used to calculate stat values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ViewAffinity {
    Neutral,
    Love,
    Oath
}

impl View {
    /// Creates a new instance.
    pub fn new(ship_id: u32) -> Self {
        Self { ship_id, level: 120, affinity: ViewAffinity::Love, retrofit: None, mode: ButtonMessageMode::Edit }
    }

    /// Makes the button send a new message.
    pub fn new_message(mut self) -> Self {
        self.mode = ButtonMessageMode::New;
        self
    }

    /// Modifies the create-reply with preresolved ship data.
    pub fn modify_with_ship(mut self, data: &HBotData, mut create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        self.mode = ButtonMessageMode::Edit;
        let base_ship = base_ship.unwrap_or(ship);

        let description = format!(
            "[{}] {:★<star_pad$}\n{} {} {}",
            ship.rarity.name(), '★',
            data.app_emojis().hull(ship.hull_type), ship.faction.name(), ship.hull_type.name(),
            star_pad = usize::from(ship.stars)
        );

        let mut embed = CreateEmbed::new()
            .author(super::get_ship_wiki_url(base_ship))
            .description(description)
            .color(ship.rarity.color_rgb())
            .fields(self.get_stats_field(ship))
            .fields(self.get_equip_field(ship))
            .fields(self.get_skills_field(data, ship));

        let mut rows = Vec::new();
        self.add_upgrade_row(&mut rows);
        self.add_retro_state_row(base_ship, &mut rows);
        self.add_nav_row(ship, &mut rows);

        if let Some(skin) = base_ship.skin_by_id(ship.default_skin_id) {
            if let Some(image_data) = data.azur_lane().get_chibi_image(&skin.image_key) {
                create = create.attachment(CreateAttachment::bytes(image_data.as_ref(), format!("{}.webp", skin.image_key)));
                embed = embed.thumbnail(format!("attachment://{}.webp", skin.image_key));
            }
        }

        create.embed(embed).components(rows)
    }

    fn add_upgrade_row(&mut self, rows: &mut Vec<CreateActionRow>) {
        rows.push(
            CreateActionRow::Buttons(vec![
                self.button_with_level(120)
                    .label("Lv.120"),
                self.button_with_level(125)
                    .label("Lv.125"),
                self.button_with_affinity(ViewAffinity::Love)
                    .emoji('❤').label("100"),
                self.button_with_affinity(ViewAffinity::Oath)
                    .emoji('💗').label("200"),
            ])
        );
    }

    fn add_nav_row(&self, ship: &ShipData, rows: &mut Vec<CreateActionRow>) {
        let self_custom_data = self.to_custom_data();

        let mut row = Vec::new();

        if !ship.skills.is_empty() {
            use super::skill::{View, ShipViewSource};

            let source = ShipViewSource::new(self.ship_id, self.retrofit).into();
            let view_skill = View::with_back(source, self_custom_data.clone());
            let button = CreateButton::new(view_skill.to_custom_id())
                .label("Skills")
                .style(ButtonStyle::Secondary);

            row.push(button);
        }

        if !ship.shadow_equip.is_empty() || !ship.depth_charges.is_empty() {
            let view = super::shadow_equip::View::new(self.clone());
            let button = CreateButton::new(view.to_custom_id())
                .label("Shadow Equip")
                .style(ButtonStyle::Secondary);

            row.push(button);
        }

        {
            let view_lines = super::lines::View::with_back(self.ship_id, self_custom_data);
            let button = CreateButton::new(view_lines.to_custom_id())
                .label("Lines")
                .style(ButtonStyle::Secondary);

            row.push(button);
        }

        if !row.is_empty() {
            rows.push(CreateActionRow::Buttons(row));
        }
    }

    fn add_retro_state_row(&mut self, base_ship: &ShipData, rows: &mut Vec<CreateActionRow>) {
        let base_button = self.button_with_retrofit(None)
            .label("Base");

        match base_ship.retrofits.len() {
            0 => {},
            1 => rows.push(CreateActionRow::Buttons(vec![
                base_button,
                self.button_with_retrofit(Some(0))
                    .label("Retrofit")
            ])),
            _ => rows.push(CreateActionRow::Buttons(
                std::iter::once(base_button)
                    .chain(self.multi_retro_buttons(base_ship))
                    .collect()
            )),
        };
    }

    fn multi_retro_buttons<'a>(&'a mut self, base_ship: &'a ShipData) -> impl Iterator<Item = CreateButton> + 'a {
        base_ship.retrofits.iter()
            .enumerate()
            .filter_map(|(index, retro)| {
                let index = u8::try_from(index).ok()?;

                // using team_type for the label since currently only DDGs have multiple
                // retro states and their main identifier is what fleet they go in
                let result = self.button_with_retrofit(Some(index))
                    .label(format!("Retrofit ({})", retro.hull_type.team_type().name()));

                Some(result)
            })
    }

    /// Gets a button that redirects to a different level.
    fn button_with_level(&mut self, level: u8) -> CreateButton {
        self.new_button(utils::field_mut!(Self: level), level, u8::into)
    }

    /// Gets a button that redirects to a different affinity.
    fn button_with_affinity(&mut self, affinity: ViewAffinity) -> CreateButton {
        self.new_button(utils::field_mut!(Self: affinity), affinity, |u| u as u16)
    }

    /// Creates a button that redirects to a retrofit state.
    fn button_with_retrofit(&mut self, retrofit: Option<u8>) -> CreateButton {
        self.new_button(utils::field_mut!(Self: retrofit), retrofit, |u| u.map(u16::from).unwrap_or(u16::MAX))
    }

    /// Creates the embed field that display the stats.
    fn get_stats_field(&self, ship: &ShipData) -> [SimpleEmbedFieldCreate; 1] {
        let stats = &ship.stats;
        let affinity = self.affinity.to_mult();

        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        fn f(n: f64) -> u32 { n.floor() as u32 }
        macro_rules! s {
            ($val:expr) => {{ f($val.calc(u32::from(self.level), affinity)) }};
        }

        let content = if ship.hull_type.team_type() != TeamType::Submarine {
            format!(
                "**`HP:`**`{: >5}` \u{2E31} **`{: <7}`**` ` \u{2E31} **`RLD:`**`{: >4}`\n\
                 **`FP:`**`{: >5}` \u{2E31} **`TRP:`**`{: >4}` \u{2E31} **`EVA:`**`{: >4}`\n\
                 **`AA:`**`{: >5}` \u{2E31} **`AVI:`**`{: >4}` \u{2E31} **`ACC:`**`{: >4}`\n\
                 **`ASW:`**`{: >4}` \u{2E31} **`SPD:`**`{: >4}`\n\
                 **`LCK:`**`{: >4}` \u{2E31} **`Cost:`**`{: >3}`",
                s!(stats.hp), stats.armor.name(), s!(stats.rld),
                s!(stats.fp), s!(stats.trp), s!(stats.eva),
                s!(stats.aa), s!(stats.avi), s!(stats.acc),
                s!(stats.asw), f(stats.spd),
                f(stats.lck), stats.cost
            )
        } else {
            format!(
                "**`HP:`**`{: >5}` \u{2E31} **`{: <7}`**` ` \u{2E31} **`RLD:`**`{: >4}`\n\
                 **`FP:`**`{: >5}` \u{2E31} **`TRP:`**`{: >4}` \u{2E31} **`EVA:`**`{: >4}`\n\
                 **`AA:`**`{: >5}` \u{2E31} **`AVI:`**`{: >4}` \u{2E31} **`ACC:`**`{: >4}`\n\
                 **`OXY:`**`{: >4}` \u{2E31} **`AMO:`**`{: >4}` \u{2E31} **`SPD:`**`{: >4}`\n\
                 **`LCK:`**`{: >4}` \u{2E31} **`Cost:`**`{: >3}`",
                s!(stats.hp), stats.armor.name(), s!(stats.rld),
                s!(stats.fp), s!(stats.trp), s!(stats.eva),
                s!(stats.aa), s!(stats.avi), s!(stats.acc),
                stats.oxy, stats.amo, f(stats.spd),
                f(stats.lck), stats.cost
            )
        };

        [("Stats", content, false)]
    }

    /// Creates the embed field that displays the weapon equipment slots.
    fn get_equip_field(&self, ship: &ShipData) -> [SimpleEmbedFieldCreate; 1] {
        let slots = ship.equip_slots.iter()
            .filter_map(|e| e.mount.as_ref().map(|m| (&e.allowed, m)));

        let mut text = String::new();
        for (allowed, mount) in slots {
            if !text.is_empty() { text.push('\n'); }

            write!(text, "**`{: >3.0}%`**`x{}` ", mount.efficiency * 100f64, mount.mounts).discard();

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

        for mount in &ship.shadow_equip {
            if !text.is_empty() { text.push('\n'); }
            write!(text, "-# **`{: >3.0}%`** {}", mount.efficiency * 100f64, mount.name).discard();
        }

        for equip in &ship.depth_charges {
            if !text.is_empty() { text.push('\n'); }
            write!(text, "-# **`ASW:`** {}", equip.name).discard();
        }

        [("Equipment", text, false)]
    }

    /// Creates the embed field that display the skill summary.
    fn get_skills_field(&self, data: &HBotData, ship: &ShipData) -> Option<SimpleEmbedFieldCreate> {
        // There isn't any way a unique augment can do anything if there are no skills
        // so we still skip the field if there are no skills but there is an augment.
        // ... Not that there are any ships without skills to begin with.
        (!ship.skills.is_empty()).then(|| {
            let mut text = String::new();
            for s in &ship.skills {
                if !text.is_empty() { text.push('\n'); }
                write!(text, "{} **{}**", s.category.emoji(), s.name).discard();
            }

            let augments = data.azur_lane().augments_by_ship_id(ship.group_id);
            for augment in augments {
                if !text.is_empty() { text.push('\n'); }
                write!(text, "-# UA: **{}**", augment.name).discard();
            }

            ("Skills", text, false)
        })
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply> {
        let ship = ctx.data.azur_lane().ship_by_id(self.ship_id).ok_or(ShipParseError)?;
        Ok(match self.retrofit.and_then(|index| ship.retrofits.get(usize::from(index))) {
            None => self.modify_with_ship(ctx.data, ctx.create_reply(), ship, None),
            Some(retrofit) => self.modify_with_ship(ctx.data, ctx.create_reply(), retrofit, Some(ship))
        })
    }

    fn message_mode(&self) -> ButtonMessageMode {
        self.mode
    }
}

impl ViewAffinity {
    /// Converts the affinity to a stat multiplier.
    fn to_mult(self) -> f64 {
        match self {
            ViewAffinity::Neutral => 1.0,
            ViewAffinity::Love => 1.06,
            ViewAffinity::Oath => 1.12,
        }
    }
}

/// Converts the equip slot to a masked link to the appropriate wiki page.
fn to_equip_slot_display(kind: EquipKind) -> &'static str {
    use config::azur_lane::equip::*;

    match kind {
        EquipKind::DestroyerGun => join!("[DD Gun](", DD_GUN_LIST_URL, ")"),
        EquipKind::LightCruiserGun => join!("[CL Gun](", CL_GUN_LIST_URL, ")"),
        EquipKind::HeavyCruiserGun => join!("[CA Gun](", CA_GUN_LIST_URL, ")"),
        EquipKind::LargeCruiserGun => join!("[CB Gun](", CB_GUN_LIST_URL, ")"),
        EquipKind::BattleshipGun => join!("[BB Gun](", BB_GUN_LIST_URL, ")"),
        EquipKind::SurfaceTorpedo => join!("[Torpedo](", SURFACE_TORPEDO_LIST_URL, ")"),
        EquipKind::SubmarineTorpedo => join!("[Torpedo](", SUB_TORPEDO_LIST_URL, ")"),
        EquipKind::AntiAirGun => join!("[AA Gun](", AA_GUN_LIST_URL, ")"),
        EquipKind::FuzeAntiAirGun => join!("[AA Gun (Fuze)](", FUZE_AA_GUN_LIST_URL, ")"),
        EquipKind::Fighter => join!("[Fighter](", FIGHTER_LIST_URL, ")"),
        EquipKind::DiveBomber => join!("[Dive Bomber](", DIVE_BOMBER_LIST_URL, ")"),
        EquipKind::TorpedoBomber => join!("[Torpedo Bomber](", TORPEDO_BOMBER_LIST_URL, ")"),
        EquipKind::SeaPlane => join!("[Seaplane](", SEAPLANE_LIST_URL, ")"),
        EquipKind::AntiSubWeapon => join!("[ASW](", ANTI_SUB_LIST_URL, ")"),
        EquipKind::AntiSubAircraft => join!("[ASW Aircraft](", ANTI_SUB_LIST_URL, ")"),
        EquipKind::Helicopter => join!("[Helicopter](", AUXILIARY_LIST_URL, ")"),
        EquipKind::Missile => join!("[Missile](", SURFACE_TORPEDO_LIST_URL, ")"),
        EquipKind::Cargo => join!("[Cargo](", CARGO_LIST_URL, ")"),
        EquipKind::Auxiliary => join!("[Auxiliary](", AUXILIARY_LIST_URL, ")"),
    }
}
