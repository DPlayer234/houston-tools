use std::num::NonZero;
use std::str::FromStr;

use rand::distributions::Uniform;
use rand::prelude::*;
use smallvec::SmallVec;

use utils::text::write_str::*;

use crate::prelude::*;
use crate::slashies::create_reply;

/// Rolls some dice.
#[poise::command(slash_command)]
pub async fn dice(
    ctx: HContext<'_>,
    #[description = "The sets of dice to roll, in a format like '2d6', separated by spaces."]
    sets: DiceSetVec,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let sets = sets.as_slice();
    let dice_count: u32 = sets.iter().map(|d| u32::from(d.count.get())).sum();
    if dice_count > 255 {
        Err(HArgError::new("You can't roll more than 255 dice at once."))?;
    }

    let (total_sum, content) = get_dice_roll_result(sets);
    let embed = CreateEmbed::new()
        .title(format!("Total \u{2211}{}", total_sum))
        .description(content)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(create_reply(ephemeral).embed(embed)).await?;
    Ok(())
}

fn get_dice_roll_result(sets: &[DiceSet]) -> (u32, String) {
    let mut content = String::new();
    let mut rng = thread_rng();

    // 32 bits are enough (max allowed input is 255*65535)
    // so we won't ever exceed the needed space
    let mut total_sum = 0u32;

    for &d in sets {
        write_str!(content, "- **{}d{}:**", d.count, d.faces);

        let sample = Uniform::new_inclusive(1, u32::from(d.faces.get()));
        let mut local_sum = 0u32;
        for _ in 0..d.count.get() {
            let roll = rng.sample(sample);
            local_sum += roll;

            write_str!(content, " {}", roll);
        }

        if d.count.get() > 1 && sets.len() > 1 {
            write_str!(content, " *(\u{2211}{})*", local_sum);
        }

        total_sum += local_sum;
        content.push('\n');
    }

    (total_sum, content)
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Expected inputs like '2d6' or '1d20 2d4'. The maximum is '255d65535'.")]
struct DiceParseError(());

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

type InnerVec = SmallVec<[DiceSet; 4]>;

#[derive(Debug)]
struct DiceSetVec(InnerVec);

impl DiceSetVec {
    #[must_use]
    fn from_vec(vec: InnerVec) -> Option<Self> {
        (!vec.is_empty()).then_some(Self(vec))
    }

    #[must_use]
    fn as_slice(&self) -> &[DiceSet] {
        self.0.as_slice()
    }
}

impl FromStr for DiceSetVec {
    type Err = DiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .map(DiceSet::from_str)
            .collect::<Result<InnerVec, Self::Err>>()
            .and_then(|v| Self::from_vec(v).ok_or(DiceParseError(())))
    }
}
