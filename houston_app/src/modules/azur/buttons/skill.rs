use arrayvec::ArrayVec;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;
use utils::text::truncate;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::modules::azur::LoadedConfig;

/// View skill details of a ship or augment.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    source: ViewSource,
    #[builder(default = None)]
    skill_index: Option<u8>,
    #[serde(borrow)]
    back: Nav<'v>,
    // this should honestly be in `ViewSource::Ship` but that's a pain
    #[builder(default = None, vis = "pub(self)")]
    augment_index: Option<u8>,
}

/// Where to load the skills from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewSource {
    Ship { ship_id: u32, retrofit: Option<u8> },
    Augment(u32),
}

impl ViewSource {
    pub const fn ship(ship_id: u32, retrofit: Option<u8>) -> Self {
        Self::Ship { ship_id, retrofit }
    }
}

impl View<'_> {
    /// Modifies the create-reply with a preresolved list of skills and a base
    /// embed.
    fn edit_with_skills<'a>(
        mut self,
        iterator: impl Iterator<Item = &'a Skill>,
        components: &mut CreateComponents<'a>,
    ) {
        components.push(CreateSeparator::new(true));

        for (index, skill) in (0..5u8).zip(iterator) {
            self.append_skill(components, index, skill);
        }
    }

    /// Appends info for a skill to the `components`.
    fn append_skill<'a>(
        &mut self,
        components: &mut CreateComponents<'a>,
        index: u8,
        skill: &'a Skill,
    ) {
        let selected = self.skill_index == Some(index);
        let style = if selected { "__" } else { "" };

        let label = format!(
            "### {style}{}{style} {}",
            skill.category.emoji(),
            skill.name
        );

        components.push(CreateTextDisplay::new(label));
        components.push(CreateTextDisplay::new(truncate(&skill.description, 1000)));

        if !skill.barrages.is_empty() || !skill.new_weapons.is_empty() {
            if selected {
                self.append_barrage_info(skill, components);
            } else {
                let button = self
                    .button_with_skill(Some(index))
                    .label("Show Barrage")
                    .style(ButtonStyle::Secondary);

                components.push(CreateActionRow::buttons(vec![button]));
            }
        }

        components.push(CreateSeparator::new(true));
    }

    /// Modifies the create-reply with preresolved ship data.
    fn edit_with_ship<'a>(mut self, azur: LoadedConfig<'a>, ship: &'a ShipData) -> EditReply<'a> {
        let mut skills: ArrayVec<&Skill, 5> = ship.skills.iter().take(4).collect();

        let mut components = CreateComponents::new();
        components.push(CreateTextDisplay::new(format!(
            "### {} [Skills]",
            ship.name
        )));

        let nav = CreateButton::new(self.back.to_custom_id())
            .emoji(emoji::back())
            .label("Back");

        let mut nav = vec![nav];

        let augments = azur.game_data().augments_by_ship_id(ship.group_id);
        for (index, augment) in (0..4u8).zip(augments) {
            if index == 0 {
                nav.push(self.button_with_augment(None).label("Default"));
            }

            let index = Some(index);
            nav.push(
                self.button_with_augment(index)
                    .label(truncate(&augment.name, 80)),
            );

            if index == self.augment_index {
                // replace upgraded skills
                for upgrade in &augment.skill_upgrades {
                    if let Some(skill) =
                        skills.iter_mut().find(|s| s.buff_id == upgrade.original_id)
                    {
                        *skill = &upgrade.skill;
                    }
                }

                // append augment effect
                if let Some(effect) = &augment.effect {
                    _ = skills.try_push(effect);
                }

                components.push(CreateSeparator::new(true));
                components.push(CreateTextDisplay::new(format!(
                    "**\"{}\" Bonus Stats**\n{}",
                    augment.name,
                    crate::fmt::azur::AugmentStats::new(augment),
                )));
            }
        }

        // no need for `into_iter`, also avoids moving the entire ArrayVec
        self.edit_with_skills(skills.iter().copied(), &mut components);
        components.push(CreateActionRow::buttons(nav));

        EditReply::clear().components_v2(components![
            CreateContainer::new(components).accent_color(ship.rarity.color_rgb())
        ])
    }

    /// Modifies the create-reply with preresolved augment data.
    fn edit_with_augment(self, augment: &Augment) -> EditReply<'_> {
        let skills = augment
            .effect
            .iter()
            .chain(augment.skill_upgrades.iter().map(|s| &s.skill));

        let mut components = CreateComponents::new();
        components.push(CreateTextDisplay::new(format!(
            "### {} [Skills]",
            augment.name
        )));

        let nav = CreateActionRow::buttons(vec![
            CreateButton::new(self.back.to_custom_id())
                .emoji(emoji::back())
                .label("Back"),
        ]);

        self.edit_with_skills(skills, &mut components);
        components.push(nav);

        EditReply::clear().components_v2(components![
            CreateContainer::new(components).accent_color(augment.rarity.color_rgb())
        ])
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

    /// Creates the embed fields for the selected skill.
    fn append_barrage_info<'a>(&self, skill: &'a Skill, components: &mut CreateComponents<'a>) {
        if !skill.barrages.is_empty() {
            let full = get_skills_extra_summary(skill);
            let description = match truncate(&full, 1024) {
                Cow::Owned(trunc) => {
                    log::warn!("Barrage data too long:\n{full}");
                    trunc
                },
                Cow::Borrowed(_) => full,
            };

            components.push(CreateSeparator::new(false));
            components.push(CreateTextDisplay::new("### __Barrage__"));
            components.push(CreateTextDisplay::new(description));
        }

        for buff in &skill.new_weapons {
            let mut fmt = crate::fmt::azur::Details::new(&buff.weapon);
            if buff.duration.is_some() {
                fmt = fmt.no_fire_rate();
            }

            let label = format!(
                "### __{}__",
                buff.weapon.name.as_deref().unwrap_or("Special Weapon")
            );

            components.push(CreateSeparator::new(false));
            components.push(CreateTextDisplay::new(label));
            components.push(CreateTextDisplay::new(fmt.to_string()));
        }
    }
}

button_value!(for<'v> View<'v>, 3);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;
        let edit = match self.source {
            ViewSource::Ship { ship_id, retrofit } => {
                let base_ship = azur
                    .game_data()
                    .ship_by_id(ship_id)
                    .ok_or(AzurParseError::Ship)?;

                let ship = retrofit
                    .and_then(|i| base_ship.retrofits.get(usize::from(i)))
                    .unwrap_or(base_ship);

                self.edit_with_ship(azur, ship)
            },
            ViewSource::Augment(augment_id) => {
                let augment = azur
                    .game_data()
                    .augment_by_id(augment_id)
                    .ok_or(AzurParseError::Augment)?;
                self.edit_with_augment(augment)
            },
        };

        ctx.edit(edit).await
    }
}

/// Constructs skill barrage display data.
fn get_skills_extra_summary(skill: &Skill) -> String {
    use utils::text::{InlineStr, WriteStr as _};

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
                writeln!(
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
            write!(
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
