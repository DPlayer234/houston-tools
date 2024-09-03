use std::num::NonZero;
use std::str::FromStr;
use std::fmt::Write;

use rand::{thread_rng, Rng};
use rand::distributions::Uniform;

use utils::Discard;

use crate::prelude::*;

/// Rolls some dice.
#[poise::command(slash_command)]
pub async fn dice(
    ctx: HContext<'_>,
    #[description = "The sets of dice to roll, in a format like '2d6', separated by spaces."]
    sets: DiceSetVec,
) -> HResult {
    let sets = sets.into_vec();
    let dice_count: u32 = sets.iter().map(|d| u32::from(d.count.get())).sum();
    if dice_count > 255 {
        Err(HArgError("You can't roll more than 255 dice at once."))?;
    }

    let (total_sum, content) = get_dice_roll_result(sets);
    let embed = CreateEmbed::new()
        .title(format!("Total \u{2211}{}", total_sum))
        .description(content)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

fn get_dice_roll_result(sets: Vec<DiceSet>) -> (u32, String) {
    let mut content = String::new();
    let mut rng = thread_rng();

    // 32 bits are enough (max allowed input is 255*65535)
    // so we won't ever exceed the needed space
    let mut total_sum = 0u32;

    let len = sets.len();
    for d in sets {
        write!(content, "- **{}d{}:**", d.count, d.faces).discard();

        let sample = Uniform::new_inclusive(1, u32::from(d.faces.get()));
        let mut local_sum = 0u32;
        for _ in 0..d.count.get() {
            let roll = rng.sample(sample);
            local_sum += roll;

            write!(content, " {}", roll).discard();
        }

        if d.count.get() > 1 && len > 1 {
            write!(content, " *(\u{2211}{})*", local_sum).discard();
        }

        total_sum += local_sum;
        content.push('\n');
    }

    (total_sum, content)
}

utils::define_simple_error!(
    #[derive(Clone, Copy)]
    DiceParseError(()):
    "Expected inputs like '2d6' or '1d20 2d4'. The maximum is '255d65535'."
);

#[derive(Debug, Clone, Copy)]
#[repr(align(4))]
struct DiceSet {
    count: NonZero<u8>,
    faces: NonZero<u16>
}

impl FromStr for DiceSet {
    type Err = DiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_inner(args: (&str, &str)) -> Option<DiceSet> {
            let count = NonZero::from_str(args.0).ok()?;
            let faces = NonZero::from_str(args.1).ok()?;
            Some(DiceSet { count, faces })
        }

        s.split_once(['d', 'D'])
            .and_then(parse_inner)
            .ok_or(DiceParseError(()))
    }
}

#[derive(Debug)]
struct DiceSetVec(Vec<DiceSet>);

impl DiceSetVec {
    #[must_use]
    fn from_vec(vec: Vec<DiceSet>) -> Option<Self> {
        (!vec.is_empty()).then_some(Self(vec))
    }

    #[must_use]
    fn into_vec(self) -> Vec<DiceSet> {
        self.0
    }
}

impl FromStr for DiceSetVec {
    type Err = DiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .map(DiceSet::from_str)
            .collect::<Result<Vec<DiceSet>, Self::Err>>()
            .and_then(|v| DiceSetVec::from_vec(v).ok_or(DiceParseError(())))
    }
}
