use std::{fmt, iter};

use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::helper::discord::unicode_emoji;
use crate::modules::azur::LoadedConfig;
use crate::modules::azur::config::WikiUrls;

/// View general ship details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub ship_id: u32,
    pub level: u8,
    pub affinity: ViewAffinity,
    pub retrofit: Option<u8>,
    #[serde(borrow)]
    pub back: Option<Nav<'v>>,
}

/// The affinity used to calculate stat values.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ViewAffinity {
    Neutral,
    Love,
    Oath,
}

impl<'v> View<'v> {
    /// Creates a new instance.
    pub fn new(ship_id: u32) -> Self {
        Self {
            ship_id,
            level: 120,
            affinity: ViewAffinity::Love,
            retrofit: None,
            back: None,
        }
    }

    /// Sets the back button target.
    pub fn back(mut self, back: Nav<'v>) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with preresolved ship data.
    pub fn create_with_ship<'a>(
        self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: Option<&'a ShipData>,
    ) -> CreateReply<'a> {
        let base_ship = base_ship.unwrap_or(ship);
        let (mut embed, rows) = self.with_ship(data, azur, ship, base_ship);

        let mut create = CreateReply::new();

        if let Some(skin) = base_ship.skin_by_id(ship.default_skin_id) {
            if let Some(image_data) = azur.game_data().get_chibi_image(&skin.image_key) {
                let filename = format!("{}.webp", skin.image_key);
                embed = embed.thumbnail(format!("attachment://{}", filename));
                create = create.attachment(CreateAttachment::bytes(image_data, filename));
            }
        }

        create.embed(embed).components(rows)
    }

    fn edit_with_ship<'a>(
        self,
        ctx: &ButtonContext<'a>,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: Option<&'a ShipData>,
    ) -> EditReply<'a> {
        let base_ship = base_ship.unwrap_or(ship);
        let (mut embed, rows) = self.with_ship(ctx.data, azur, ship, base_ship);
        let mut create = EditReply::new();

        // try expressions when
        let base_skin = || {
            let skin = base_ship.skin_by_id(ship.default_skin_id)?;
            let image = azur.game_data().get_chibi_image(&skin.image_key)?;
            Some((skin, image))
        };

        if let Some((skin, image_data)) = base_skin() {
            embed = embed.thumbnail(format!("attachment://{}.webp", skin.image_key));

            if Some(skin.image_key.as_str()) != super::get_ship_preview_name(ctx) {
                create = create.new_attachment(CreateAttachment::bytes(
                    image_data,
                    format!("{}.webp", skin.image_key),
                ));
            }
        } else {
            create = create.clear_attachments();
        }

        create.embed(embed).components(rows)
    }

    fn with_ship<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: &'a ShipData,
    ) -> (CreateEmbed<'a>, Vec<CreateActionRow<'a>>) {
        let description = format!(
            "[{}] {:★<star_pad$}\n{} {} {}",
            ship.rarity.name(),
            '★',
            super::hull_emoji(ship.hull_type, data),
            ship.faction.name(),
            ship.hull_type.name(),
            star_pad = usize::from(ship.stars)
        );

        let embed = CreateEmbed::new()
            .author(azur.wiki_urls().ship(base_ship))
            .description(description)
            .color(ship.rarity.color_rgb())
            .fields(self.get_stats_field(ship))
            .fields(self.get_equip_field(azur, ship))
            .fields(self.get_skills_field(azur, ship));

        let mut rows = Vec::new();
        self.add_upgrade_row(&mut rows);
        self.add_retro_state_row(base_ship, &mut rows);
        self.add_nav_row(ship, &mut rows);

        (embed, rows)
    }

    fn add_upgrade_row(&mut self, rows: &mut Vec<CreateActionRow<'_>>) {
        let mut row = vec![
            self.button_with_level(120).label("Lv.120"),
            self.button_with_level(125).label("Lv.125"),
            self.button_with_affinity(ViewAffinity::Love)
                .emoji(unicode_emoji("❤"))
                .label("100"),
            self.button_with_affinity(ViewAffinity::Oath)
                .emoji(unicode_emoji("💗"))
                .label("200"),
        ];

        if let Some(back) = &self.back {
            row.insert(
                0,
                CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
                    .label("Back"),
            );
        }

        rows.push(CreateActionRow::buttons(row));
    }

    fn add_nav_row(&self, ship: &ShipData, rows: &mut Vec<CreateActionRow<'_>>) {
        let mut row = Vec::new();

        if !ship.skills.is_empty() {
            use super::skill::{ShipViewSource, View};

            let source = ShipViewSource::new(self.ship_id, self.retrofit).into();
            let view_skill = View::with_back(source, self.to_nav());
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
            let view_lines = super::lines::View::with_back(self.ship_id, self.to_nav());
            let button = CreateButton::new(view_lines.to_custom_id())
                .label("Lines")
                .style(ButtonStyle::Secondary);

            row.push(button);
        }

        if !row.is_empty() {
            rows.push(CreateActionRow::buttons(row));
        }
    }

    fn add_retro_state_row(&mut self, base_ship: &ShipData, rows: &mut Vec<CreateActionRow<'_>>) {
        let base_button = self.button_with_retrofit(None).label("Base");

        match base_ship.retrofits.len() {
            0 => {},
            1 => rows.push(CreateActionRow::buttons(vec![
                base_button,
                self.button_with_retrofit(Some(0)).label("Retrofit"),
            ])),
            _ => rows.push(CreateActionRow::buttons(
                iter::once(base_button)
                    .chain(self.multi_retro_buttons(base_ship))
                    .collect::<Vec<_>>(),
            )),
        };
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

    /// Creates the embed field that display the stats.
    fn get_stats_field<'a>(&self, ship: &ShipData) -> [EmbedFieldCreate<'a>; 1] {
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
                "**`HP:`**`{hp: >5}` \u{2E31} **`{armor_name}`**`{: <armor_pad$}` \u{2E31} **`RLD:`**`{rld: >4}`\n\
                 **`FP:`**`{fp: >5}` \u{2E31} **`TRP:`**`{trp: >4}` \u{2E31} **`EVA:`**`{eva: >4}`\n\
                 **`AA:`**`{aa: >5}` \u{2E31} **`AVI:`**`{avi: >4}` \u{2E31} **`ACC:`**`{acc: >4}`\n\
                 **`ASW:`**`{asw: >4}` \u{2E31} **`SPD:`**`{spd: >4}`\n\
                 **`LCK:`**`{lck: >4}` \u{2E31} **`Cost:`**`{cost: >3}`",
                "",
            )
        } else {
            let ShipStatBlock { oxy, amo, .. } = *stats;
            format!(
                "**`HP:`**`{hp: >5}` \u{2E31} **`{armor_name}`**`{: <armor_pad$}` \u{2E31} **`RLD:`**`{rld: >4}`\n\
                 **`FP:`**`{fp: >5}` \u{2E31} **`TRP:`**`{trp: >4}` \u{2E31} **`EVA:`**`{eva: >4}`\n\
                 **`AA:`**`{aa: >5}` \u{2E31} **`AVI:`**`{avi: >4}` \u{2E31} **`ACC:`**`{acc: >4}`\n\
                 **`OXY:`**`{oxy: >4}` \u{2E31} **`AMO:`**`{amo: >4}` \u{2E31} **`SPD:`**`{spd: >4}`\n\
                 **`LCK:`**`{lck: >4}` \u{2E31} **`Cost:`**`{cost: >3}`",
                "",
            )
        };

        [embed_field_create("Stats", content, false)]
    }

    /// Creates the embed field that displays the weapon equipment slots.
    fn get_equip_field<'a>(
        &self,
        azur: LoadedConfig<'a>,
        ship: &ShipData,
    ) -> [EmbedFieldCreate<'a>; 1] {
        let slots = ship
            .equip_slots
            .iter()
            .filter_map(|e| e.mount.as_ref().map(|m| (&e.allowed, m)));

        let mut text = String::new();
        for (allowed, mount) in slots {
            if !text.is_empty() {
                text.push('\n');
            }

            let slots = Join::simple("/")
                .display_as(allowed, |&kind| equip_slot_display(azur.wiki_urls(), kind));

            write!(
                text,
                "**`{: >3.0}%`**`x{}` {slots}",
                mount.efficiency * 100f64,
                mount.mounts
            );

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

        [embed_field_create("Equipment", text, false)]
    }

    /// Creates the embed field that display the skill summary.
    fn get_skills_field<'a>(
        &self,
        azur: LoadedConfig<'a>,
        ship: &ShipData,
    ) -> Option<EmbedFieldCreate<'a>> {
        // There isn't any way a unique augment can do anything if there are no skills
        // so we still skip the field if there are no skills but there is an augment.
        // ... Not that there are any ships without skills to begin with.
        (!ship.skills.is_empty()).then(|| {
            let mut text = String::new();
            for s in &ship.skills {
                if !text.is_empty() {
                    text.push('\n');
                }

                write!(text, "{} **{}**", s.category.emoji(), s.name);
            }

            let augments = azur.game_data().augments_by_ship_id(ship.group_id);
            for augment in augments {
                if !text.is_empty() {
                    text.push('\n');
                }

                write!(text, "-# UA: **{}**", augment.name);
            }

            embed_field_create("Skills", text, false)
        })
    }
}

button_value!(for<'v> View<'v>, 1);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let ship = azur
            .game_data()
            .ship_by_id(self.ship_id)
            .ok_or(AzurParseError::Ship)?;

        let edit = match self
            .retrofit
            .and_then(|index| ship.retrofits.get(usize::from(index)))
        {
            None => self.edit_with_ship(&ctx, azur, ship, None),
            Some(retrofit) => self.edit_with_ship(&ctx, azur, retrofit, Some(ship)),
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
