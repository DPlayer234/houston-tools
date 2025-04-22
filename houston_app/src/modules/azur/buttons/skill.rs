use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;
use smallvec::{SmallVec, smallvec};
use utils::text::truncate;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::modules::azur::LoadedConfig;

/// View skill details of a ship or augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub source: ViewSource,
    pub skill_index: Option<u8>,
    #[serde(borrow)]
    pub back: Nav<'v>,
    // this should honestly be in `ShipViewSource` but that's a pain
    augment_index: Option<u8>,
}

/// Where to load the skills from.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ViewSource {
    Ship(ShipViewSource),
    Augment(u32),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShipViewSource {
    pub ship_id: u32,
    pub retrofit: Option<u8>,
}

impl ShipViewSource {
    pub fn new(ship_id: u32, retrofit: Option<u8>) -> Self {
        Self { ship_id, retrofit }
    }
}

impl From<ShipViewSource> for ViewSource {
    fn from(value: ShipViewSource) -> Self {
        Self::Ship(value)
    }
}

impl<'v> View<'v> {
    /// Creates a new instance including a button to go back with some custom
    /// ID.
    pub fn with_back(source: ViewSource, back: Nav<'v>) -> Self {
        Self {
            source,
            skill_index: None,
            back,
            augment_index: None,
        }
    }

    /// Modifies the create-reply with a preresolved list of skills and a base
    /// embed.
    fn edit_with_skills<'a>(
        mut self,
        iterator: impl Iterator<Item = &'a Skill>,
        mut embed: CreateEmbed<'a>,
    ) -> (CreateEmbed<'a>, CreateActionRow<'a>) {
        let mut components = Vec::new();

        for (t_index, skill) in (0..5u8).zip(iterator) {
            let t_index = Some(t_index);
            if t_index == self.skill_index {
                embed = embed
                    .color(skill.category.color_rgb())
                    .fields(self.create_ex_skill_fields(skill));
            } else {
                embed = embed.fields(self.create_skill_field(skill));
            }

            if !skill.barrages.is_empty() || !skill.new_weapons.is_empty() {
                let button = self
                    .button_with_skill(t_index)
                    .label(truncate(&skill.name, 80))
                    .style(ButtonStyle::Secondary);

                components.push(button);
            }
        }

        (embed, CreateActionRow::buttons(components))
    }

    /// Modifies the create-reply with preresolved ship data.
    fn edit_with_ship<'a>(
        mut self,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: Option<&'a ShipData>,
    ) -> EditReply<'a> {
        let base_ship = base_ship.unwrap_or(ship);

        let mut skills: Vec<&Skill> = ship.skills.iter().take(4).collect();
        let mut embed = CreateEmbed::new()
            .color(ship.rarity.color_rgb())
            .author(azur.wiki_urls().ship(base_ship));

        let components = CreateButton::new(self.back.to_custom_id())
            .emoji(emoji::back())
            .label("Back");
        let mut components = vec![components];

        let augments = azur.game_data().augments_by_ship_id(ship.group_id);
        for (a_index, augment) in (0..4u8).zip(augments) {
            if a_index == 0 {
                components.push(self.button_with_augment(None).label("Default"));
            }

            let a_index = Some(a_index);
            components.push(
                self.button_with_augment(a_index)
                    .label(truncate(&augment.name, 80)),
            );

            if a_index == self.augment_index {
                // replace upgraded skill
                if let Some(upgrade) = &augment.skill_upgrade {
                    if let Some(skill) =
                        skills.iter_mut().find(|s| s.buff_id == upgrade.original_id)
                    {
                        *skill = &upgrade.skill;
                    }
                }

                // append augment effect
                if let Some(effect) = &augment.effect {
                    skills.push(effect);
                }

                embed = embed.field(
                    format!("'{}' Bonus Stats", augment.name),
                    format!("{}", crate::fmt::azur::AugmentStats::new(augment)),
                    false,
                );
            }
        }

        let (embed, row) = self.edit_with_skills(skills.into_iter(), embed);
        EditReply::clear()
            .embed(embed)
            .components(rows_without_empty([
                CreateActionRow::buttons(components),
                row,
            ]))
    }

    /// Modifies the create-reply with preresolved augment data.
    fn edit_with_augment(self, augment: &Augment) -> EditReply<'_> {
        let embed = CreateEmbed::new()
            .color(augment.rarity.color_rgb())
            .author(CreateEmbedAuthor::new(&augment.name));

        let skills = augment
            .effect
            .iter()
            .chain(augment.skill_upgrade.as_ref().map(|s| &s.skill));

        let nav_row = CreateActionRow::buttons(vec![
            CreateButton::new(self.back.to_custom_id())
                .emoji(emoji::back())
                .label("Back"),
        ]);

        let (embed, row) = self.edit_with_skills(skills, embed);
        EditReply::clear()
            .embed(embed)
            .components(rows_without_empty([nav_row, row]))
    }

    /// Creates a button that redirects to a skill index.
    fn button_with_skill<'a>(&mut self, index: Option<u8>) -> CreateButton<'a> {
        self.button_with_u8(|s| &mut s.skill_index, index)
    }

    /// Creates a button that redirects to a skill index.
    fn button_with_augment<'a>(&mut self, index: Option<u8>) -> CreateButton<'a> {
        self.button_with_u8(|s| &mut s.augment_index, index)
    }

    /// Shared logic for buttons that use a `Option<u8>` field.
    fn button_with_u8<'a>(
        &mut self,
        field: impl Fn(&mut Self) -> &mut Option<u8>,
        index: Option<u8>,
    ) -> CreateButton<'a> {
        self.new_button(field, index, |u| u.map_or(u16::MAX, u16::from))
    }

    /// Creates the embed field for a skill.
    fn create_skill_field<'a>(&self, skill: &'a Skill) -> [EmbedFieldCreate<'a>; 1] {
        [embed_field_create(
            format!("{} {}", skill.category.emoji(), skill.name),
            truncate(&skill.description, 1000),
            false,
        )]
    }

    /// Creates the embed fields for the selected skill.
    fn create_ex_skill_fields<'a>(&self, skill: &'a Skill) -> SmallVec<[EmbedFieldCreate<'a>; 2]> {
        let mut fields = smallvec![embed_field_create(
            format!("{} __{}__", skill.category.emoji(), skill.name),
            truncate(&skill.description, 1000),
            false,
        )];

        if !skill.barrages.is_empty() {
            let full = get_skills_extra_summary(skill);
            fields.push(embed_field_create(
                "__Barrage__".to_owned(),
                match truncate(&full, 1024) {
                    Cow::Owned(trunc) => {
                        log::warn!("Barrage data too long:\n{full}");
                        trunc
                    },
                    Cow::Borrowed(_) => full,
                },
                false,
            ));
        }

        for buff in &skill.new_weapons {
            let mut fmt = crate::fmt::azur::Details::new(&buff.weapon);
            if buff.duration.is_some() {
                fmt = fmt.no_fire_rate();
            }

            fields.push(embed_field_create(
                format!(
                    "__{}__",
                    buff.weapon.name.as_deref().unwrap_or("Special Weapon")
                ),
                fmt.to_string(),
                true,
            ))
        }

        fields
    }
}

fn rows_without_empty<'a, I>(rows: I) -> Vec<CreateActionRow<'a>>
where
    I: IntoIterator<Item = CreateActionRow<'a>>,
{
    rows.into_iter()
        .filter(|a| !matches!(a, CreateActionRow::Buttons(a) if a.is_empty()))
        .collect()
}

button_value!(View<'_>, 3);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let edit = match &self.source {
            ViewSource::Ship(source) => {
                let base_ship = azur
                    .game_data()
                    .ship_by_id(source.ship_id)
                    .ok_or(AzurParseError::Ship)?;
                let ship = source
                    .retrofit
                    .and_then(|i| base_ship.retrofits.get(usize::from(i)))
                    .unwrap_or(base_ship);
                self.edit_with_ship(azur, ship, Some(base_ship))
            },
            ViewSource::Augment(augment_id) => {
                let augment = azur
                    .game_data()
                    .augment_by_id(*augment_id)
                    .ok_or(AzurParseError::Augment)?;
                self.edit_with_augment(augment)
            },
        };

        ctx.edit(edit).await
    }
}

/// Constructs skill barrage display data.
fn get_skills_extra_summary(skill: &Skill) -> String {
    use utils::text::InlineStr;
    use utils::text::write_str::*;

    let mut buf = String::new();
    write_join_map(
        &mut buf,
        "\n\n",
        &skill.barrages,
        write_skill_barrage_summary,
    );

    if buf.is_empty() {
        // this happens if the barrage were to be entirely
        // aircraft without surface damage barrages.
        buf.push_str("<recon only>");
    }

    return buf;

    fn write_join_map<I, F>(buf: &mut String, join: &str, iter: I, mut f: F) -> bool
    where
        I: IntoIterator,
        F: FnMut(&mut String, I::Item) -> bool,
    {
        let mut any = false;
        let mut last = false;

        for item in iter {
            if last {
                buf.push_str(join);
            }

            last = f(buf, item);
            any |= last;
        }

        any
    }

    fn try_write_or_undo<F>(buf: &mut String, f: F) -> bool
    where
        F: FnOnce(&mut String) -> bool,
    {
        let start = buf.len();
        let any = f(buf);
        if !any {
            buf.truncate(start);
        }

        any
    }

    fn write_skill_barrage_summary(buf: &mut String, barrage: &SkillBarrage) -> bool {
        try_write_or_undo(buf, |buf| {
            buf.push_str("__`Trgt. | Dmg.       | Ammo:  L / M / H  | Scaling  | Fl.`__\n");
            write_join_map(buf, "\n", &barrage.attacks, write_skill_attack_summary)
        })
    }

    fn write_skill_attack_summary(buf: &mut String, attack: &SkillAttack) -> bool {
        match &attack.weapon.data {
            WeaponData::Bullets(bullets) => {
                write_barrage_summary(buf, bullets, Some(attack.target))
            },
            WeaponData::Aircraft(aircraft) => try_write_or_undo(buf, |buf| {
                writeln_str!(
                    buf,
                    "`{: >5} |{: >3} x Aircraft                             |    `",
                    attack.target.short_name(),
                    aircraft.amount
                );
                write_aircraft_summary(buf, aircraft)
            }),
            _ => false,
        }
    }

    fn write_barrage_summary(
        buf: &mut String,
        barrage: &Barrage,
        target: Option<SkillAttackTarget>,
    ) -> bool {
        struct Value<'a> {
            amount: u32,
            bullet: &'a Bullet,
        }

        fn match_key(a: &Bullet, b: &Bullet) -> bool {
            a.kind == b.kind && a.ammo == b.ammo && a.modifiers == b.modifiers
        }

        let mut sets: Vec<Value<'_>> = Vec::new();
        for bullet in &barrage.bullets {
            // find & modify, or insert
            match sets.iter_mut().find(|i| match_key(i.bullet, bullet)) {
                Some(entry) => entry.amount += bullet.amount,
                None => sets.push(Value {
                    amount: bullet.amount,
                    bullet,
                }),
            }
        }

        write_join_map(buf, "\n", sets, |buf, Value { amount, bullet }| {
            let ArmorModifiers(l, m, h) = bullet.modifiers;
            let shrapnel_mark = if bullet.kind == BulletKind::Shrapnel {
                "*"
            } else {
                " "
            };
            write_str!(
                buf,
                // damage with coeff |
                // ammo type & mods |
                // % of scaling stat |
                // amount | totals
                "`\
                {: <5} |\
                {: >3} x{: >6.1}{}|\
                {: >5}: {: >3.0}/{: >3.0}/{: >3.0} |\
                {: >4.0}% {: <3} | \
                {}`",
                target.map_or("", |t| t.short_name()),
                amount,
                barrage.damage * barrage.coefficient,
                shrapnel_mark,
                bullet.ammo.short_name(),
                l * 100f64,
                m * 100f64,
                h * 100f64,
                barrage.scaling * 100f64,
                barrage.scaling_stat.name(),
                get_bullet_flags(bullet),
            );
            true
        })
    }

    fn write_aircraft_summary(buf: &mut String, aircraft: &Aircraft) -> bool {
        write_join_map(buf, "\n", &aircraft.weapons, |buf, weapon| {
            match &weapon.data {
                WeaponData::Bullets(barrage) => write_barrage_summary(buf, barrage, None),
                _ => false,
            }
        })
    }

    fn get_bullet_flags(bullet: &Bullet) -> InlineStr<3> {
        let mut res = [b'-'; 3];
        if bullet.pierce != 0 {
            res[0] = b'P';
        }
        if bullet.flags.contains(BulletFlags::IGNORE_SHIELD) {
            res[1] = b'I';
        }
        if bullet.flags.dive_filter().is_empty() {
            res[2] = b'D';
        }

        // SAFETY: Always ASCII here.
        unsafe { InlineStr::from_utf8_unchecked(res) }
    }
}
