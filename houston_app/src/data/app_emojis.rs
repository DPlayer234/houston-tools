use azur_lane::ship::HullType;
use once_cell::sync::Lazy;
use serenity::all::{Http, EmojiIdentifier, ReactionType};
use crate::prelude::*;

async fn update_emoji(_ctx: &Http, _image_data: &[u8]) -> HResult<ReactionType> {
    // todo: update for app
    Ok(DEFAULT_REACTION_TYPE.clone())
}

static DEFAULT_REACTION_TYPE: Lazy<ReactionType> = Lazy::new(|| ReactionType::from('❔'));

macro_rules! generate {
    ($($emoji:ident),* $(,)?) => {
        #[derive(Debug)]
        pub struct HAppEmojiStore {
            $(pub $emoji: ReactionType,)*
        }

        #[derive(Debug, Clone, Copy)]
        pub struct HAppEmojis<'a>(pub(super) Option<&'a HAppEmojiStore>);

        #[allow(dead_code)]
        impl<'a> HAppEmojis<'a> {
            $(
                #[must_use]
                pub fn $emoji(self) -> &'a ReactionType {
                    match self.0 {
                        Some(e) => &e.$emoji,
                        None => &*DEFAULT_REACTION_TYPE
                    }
                }
            )*
        }

        impl HAppEmojiStore {
            pub async fn load_and_update(ctx: &Http) -> HResult<HAppEmojiStore> {
                // todo: request from context
                let emojis: Vec<EmojiIdentifier> = vec![
                    "<:hull_dd:1265681972139659449>".parse()?
                ];

                struct Temp {
                    $($emoji: Option<ReactionType>,)*
                }

                let mut exist = Temp {
                    $($emoji: None,)*
                };

                for emoji in emojis {
                    match emoji.name.as_str() {
                        $(stringify!($emoji) => exist.$emoji = Some(emoji.into()),)*
                        _ => (),
                    }
                }

                Ok(Self {
                    $(
                        $emoji: match exist.$emoji {
                            Some(e) => e,
                            // todo: actual include should be:
                            // include_bytes!(concat!("../../assets/", stringify!($emoji), ".png"))
                            None => update_emoji(ctx, b"todo").await?
                        },
                    )*
                })
            }
        }
    };
}

generate!(
    hull_dd,
    hull_cl,
    hull_ca,
    hull_bb,
    hull_cvl,
    hull_cv,
    hull_ss,
    hull_bbv,
    hull_ar,
    hull_bm,
    hull_ssv,
    hull_cb,
    hull_ae,
    hull_ddgv,
    hull_ddgm,
    hull_ixs,
    hull_ixv,
    hull_ixm,
);

impl<'a> HAppEmojis<'a> {
    pub fn hull(self, hull_type: HullType) -> &'a ReactionType {
        let Some(s) = self.0 else {
            return &*DEFAULT_REACTION_TYPE
        };

        match hull_type {
            HullType::Unknown => &*DEFAULT_REACTION_TYPE,
            HullType::Destroyer => &s.hull_dd,
            HullType::LightCruiser => &s.hull_cl,
            HullType::HeavyCruiser => &s.hull_ca,
            HullType::Battlecruiser => &s.hull_cb,
            HullType::Battleship => &s.hull_bb,
            HullType::LightCarrier => &s.hull_cvl,
            HullType::AircraftCarrier => &s.hull_cv,
            HullType::Submarine => &s.hull_ss,
            HullType::AviationBattleship => &s.hull_bbv,
            HullType::RepairShip => &s.hull_ar,
            HullType::Monitor => &s.hull_bm,
            HullType::AviationSubmarine => &s.hull_ssv,
            HullType::LargeCruiser => &s.hull_cb,
            HullType::MunitionShip => &s.hull_ae,
            HullType::MissileDestroyerV => &s.hull_ddgv,
            HullType::MissileDestroyerM => &s.hull_ddgm,
            HullType::FrigateS => &s.hull_ixs,
            HullType::FrigateV => &s.hull_ixv,
            HullType::FrigateM => &s.hull_ixm,
        }
    }
}
