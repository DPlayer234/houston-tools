use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

use crate::buttons::*;
use super::AzurParseError;

/// View skill details of a ship or augment.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub source: ViewSource,
    pub skill_index: Option<u8>,
    pub back: Option<CustomData>,
    augment_index: Option<u8>,
}

/// Where to load the skills from.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ViewSource {
    Ship(ShipViewSource),
    Augment(u32),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

type EmbedFieldCreate<'a> = (String, Cow<'a, str>, bool);

impl View {
    /// Creates a new instance including a button to go back with some custom ID.
    pub fn with_back(source: ViewSource, back: CustomData) -> Self {
        Self { source, skill_index: None, back: Some(back), augment_index: None }
    }

    /// Modifies the create-reply with a preresolved list of skills and a base embed.
    fn modify_with_skills<'a>(
        mut self,
        iterator: impl Iterator<Item = &'a Skill>,
        mut embed: CreateEmbed<'a>,
    ) -> (CreateEmbed<'a>, CreateActionRow<'a>) {
        let mut components = Vec::new();

        for (t_index, skill) in iterator.enumerate().take(5) {
            #[allow(clippy::cast_possible_truncation)]
            let t_index = Some(t_index as u8);

            if t_index == self.skill_index {
                embed = embed.color(skill.category.color_rgb())
                    .fields(self.create_ex_skill_fields(skill));
            } else {
                embed = embed.fields(self.create_skill_field(skill));
            }

            if !skill.barrages.is_empty() || !skill.new_weapons.is_empty() {
                let button = self.button_with_skill(t_index)
                    .label(utils::text::truncate(&skill.name, 25))
                    .style(ButtonStyle::Secondary);

                components.push(button);
            }
        }

        (embed, CreateActionRow::buttons(components))
    }

    /// Modifies the create-reply with preresolved ship data.
    fn modify_with_ship<'a>(
        mut self,
        data: &'a HBotData,
        create: CreateReply<'a>,
        ship: &'a ShipData,
        base_ship: Option<&'a ShipData>,
    ) -> CreateReply<'a> {
        let base_ship = base_ship.unwrap_or(ship);

        let mut skills: Vec<&Skill> = ship.skills.iter().take(4).collect();
        let mut embed = CreateEmbed::new().color(ship.rarity.color_rgb()).author(super::get_ship_wiki_url(base_ship));

        let mut components = Vec::new();
        if let Some(back) = &self.back {
            components.push(CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back"));
        }

        for (a_index, augment) in data.azur_lane().augments_by_ship_id(ship.group_id).enumerate().take(4) {
            if a_index == 0 {
                components.push(
                    self.button_with_augment(None)
                        .label("Default")
                );
            }

            #[allow(clippy::cast_possible_truncation)]
            let a_index = Some(a_index as u8);
            components.push(
                self.button_with_augment(a_index)
                    .label(utils::text::truncate(&augment.name, 25))
            );

            if a_index == self.augment_index {
                // replace upgraded skill
                if let Some(upgrade) = &augment.skill_upgrade {
                    if let Some(skill) = skills.iter_mut().find(|s| s.buff_id == upgrade.original_id) {
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
                    false
                );
            }
        }

        let (embed, row) = self.modify_with_skills(skills.into_iter(), embed);
        create.embed(embed).components(rows_without_empty([CreateActionRow::buttons(components), row]))
    }

    /// Modifies the create-reply with preresolved augment data.
    fn modify_with_augment<'a>(self, create: CreateReply<'a>, augment: &'a Augment) -> CreateReply<'a> {
        let embed = CreateEmbed::new().color(ShipRarity::SR.color_rgb()).author(CreateEmbedAuthor::new(&augment.name));
        let skills = augment.effect.iter().chain(augment.skill_upgrade.as_ref().map(|s| &s.skill));

        let nav_row = self.back.as_ref().map(|back| CreateActionRow::buttons(vec![
            CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back")
        ]));

        let (embed, row) = self.modify_with_skills(skills, embed);
        create.embed(embed).components(rows_without_empty([nav_row, Some(row)]))
    }

    /// Creates a button that redirects to a skill index.
    fn button_with_skill<'a>(&mut self, index: Option<u8>) -> CreateButton<'a> {
        self.button_with_u8(utils::field_mut!(Self: skill_index), index)
    }

    /// Creates a button that redirects to a skill index.
    fn button_with_augment<'a>(&mut self, index: Option<u8>) -> CreateButton<'a> {
        self.button_with_u8(utils::field_mut!(Self: augment_index), index)
    }

    /// Shared logic for buttons that use a `Option<u8>` field.
    fn button_with_u8<'a>(&mut self, field: impl FieldMut<Self, Option<u8>>, index: Option<u8>) -> CreateButton<'a> {
        self.new_button(field, index, |u| u.map_or(u16::MAX, u16::from))
    }

    /// Creates the embed field for a skill.
    fn create_skill_field<'a>(&self, skill: &'a Skill) -> [EmbedFieldCreate<'a>; 1] {
        [(
            format!("{} {}", skill.category.emoji(), skill.name),
            utils::text::truncate(&skill.description, 1000),
            false,
        )]
    }

    /// Creates the embed fields for the selected skill.
    fn create_ex_skill_fields<'a>(&self, skill: &'a Skill) -> Vec<EmbedFieldCreate<'a>> {
        let mut fields = vec![(
            format!("{} __{}__", skill.category.emoji(), skill.name),
            utils::text::truncate(&skill.description, 1000),
            false
        )];

        if !skill.barrages.is_empty() {
            fields.push((
                "__Barrage__".to_owned(),
                {
                    let m = get_skills_extra_summary(skill);
                    if m.len() <= 1024 { m.into() } else { log::warn!("barrage:\n{m}"); "<barrage data too long>".into() }
                },
                false
            ));
        }

        for buff in &skill.new_weapons {
            let fmt = crate::fmt::azur::Details::new(&buff.weapon);
            fields.push((
                format!("__{}__", buff.weapon.name.as_deref().unwrap_or("Special Weapon")),
                match buff.duration {
                    Some(_) => fmt.no_fire_rate().to_string().into(),
                    None => fmt.to_string().into(),
                },
                true
            ))
        }

        fields
    }
}

fn rows_without_empty<'a, I, T>(rows: I) -> Vec<CreateActionRow<'a>>
where
    I: IntoIterator<Item = T>,
    T: Into<Option<CreateActionRow<'a>>>,
{
    rows.into_iter()
        .filter_map(|a| a.into())
        .filter(|a| !matches!(a, CreateActionRow::Buttons(a) if a.is_empty()))
        .collect()
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        match &self.source {
            ViewSource::Ship(source) => {
                let base_ship = ctx.data.azur_lane().ship_by_id(source.ship_id).ok_or(AzurParseError::Ship)?;
                let ship = source.retrofit.and_then(|i| base_ship.retrofits.get(usize::from(i))).unwrap_or(base_ship);
                Ok(self.modify_with_ship(ctx.data, ctx.create_reply(), ship, Some(base_ship)))
            }
            ViewSource::Augment(augment_id) => {
                let augment = ctx.data.azur_lane().augment_by_id(*augment_id).ok_or(AzurParseError::Augment)?;
                Ok(self.modify_with_augment(ctx.create_reply(), augment))
            }
        }
    }
}

/// Constructs skill barrage display data.
fn get_skills_extra_summary(skill: &Skill) -> String {
    use utils::text::InlineStr;

    return join("\n\n", skill.barrages.iter().filter_map(get_skill_barrage_summary)).unwrap_or_else(String::new);

    macro_rules! idk {
        ($opt:expr, $($arg:tt)*) => {
            match $opt {
                None => None,
                Some(v) => Some(format!($($arg)*, sum = v))
            }
        };
    }

    fn get_skill_barrage_summary(barrage: &SkillBarrage) -> Option<String> {
        idk!(
            join("\n", barrage.attacks.iter().filter_map(get_skill_attack_summary)),
            "__`Trgt. | Dmg.       | Ammo:  L / M / H  | Scaling  | Fl.`__\n{sum}"
            // `Fix.  | 12 x  58.0 | Nor.: 120/ 80/ 80 | 100% AVI | ---`
        )
    }

    fn get_skill_attack_summary(attack: &SkillAttack) -> Option<String> {
        match &attack.weapon.data {
            WeaponData::Bullets(bullets) => get_barrage_summary(bullets, Some(attack.target)),
            WeaponData::Aircraft(aircraft) => idk!(
                get_aircraft_summary(aircraft),
                "`{: >5} |{: >3} x Aircraft                             |    `\n{sum}",
                attack.target.short_name(), aircraft.amount
            ),
            _ => None
        }
    }

    fn get_barrage_summary(barrage: &Barrage, target: Option<SkillAttackTarget>) -> Option<String> {
        struct Value<'a> { amount: u32, bullet: &'a Bullet }

        fn match_key(a: &Bullet, b: &Bullet) -> bool {
            a.kind == b.kind &&
            a.ammo == b.ammo &&
            a.modifiers == b.modifiers
        }

        let mut sets: Vec<Value> = Vec::new();
        for bullet in &barrage.bullets {
            // find & modify, or insert
            match sets.iter_mut().find(|i| match_key(i.bullet, bullet)) {
                Some(entry) => entry.amount += bullet.amount,
                None => sets.push(Value { amount: bullet.amount, bullet }),
            }
        }

        join("\n", sets.into_iter().map(|Value { amount, bullet }| {
            let ArmorModifiers(l, m, h) = bullet.modifiers;
            let sprapnel_mark = if bullet.kind == BulletKind::Shrapnel { "*" } else { " " };
            format!(
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
                amount, barrage.damage * barrage.coefficient, sprapnel_mark,
                bullet.ammo.short_name(), l * 100f64, m * 100f64, h * 100f64,
                barrage.scaling * 100f64, barrage.scaling_stat.name(),
                get_bullet_flags(bullet),
            )
        }))
    }

    fn get_bullet_flags(bullet: &Bullet) -> InlineStr<3> {
        let mut res = [b'-'; 3];
        if bullet.pierce != 0 { res[0] = b'P'; }
        if bullet.flags.contains(BulletFlags::IGNORE_SHIELD) { res[1] = b'I'; }
        if bullet.flags.dive_filter().is_empty() { res[2] = b'D'; }

        // SAFETY: Always ASCII here.
        unsafe { InlineStr::from_utf8_unchecked(res) }
    }

    fn get_aircraft_summary(aircraft: &Aircraft) -> Option<String> {
        join("\n", aircraft.weapons.iter().filter_map(|weapon| match &weapon.data {
            WeaponData::Bullets(barrage) => get_barrage_summary(barrage, None),
            _ => None
        }))
    }

    fn join(separator: &str, mut items: impl Iterator<Item = String>) -> Option<String> {
        let mut result = items.next()?;
        for item in items {
            result.push_str(separator);
            result.push_str(&item);
        }
        Some(result)
    }
}
