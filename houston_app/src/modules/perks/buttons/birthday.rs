use bson::doc;
use mongodb::options::ReturnDocument;

use crate::buttons::prelude::*;
use crate::helper::bson::bson_id;
use crate::modules::perks::day_of_year::DayOfYear;
use crate::modules::perks::model::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Set {
    day_of_year: DayOfYear,
}

impl Set {
    pub fn new(day_of_year: DayOfYear) -> Self {
        Self { day_of_year }
    }
}

impl ButtonArgsReply for Set {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let user_id = ctx.interaction.user.id;

        ctx.acknowledge().await?;

        let db = ctx.data.database()?;

        let filter = doc! {
            "user": bson_id!(user_id),
        };

        let update = doc! {
            "$setOnInsert": {
                "user": bson_id!(user_id),
                "day_of_year": bson::ser::to_bson(&self.day_of_year)?,
            },
        };

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
