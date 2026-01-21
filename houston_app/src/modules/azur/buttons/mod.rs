//! Some small guidelines for the API of buttons here:
//!
//! - Accept `LazyData` instead of re-grabbing it manually from the bot data
//!   unless there is no benefit to it.
//! - Private methods should avoid doing duplicate work and be infallible if
//!   possible -- pass the Config through as an additional parameter if needed
//! - Builder-style methods >> [`Option`] parameter on `new` function

use azur_lane::ship::HullType;

use crate::buttons::prelude::*;

pub mod augment;
pub mod equip;
pub mod juustagram_chat;
pub mod lines;
pub mod search_augment;
pub mod search_equip;
pub mod search_juustagram_chat;
pub mod search_ship;
pub mod search_special_secretary;
pub mod shadow_equip;
pub mod ship;
pub mod skill;
pub mod special_secretary;

#[derive(Debug, thiserror::Error)]
enum AzurParseError {
    #[error("unknown ship")]
    Ship,
    #[error("unknown equip")]
    Equip,
    #[error("unknown augment")]
    Augment,
    #[error("unknown special secretary")]
    SpecialSecretary,
    #[error("unknown juustagram chat")]
    JuustagramChat,
}

/// Tries to get the ship preview name from the message, assuming
/// - the message's first component is a container
/// - whose first component is a section
/// - whose accessory is a thumbnail with the preview.
fn get_ship_preview_name(ctx: ButtonContext<'_>) -> Option<&str> {
    if let Some(Component::Container(container)) = ctx.interaction.message.components.first()
        && let Some(ContainerComponent::Section(section)) = container.components.first()
        && let SectionAccessory::Thumbnail(thumbnail) = &*section.accessory
    {
        get_thumbnail_filename(&thumbnail.media.url)
    } else {
        None
    }
}

fn get_thumbnail_filename(url: &str) -> Option<&str> {
    let (_, name) = url.rsplit_once('/')?;
    Some(name.split_once('.').map_or(name, |a| a.0))
}

pub fn hull_emoji(hull_type: HullType, data: &HBotData) -> &ReactionType {
    let e = data.app_emojis();
    match hull_type {
        HullType::Unknown => e.fallback(),
        HullType::Destroyer => e.hull_dd(),
        HullType::LightCruiser => e.hull_cl(),
        HullType::HeavyCruiser => e.hull_ca(),
        HullType::Battlecruiser => e.hull_bc(),
        HullType::Battleship => e.hull_bb(),
        HullType::LightCarrier => e.hull_cvl(),
        HullType::AircraftCarrier => e.hull_cv(),
        HullType::Submarine => e.hull_ss(),
        HullType::AviationBattleship => e.hull_bbv(),
        HullType::RepairShip => e.hull_ar(),
        HullType::Monitor => e.hull_bm(),
        HullType::AviationSubmarine => e.hull_ssv(),
        HullType::LargeCruiser => e.hull_cb(),
        HullType::MunitionShip => e.hull_ae(),
        HullType::MissileDestroyerV => e.hull_ddgv(),
        HullType::MissileDestroyerM => e.hull_ddgm(),
        HullType::FrigateS => e.hull_ixs(),
        HullType::FrigateV => e.hull_ixv(),
        HullType::FrigateM => e.hull_ixm(),
    }
}

/// Returns an iterator for the elements on this page.
///
/// If there are no results or the page is out of bounds, _returns from the
/// caller_ with an alternate reply.
macro_rules! page_iter {
    ($iter:expr, $page:expr) => {{
        let mut iter = $iter.by_ref().take(PAGE_SIZE).peekable();
        if iter.peek().is_none() {
            return $crate::modules::azur::buttons::pagination_impl::no_results($page);
        }
        iter
    }};
}

/// Appends the page navigation row to the [`ComponentVec`].
macro_rules! page_nav {
    ($components:expr, $obj:expr, $remainder:expr) => {
        if let Some(nav) =
            $crate::modules::azur::buttons::pagination_impl::nav_row(&mut $obj, $remainder, |s| {
                &mut s.page
            })
        {
            $components.push(::serenity::builder::CreateSeparator::new(true));
            $components.push(nav);
        }
    };
}

pub(crate) use {page_iter, page_nav};

mod pagination_impl {
    use super::search::PAGE_SIZE;
    use crate::buttons::prelude::*;
    use crate::modules::core::buttons::ToPage;

    pub fn no_results<'new>(page: u16) -> Result<CreateReply<'new>> {
        if page == 0 {
            let embed = CreateEmbed::new()
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            Ok(CreateReply::new().embed(embed))
        } else {
            Err(HArgError::new("This page has no data.").into())
        }
    }

    pub fn nav_row<'a, T, I, F>(obj: &mut T, remainder: I, page: F) -> Option<CreateActionRow<'a>>
    where
        T: ButtonValue,
        I: Iterator,
        F: Fn(&mut T) -> &mut u16,
    {
        #[expect(clippy::cast_possible_truncation)]
        let page_count = 1 + *page(obj) + remainder.count().div_ceil(PAGE_SIZE) as u16;
        ToPage::build_row(obj, page)
            .exact_page_count(page_count)
            .end()
    }
}

mod search {
    use std::slice::Iter;

    use crate::modules::azur::data::{ByLookupIter, ByPrefixIter};

    pub const PAGE_SIZE: usize = 10;

    pub struct Filtered<'a, T, F> {
        inner: Inner<'a, T>,
        filter: F,
    }

    pub trait Filtering<T> {
        fn is_match(&self, item: &T) -> bool;
    }

    impl<'a, T, F> Filtered<'a, T, F>
    where
        F: Filtering<T>,
    {
        pub fn slice(inner: &'a [T], filter: F) -> Self {
            Self {
                inner: Inner::Slice(inner.iter()),
                filter,
            }
        }

        pub fn by_prefix(inner: ByPrefixIter<'a, T>, filter: F) -> Self {
            Self {
                inner: Inner::ByPrefix(inner),
                filter,
            }
        }

        pub fn by_lookup(inner: ByLookupIter<'a, T>, filter: F) -> Self {
            Self {
                inner: Inner::ByLookup(inner),
                filter,
            }
        }

        pub fn at_page(mut self, page: u16) -> Self {
            let mut skip = PAGE_SIZE * usize::from(page);
            while skip > 0 && self.next().is_some() {
                skip -= 1;
            }

            self
        }
    }

    enum Inner<'a, T> {
        Slice(Iter<'a, T>),
        ByPrefix(ByPrefixIter<'a, T>),
        ByLookup(ByLookupIter<'a, T>),
    }

    impl<'a, T: 'a, F> Iterator for Filtered<'a, T, F>
    where
        F: Filtering<T>,
    {
        type Item = &'a T;

        fn next(&mut self) -> Option<Self::Item> {
            macro_rules! next {
                ($iter:expr) => {
                    while let Some(item) = $iter.next() {
                        if self.filter.is_match(item) {
                            return Some(item);
                        }
                    }
                };
            }

            match &mut self.inner {
                Inner::Slice(it) => next!(it),
                Inner::ByPrefix(it) => next!(it),
                Inner::ByLookup(it) => next!(it),
            }
            None
        }

        fn fold<B, L>(self, init: B, mut f: L) -> B
        where
            L: FnMut(B, Self::Item) -> B,
        {
            let filter = &self.filter;
            let fold_inner = move |acc, item| {
                if filter.is_match(item) {
                    f(acc, item)
                } else {
                    acc
                }
            };

            match self.inner {
                Inner::Slice(it) => it.fold(init, fold_inner),
                Inner::ByPrefix(it) => it.fold(init, fold_inner),
                Inner::ByLookup(it) => it.fold(init, fold_inner),
            }
        }
    }

    pub struct All;
    impl<T> Filtering<T> for All {
        fn is_match(&self, _item: &T) -> bool {
            true
        }
    }
}
