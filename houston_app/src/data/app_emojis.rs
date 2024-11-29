use std::sync::LazyLock;

use serenity::http::Http;

use azur_lane::ship::HullType;

use super::HBotConfig;
use crate::modules::Module;
use crate::prelude::*;

macro_rules! generate {
    ({ $($key:ident = $name:literal $(if $condition:expr)?;)* }) => {
        #[derive(Debug)]
        pub struct HAppEmojiStore {
            $(pub $key: ReactionType,)*
        }

        #[derive(Debug, Clone, Copy)]
        pub struct HAppEmojis<'a>(pub(super) Option<&'a HAppEmojiStore>);

        #[allow(dead_code, reason = "macro generated in bulk")]
        impl<'a> HAppEmojis<'a> {
            $(
                #[must_use]
                pub fn $key(self) -> &'a ReactionType {
                    match self.0 {
                        Some(e) => &e.$key,
                        None => &FALLBACK_EMOJI
                    }
                }
            )*
        }

        impl HAppEmojiStore {
            pub async fn load_and_update(config: &HBotConfig, ctx: &Http) -> Result<HAppEmojiStore> {
                let emojis = load_emojis(ctx).await.context("failed to load app emojis")?;

                struct Temp {
                    $($key: Option<ReactionType>,)*
                }

                let mut exist = Temp {
                    $($key: None,)*
                };

                for emoji in emojis {
                    match emoji.name.as_str() {
                        $($name => exist.$key = Some(emoji.into()),)*
                        _ => (),
                    }
                }

                Ok(Self {
                    $(
                        $key: match exist.$key {
                            Some(e) => e,
                            $( None if !$condition(config) => FALLBACK_EMOJI.clone(), )?
                            None => update_emoji(ctx, $name, include_bytes!(concat!("../../assets/emojis/", $name, ".png"))).await?,
                        },
                    )*
                })
            }
        }
    };
}

fn azur(config: &HBotConfig) -> bool {
    crate::modules::azur::Module.enabled(config)
}

generate!({
    hull_dd   = "Hull_DD"   if azur;
    hull_cl   = "Hull_CL"   if azur;
    hull_ca   = "Hull_CA"   if azur;
    hull_bc   = "Hull_BC"   if azur;
    hull_bb   = "Hull_BB"   if azur;
    hull_cvl  = "Hull_CVL"  if azur;
    hull_cv   = "Hull_CV"   if azur;
    hull_ss   = "Hull_SS"   if azur;
    hull_bbv  = "Hull_BBV"  if azur;
    hull_ar   = "Hull_AR"   if azur;
    hull_bm   = "Hull_BM"   if azur;
    hull_ssv  = "Hull_SSV"  if azur;
    hull_cb   = "Hull_CB"   if azur;
    hull_ae   = "Hull_AE"   if azur;
    hull_ddgv = "Hull_DDGv" if azur;
    hull_ddgm = "Hull_DDGm" if azur;
    hull_ixs  = "Hull_IXs"  if azur;
    hull_ixv  = "Hull_IXv"  if azur;
    hull_ixm  = "Hull_IXm"  if azur;
});

static FALLBACK_EMOJI: LazyLock<ReactionType> = LazyLock::new(|| ReactionType::from('❔'));

async fn load_emojis(ctx: &Http) -> Result<Vec<Emoji>> {
    Ok(ctx.get_application_emojis().await?)
}

#[inline(never)]
async fn update_emoji(ctx: &Http, name: &str, image_data: &[u8]) -> Result<ReactionType> {
    let map = serde_json::json!({
        "name": name,
        "image": png_to_data_url(image_data),
    });

    let emoji = ctx.create_application_emoji(&map).await?;

    log::info!("Added Application Emoji: {}", emoji);
    Ok(emoji.into())
}

fn png_to_data_url(png: &[u8]) -> String {
    use base64::prelude::*;

    let mut res = String::new();
    res.push_str("data:image/png;base64,");
    BASE64_STANDARD.encode_string(png, &mut res);

    res
}

impl<'a> HAppEmojis<'a> {
    #[must_use]
    pub fn hull(self, hull_type: HullType) -> &'a ReactionType {
        let Some(s) = self.0 else {
            return &FALLBACK_EMOJI
        };

        match hull_type {
            HullType::Unknown => &FALLBACK_EMOJI,
            HullType::Destroyer => &s.hull_dd,
            HullType::LightCruiser => &s.hull_cl,
            HullType::HeavyCruiser => &s.hull_ca,
            HullType::Battlecruiser => &s.hull_bc,
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
