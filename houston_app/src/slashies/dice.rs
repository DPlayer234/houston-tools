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
    #[description = "The sets of dice to roll, in a format like '2d6'."]
    sets: DiceSetVec,
) -> HResult {
    let sets = sets.into_vec();
    let dice_count: u32 = sets.iter().map(|d| u32::from(d.count.get())).sum();
    if dice_count > 255 {
        Err(HArgError("Too many dice in total."))?;
    }

    let content = get_dice_roll_result(sets);
    let embed = CreateEmbed::new()
        .description(content)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

fn get_dice_roll_result(sets: Vec<DiceSet>) -> String {
    let mut content = String::new();
    let mut rng = thread_rng();

    // Sum into u64 to avoid overflow risk
    let mut total_sum = 0u64;

    let len = sets.len();
    for d in sets {
        write!(content, "- **{}:**", d).discard();

        let sample = Uniform::new_inclusive(1u16, d.faces.get());
        let mut local_sum = 0u32;
        for _ in 0..d.count.get() {
            let roll = rng.sample(sample);
            local_sum += u32::from(roll);

            write!(content, " {}", roll).discard();
        }

        if d.count.get() > 1 && len > 1 {
            write!(content, " *(\u{2211}{})*", local_sum).discard();
        }

        total_sum += u64::from(local_sum);
        content.push('\n');
    }

    let header = format!("### Total \u{2211}{}\n", total_sum);
    content.insert_str(0, &header);

    content
}

#[derive(Debug, Clone, Copy)]
struct DiceSet {
    pub count: NonZero<u8>,
    pub faces: NonZero<u16>
}

#[derive(Debug, Clone)]
struct DiceSetVec(Vec<DiceSet>);

#[derive(Debug, Clone, Copy)]
struct DiceParseError;

impl DiceSet {
    #[must_use]
    fn new(count: NonZero<u8>, faces: NonZero<u16>) -> Self {
        Self { count, faces }
    }
}

impl FromStr for DiceSet {
    type Err = DiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_inner(args: (&str, &str)) -> Option<DiceSet> {
            let (l, r) = args;
            let l = NonZero::from_str(l).ok()?;
            let r = NonZero::from_str(r).ok()?;
            Some(DiceSet::new(l, r))
        }

        s.split_once(['d', 'D'])
            .and_then(parse_inner)
            .ok_or(DiceParseError)
    }
}

impl std::fmt::Display for DiceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}d{}", self.count.get(), self.faces.get())
    }
}

impl DiceSetVec {
    #[must_use]
    fn from_vec(vec: Vec<DiceSet>) -> Self {
        Self(vec)
    }

    #[must_use]
    fn into_vec(self) -> Vec<DiceSet> {
        self.0
    }
}

impl FromStr for DiceSetVec {
    type Err = DiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn map_vector(v: Vec<DiceSet>) -> Result<DiceSetVec, DiceParseError> {
            if v.is_empty() {
                Err(DiceParseError)
            } else {
                Ok(DiceSetVec::from_vec(v))
            }
        }

        s.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .map(DiceSet::from_str)
            .collect::<Result<Vec<DiceSet>, Self::Err>>()
            .and_then(map_vector)
    }
}

impl std::fmt::Display for DiceSetVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for d in &self.0 {
            if first {
                f.write_str(" ")?;
                first = false;
            }

            write!(f, "{}", d)?;
        }

        Ok(())
    }
}

impl std::error::Error for DiceParseError {}

impl std::fmt::Display for DiceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expected inputs like '2d6'. The maximum is '255d65535'.")
    }
}
