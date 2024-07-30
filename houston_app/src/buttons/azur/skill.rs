use std::collections::HashMap;

use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

use crate::buttons::*;
use super::AugmentParseError;
use super::ShipParseError;

/// View skill details of a ship or augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub source: ViewSource,
    pub skill_index: Option<u8>,
    pub back: Option<CustomData>
}

/// Where to load the skills from.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ViewSource {
    Ship(u32, Option<u8>),
    Augment(u32),
}

type OwnedCreateEmbedField = (String, String, bool);

impl View {
    /// Creates a new instance including a button to go back with some custom ID.
    pub fn with_back(source: ViewSource, back: CustomData) -> Self {
        Self { source, skill_index: None, back: Some(back) }
    }

    /// Modifies the create-reply with a preresolved list of skills and a base embed.
    fn modify_with_skills<'a>(mut self, create: CreateReply, iterator: impl Iterator<Item = &'a Skill>, mut embed: CreateEmbed) -> CreateReply {
        let index = self.skill_index.map(usize::from);
        let mut components = Vec::new();

        if let Some(ref back) = self.back {
            components.push(CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back"));
        }

        for (t_index, skill) in iterator.enumerate().take(4) {
            if Some(t_index) == index {
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

        let rows = vec![
            CreateActionRow::Buttons(components)
        ];

        create.embed(embed).components(rows)
    }

    /// Modifies the create-reply with preresolved ship data.
    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);
        self.modify_with_skills(
            create,
            ship.skills.iter(),
            CreateEmbed::new().color(ship.rarity.color_rgb()).author(super::get_ship_wiki_url(base_ship))
        )
    }

    /// Modifies the create-reply with preresolved augment data.
    pub fn modify_with_augment(self, create: CreateReply, augment: &Augment) -> CreateReply {
        self.modify_with_skills(
            create,
            augment.effect.iter().chain(augment.skill_upgrade.as_ref()),
            CreateEmbed::new().color(ShipRarity::SR.color_rgb()).author(CreateEmbedAuthor::new(&augment.name))
        )
    }

    /// Creates a button that redirects to a skill index.
    fn button_with_skill(&mut self, index: usize) -> CreateButton {
        self.new_button(utils::field_mut!(Self: skill_index), Some(index as u8), |u| u.unwrap_or_default().into())
    }

    /// Creates the embed field for a skill.
    fn create_skill_field(&self, skill: &Skill) -> [OwnedCreateEmbedField; 1] {
        [(
            format!("{} {}", skill.category.emoji(), skill.name),
            utils::text::truncate(&skill.description, 1000),
            false
        )]
    }

    /// Creates the embed fields for the selected skill.
    fn create_ex_skill_fields(&self, skill: &Skill) -> Vec<OwnedCreateEmbedField> {
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
                    if m.len() <= 1024 { m } else { log::warn!("barrage:\n{m}"); "<barrage data too long>".to_owned() }
                },
                false
            ));
        }

        for weapon in &skill.new_weapons {
            fields.push((
                format!("__{}__", weapon.name.as_deref().unwrap_or("Special Weapon")),
                crate::fmt::azur::DisplayWeapon::new(weapon).to_string(),
                true
            ))
        }

        fields
    }
}

impl ButtonArgsModify for View {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        match self.source {
            ViewSource::Ship(ship_id, retro_index) => {
                let base_ship = data.azur_lane().ship_by_id(ship_id).ok_or(ShipParseError)?;
                let ship = retro_index.and_then(|i| base_ship.retrofits.get(usize::from(i))).unwrap_or(base_ship);
                Ok(self.modify_with_ship(create, ship, Some(base_ship)))
            }
            ViewSource::Augment(augment_id) => {
                let augment = data.azur_lane().augment_by_id(augment_id).ok_or(AugmentParseError)?;
                Ok(self.modify_with_augment(create, augment))
            }
        }
    }
}

/// Constructs skill barrage display data.
fn get_skills_extra_summary(skill: &Skill) -> String {
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
            "__`Trgt. | Amount x  Dmg. | Ammo:  L / M / H  | Scaling `__\n{sum}"
            // `Fix.  |     12 x  58.0 | Nor.: 120/ 80/ 80 | 100%  FP`
        )
    }

    fn get_skill_attack_summary(attack: &SkillAttack) -> Option<String> {
        match &attack.weapon.data {
            WeaponData::Bullets(bullets) => get_barrage_summary(bullets, Some(attack.target)),
            WeaponData::Aircraft(aircraft) => idk!(
                get_aircraft_summary(aircraft),
                "`{: >5} | {: >6} x Aircraft                            `\n{sum}",
                attack.target.short_name(), aircraft.amount
            ),
            _ => None
        }
    }

    fn get_barrage_summary(barrage: &Barrage, target: Option<SkillAttackTarget>) -> Option<String> {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct Key { kind: BulletKind, ammo: AmmoKind }
        struct Value<'a> { amount: u32, bullet: &'a Bullet }

        let mut sets: HashMap<Key, Value> = HashMap::new();
        for bullet in barrage.bullets.iter() {
            let key = Key { kind: bullet.kind, ammo: bullet.ammo };
            sets.entry(key)
                .and_modify(|v| v.amount += bullet.amount)
                .or_insert(Value { amount: bullet.amount, bullet });
        }

        join("\n", sets.into_iter().map(|(key, Value { amount, bullet })| {
            let ArmorModifiers(l, m, h) = bullet.modifiers;
            format!(
                // damage with coeff |
                // ammo type & mods |
                // % of scaling stat |
                // amount | totals
                "`\
                {: <5} | \
                {: >6} x{: >6.1} | \
                {: >4}: {: >3.0}/{: >3.0}/{: >3.0} | \
                {: >3.0}% {: >3}`",
                target.map(|t| t.short_name()).unwrap_or(""),
                amount, barrage.damage * barrage.coefficient,
                key.ammo.short_name(), l * 100f64, m * 100f64, h * 100f64,
                barrage.scaling * 100f64, barrage.scaling_stat.name()
            )
        }))
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
