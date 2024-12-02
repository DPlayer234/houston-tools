use crate::buttons::prelude::*;

/// Opens a modal for page navigation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToPage {
    data: CustomData,
}

impl ToPage {
    /// Opens a modal for page navigation.
    pub fn new(data: CustomData) -> Self {
        Self { data }
    }

    pub fn load_page(page: &mut u16, interaction: ButtonInteraction<'_>) {
        if let Some(new_page) = Self::get_page(interaction) {
            *page = new_page;
        }
    }

    pub fn get_page(interaction: ButtonInteraction<'_>) -> Option<u16> {
        let component = interaction.modal_data()?
            .components.first()?
            .components.first()?;

        let ActionRowComponent::InputText(InputText {
            value: Some(value),
            custom_id,
            ..
        }) = component else {
            return None;
        };

        if custom_id.as_str() != "page" {
            return None;
        }

        let page: u16 = value.parse().ok()?;
        page.checked_sub(1)
    }


    pub fn get_pagination_buttons<'new, T: ToCustomData>(
        obj: &mut T,
        page_field: impl utils::fields::FieldMut<T, u16>,
        has_next: bool,
    ) -> Option<CreateActionRow<'new>> {
        let page = *page_field.get(obj);
        (page > 0 || has_next).then(move || CreateActionRow::buttons(vec![
            if page > 0 {
                obj.new_button(&page_field, page - 1, |_| 1)
            } else {
                CreateButton::new("#no-back").disabled(true)
            }.emoji('◀'),

            CreateButton::new(Self::new(obj.to_custom_data()).to_custom_id())
                .label((page + 1).to_string()),

            if has_next {
                obj.new_button(&page_field, page + 1, |_| 2)
            } else {
                CreateButton::new("#no-forward").disabled(true)
            }.emoji('▶'),
        ]))
    }
}

impl ButtonArgsReply for ToPage {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let input_text = CreateInputText::new(InputTextStyle::Short, "Page", "page")
            .min_length(1)
            .max_length(4)
            .placeholder("Enter page...")
            .required(true);

        let components = vec![
            CreateActionRow::input_text(input_text),
        ];

        let modal = CreateModal::new(self.data.to_custom_id(), "Go to page...")
            .components(components);

        let create = CreateInteractionResponse::Modal(modal);

        ctx.reply(create).await
    }
}
