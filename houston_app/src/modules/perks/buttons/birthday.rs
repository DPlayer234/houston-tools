use mongodb::options::ReturnDocument;

use crate::buttons::prelude::*;
use crate::modules::perks::DayOfYear;
use crate::modules::perks::model::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Set {
    day_of_year: DayOfYear,
    region: u16,
}

impl Set {
    pub fn new(day_of_year: DayOfYear, region: u16) -> Self {
        Self {
            day_of_year,
            region,
        }
    }
}

button_value!(Set, 15);
impl ButtonReply for Set {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let user_id = ctx.interaction.user.id;

        ctx.acknowledge().await?;

        let db = ctx.data.database()?;

        let filter = Birthday::filter().user(user_id).into_document()?;

        let update = Birthday::update()
            .set_on_insert(|b| b.region(self.region).day_of_year(self.day_of_year))
            .into_document()?;

        let birthday = Birthday::collection(db)
            .find_one_and_update(filter, update)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await?;

        if let Some(birthday) = birthday {
            let msg = format!(
                "You already confirmed your birthday as **{}**.",
                birthday.day_of_year
            );
            return Err(HArgError::new(msg).into());
        }

        let description = format!("Set your birthday to **{}**!", self.day_of_year);

        let embed = CreateEmbed::new()
            .description(description)
            .color(ctx.data.config().embed_color);

        let reply = EditReply::new().embed(embed).components(&[]);

        ctx.edit(reply).await?;
        Ok(())
    }
}
