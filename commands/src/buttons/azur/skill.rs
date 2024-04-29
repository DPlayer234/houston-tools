use std::collections::HashMap;
//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

use super::AugmentParseError;
use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewSkill {
    pub source: ViewSkillSource,
    pub skill_index: Option<u8>,
    pub back: Option<String>
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub enum ViewSkillSource {
    Ship(u32, Option<u8>),
    Augment(u32),
}

impl From<ViewSkill> for ButtonArgs {
    fn from(value: ViewSkill) -> Self {
        ButtonArgs::ViewSkill(value)
    }
}

type OwnedCreateEmbedField = (String, String, bool);

impl ViewSkill {
    pub fn new(source: ViewSkillSource) -> Self {
        Self { source, skill_index: None, back: None }
    }

    pub fn with_back(source: ViewSkillSource, back: String) -> Self {
        Self { source, skill_index: None, back: Some(back) }
    }

    fn modify_with_skills<'a>(self, create: CreateReply, iterator: impl Iterator<Item = &'a Skill>, mut embed: CreateEmbed) -> CreateReply {
        let index = self.skill_index.map(usize::from);
        let mut components = Vec::new();

        if let Some(ref back) = self.back {
            components.push(CreateButton::new(back.as_str()).emoji('⏪').label("Back"));
        }

        for (t_index, skill) in iterator.enumerate().take(4) {
            if Some(t_index) == index {
                embed = embed.color(skill.category.color_rgb())
                    .fields(self.create_ex_skill_field(skill));
            } else {
                embed = embed.fields(self.create_skill_field(skill));
            }

            if !skill.barrages.is_empty() {
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

    fn button_with_skill(&self, index: usize) -> CreateButton {
        self.new_button(utils::field!(Self: skill_index), Some(index as u8), || Sentinel::new(1, index as u32))
    }
    
    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);
        self.modify_with_skills(
            create,
            ship.skills.iter(),
            CreateEmbed::new().color(ship.rarity.color_rgb()).author(super::get_ship_url(base_ship))
        )
    }

    pub fn modify_with_augment(self, create: CreateReply, augment: &Augment) -> CreateReply {
        self.modify_with_skills(
            create,
            augment.effect.iter().chain(augment.skill_upgrade.as_ref()),
            CreateEmbed::new().color(ShipRarity::SR.color_rgb()).author(CreateEmbedAuthor::new(&augment.name))
        )
    }

    fn create_skill_field(&self, skill: &Skill) -> [OwnedCreateEmbedField; 1] {
        [(
            format!("{} {}", skill.category.emoji(), skill.name),
            utils::text::truncate(&skill.description, 1000),
            false
        )]
    }

    fn create_ex_skill_field(&self, skill: &Skill) -> [OwnedCreateEmbedField; 2] {
        [
            (
                format!("{} __{}__", skill.category.emoji(), skill.name),
                utils::text::truncate(&skill.description, 1000),
                false
            ),
            (
                "__Barrage__".to_owned(),
                {
                    let m = get_skills_extra_summary(skill);
                    if m.len() <= 1024 { m } else { println!("barrage:\n{m}"); "<barrage data too long>".to_owned() }
                },
                false
            )
        ]
    }
}

impl ButtonArgsModify for ViewSkill {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        match self.source {
            ViewSkillSource::Ship(ship_id, retro_index) => {
                let base_ship = data.azur_lane().ship_by_id(ship_id).ok_or(ShipParseError)?;
                let ship = retro_index.and_then(|i| base_ship.retrofits.get(usize::from(i))).unwrap_or(base_ship);
                Ok(self.modify_with_ship(create, ship, Some(base_ship)))
            }
            ViewSkillSource::Augment(augment_id) => {
                let augment = data.azur_lane().augment_by_id(augment_id).ok_or(AugmentParseError)?;
                Ok(self.modify_with_augment(create, augment))
            }
        }
    }
}

macro_rules! idk {
    ($opt:expr, $($arg:tt)*) => {
        match $opt {
            None => None,
            Some(v) => Some(format!($($arg)*, sum = v))
        }
    };
}

fn get_skills_extra_summary(skill: &Skill) -> String {
    return join("\n\n", skill.barrages.iter().filter_map(get_skill_barrage_summary)).unwrap_or_else(String::new);

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
            )
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
                key.ammo.short_name(), l * 100f32, m * 100f32, h * 100f32,
                barrage.scaling * 100f32, barrage.scaling_stat.name()
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
