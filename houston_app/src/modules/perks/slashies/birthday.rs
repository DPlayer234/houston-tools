use chrono::*;
use mongodb::options::ReturnDocument;

use crate::buttons::ToCustomData;
use crate::modules::perks::config::BirthdayRegionConfig;
use crate::modules::perks::model::*;
use crate::modules::perks::DayOfYear;
use crate::slashies::prelude::*;

/// Manage your birthday.
#[chat_command(contexts = "Guild | BotDm", integration_types = "Guild")]
pub mod birthday {
    /// Add your birthday.
    #[sub_command]
    async fn add(
        ctx: Context<'_>,
        /// The month.
        month: EMonth,
        /// The day of the month.
        day: u8,
        /// Which time zone region to use.
        #[autocomplete = "autocomplete_region"]
        region: Option<u16>,
    ) -> Result {
        let data = ctx.data_ref();
        let db = data.database()?;

        ctx.defer_as(Ephemeral).await?;

        let filter = Birthday::filter().user(ctx.user().id).into_document()?;

        let birthday = Birthday::collection(db).find_one(filter).await?;

        if let Some(birthday) = birthday {
            let msg = format!(
                "You already set your birthday to **{}**.",
                birthday.day_of_year
            );
            return Err(HArgError::new(msg).into());
        }

        let region = region.unwrap_or(0);
        _ = get_region(ctx, region)?;

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
            CreateButton::new(Set::new(day_of_year, region).to_custom_id())
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

    /// Checks your set birthday.
    #[sub_command]
    async fn check(ctx: Context<'_>) -> Result {
        let data = ctx.data_ref();
        let db = data.database()?;

        ctx.defer_as(Ephemeral).await?;

        let filter = Birthday::filter().user(ctx.user().id).into_document()?;

        let birthday = Birthday::collection(db).find_one(filter).await?;

        let Some(birthday) = birthday else {
            let command_id = ctx.interaction.data.id;
            let msg = format!(
                "Your birthday isn't set.\n\
                 Add it with: </birthday add:{command_id}>"
            );
            return Err(HArgError::new(msg).into());
        };

        let day_of_year = birthday.day_of_year;
        let region_name = get_region(ctx, birthday.region).map_or("<unknown>", |r| &r.name);

        let description = format!(
            "**Birthday:** {day_of_year}\n\
             **Region:** {region_name}"
        );

        let embed = CreateEmbed::new()
            .description(description)
            .color(data.config().embed_color);

        let reply = CreateReply::new().embed(embed);

        ctx.send(reply).await?;
        Ok(())
    }

    /// Sets your birthday time zone.
    #[sub_command(name = "time-zone")]
    async fn time_zone(
        ctx: Context<'_>,
        /// Which time zone region to use.
        #[autocomplete = "autocomplete_region"]
        region: u16,
    ) -> Result {
        let data = ctx.data_ref();
        let db = data.database()?;

        ctx.defer_as(Ephemeral).await?;

        let region_info = get_region(ctx, region)?;

        let filter = Birthday::filter().user(ctx.user().id).into_document()?;

        let update = Birthday::update()
            .set(|b| b.region(region))
            .into_document()?;

        let birthday = Birthday::collection(db)
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .await?;

        if birthday.is_none() {
            let command_id = ctx.interaction.data.id;
            let msg = format!("Please add a birthday first: </birthday add:{command_id}>");
            return Err(HArgError::new(msg).into());
        }

        let description = format!("Set your region to **{}**.", region_info.name);

        let embed = CreateEmbed::new()
            .color(data.config().embed_color)
            .description(description);

        let reply = CreateReply::new().embed(embed);
        ctx.send(reply).await?;
        Ok(())
    }
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

fn get_region(ctx: Context<'_>, region: u16) -> Result<&BirthdayRegionConfig> {
    let region = ctx
        .data_ref()
        .config()
        .perks()?
        .birthday
        .as_ref()
        .context("birthday feature must be enabled")?
        .regions
        .get(usize::from(region))
        .ok_or(HArgError::new_const("That region is invalid."))?;

    Ok(region)
}

async fn autocomplete_region<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    let regions: Vec<_> = ctx
        .data_ref()
        .config()
        .perks
        .as_ref()
        // flatten the options and vecs down into one iterator
        .into_iter()
        .filter_map(|p| p.birthday.as_ref())
        .flat_map(|p| &p.regions)
        .enumerate()
        // filter to ones whose name contains the input
        // if the input is empty, that's all of them
        .filter(|(_, region)| region.name.contains(partial))
        // map it to an autocomplete choice with the region index as the value
        .map(|(index, region)| {
            AutocompleteChoice::new(&region.name, AutocompleteValue::Integer(index as u64))
        })
        .collect();

    CreateAutocompleteResponse::new().set_choices(regions)
}
