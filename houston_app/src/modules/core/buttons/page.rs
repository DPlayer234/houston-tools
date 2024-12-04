use utils::fields::FieldMut;

use crate::buttons::prelude::*;

/// Opens a modal for page navigation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToPage(CustomData);

impl ToPage {
    /// Opens a modal for page navigation.
    pub fn new(data: CustomData) -> Self {
        Self(data)
    }

    pub fn set_page_from(page: &mut u16, interaction: &ModalInteraction) {
        if let Some(new_page) = Self::get_page(interaction) {
            *page = new_page;
        }
    }

    pub fn get_page(interaction: &ModalInteraction) -> Option<u16> {
        let component = interaction
            .data
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
        (1..=9999).contains(&page).then_some(page - 1)
    }

    pub fn build_row<T, F>(
        obj: &mut T,
        page_field: F,
    ) -> PageRowBuilder<'_, T, F>
    where
        T: ToCustomData,
        F: FieldMut<T, u16>,
    {
        PageRowBuilder {
            obj,
            page_field,
            max_page: MaxPage::NoMore,
        }
    }
}

#[derive(Debug)]
pub struct PageRowBuilder<'a, T, F> {
    obj: &'a mut T,
    page_field: F,
    max_page: MaxPage,
}

#[derive(Debug)]
enum MaxPage {
    NoMore,
    Exact(u16),
    Minimum(u16),
}

impl<T, F> PageRowBuilder<'_, T, F>
where
    T: ToCustomData,
    F: FieldMut<T, u16>,
{
    pub fn exact_page_count(mut self, pages: u16) -> Self {
        self.max_page = MaxPage::Exact(pages);
        self
    }

    pub fn min_page_count(mut self, pages: u16) -> Self {
        self.max_page = MaxPage::Minimum(pages);
        self
    }

    pub fn auto_page_count(self, pages: u16, has_more: bool, max_show: u16) -> Self {
        if pages <= max_show {
            self.exact_page_count(pages)
        } else if has_more {
            let page = *self.page_field.get(self.obj);
            self.min_page_count(max_show.max(page + 1))
        } else {
            let page = *self.page_field.get(self.obj);
            self.exact_page_count(page + 1)
        }
    }

    pub fn end<'new>(self) -> Option<CreateActionRow<'new>> {
        let page = *self.page_field.get(self.obj);

        let has_more = match self.max_page {
            MaxPage::NoMore => false,
            MaxPage::Exact(e) => e > page + 1,
            MaxPage::Minimum(_) => true,
        };

        (page > 0 || has_more).then(move || CreateActionRow::buttons(vec![
            if page > 0 {
                self.obj.new_button(&self.page_field, page - 1, |_| 1)
            } else {
                CreateButton::new("#no-back").disabled(true)
            }.emoji('◀'),

            CreateButton::new(ToPage::new(self.obj.to_custom_data()).to_custom_id())
                .label(match self.max_page {
                    MaxPage::NoMore => format!("{0} / {0}", page + 1),
                    MaxPage::Exact(max) => format!("{} / {}", page + 1, max),
                    MaxPage::Minimum(min) => format!("{} / {}+", page + 1, min),
                }),

            if has_more {
                self.obj.new_button(&self.page_field, page + 1, |_| 2)
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

        let custom_id = self.0.to_custom_id();
        let modal = CreateModal::new(custom_id, "Go to page...")
            .components(components);

        ctx.modal(modal).await
    }
}