use bson::doc;
use chrono::*;

use crate::buttons::ToCustomData;
use crate::helper::bson::bson_id;
use crate::modules::perks::model::*;
use crate::modules::perks::DayOfYear;
use crate::slashies::prelude::*;

/// Sets your birthday.
#[chat_command(contexts = "Guild | BotDm", integration_types = "Guild")]
pub async fn birthday(
    ctx: Context<'_>,
    /// The month.
    month: EMonth,
    /// The day of the month.
    day: u8,
) -> Result {
    let data = ctx.data_ref();
    let db = data.database()?;

    ctx.defer_as(Ephemeral).await?;

    let filter = doc! {
        "user": bson_id!(ctx.user().id),
    };

    let birthday = Birthday::collection(db).find_one(filter).await?;

    if let Some(birthday) = birthday {
        let msg = format!(
            "You already set your birthday to **{}**.",
            birthday.day_of_year
        );
        return Err(HArgError::new(msg).into());
    }

    let day_of_year = DayOfYear::from_md(month.convert(), day)
        .ok_or(HArgError::new_const("That date is not valid."))?;

    let description = format!(
        "Confirm that this is your birthday:\n\
         - **{day_of_year}**\n\
         -# You will receive some presents on this day.\n\
         -# You cannot change this later."
    );

    let embed = CreateEmbed::new()
        .description(description)
        .color(ERROR_EMBED_COLOR);

    use crate::modules::core::buttons::Delete;
    use crate::modules::perks::buttons::birthday::Set;

    let components = CreateActionRow::buttons(vec![
        CreateButton::new(Set::new(day_of_year).to_custom_id())
            .label("Confirm")
            .style(ButtonStyle::Success),
        CreateButton::new(Delete.to_custom_id())
            .label("Cancel")
            .style(ButtonStyle::Danger),
    ]);

    let reply = CreateReply::new().embed(embed).components(vec![components]);

    ctx.send(reply).await?;
    Ok(())
}

#[derive(houston_cmd::ChoiceArg)]
enum EMonth {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl EMonth {
    const fn convert(self) -> Month {
        match self {
            Self::January => Month::January,
            Self::February => Month::February,
            Self::March => Month::March,
            Self::April => Month::April,
            Self::May => Month::May,
            Self::June => Month::June,
            Self::July => Month::July,
            Self::August => Month::August,
            Self::September => Month::September,
            Self::October => Month::October,
            Self::November => Month::November,
            Self::December => Month::December,
        }
    }
}
