use std::sync::LazyLock;

use azur_lane::ship::HullType;
use serenity::http::Http;

use super::HBotConfig;
use crate::modules::Module;
use crate::prelude::*;

macro_rules! generate {
    ({ $($key:ident = $name:literal, $path:literal $(if $condition:expr)?;)* }) => {
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
                            None => update_emoji(ctx, $name, include_bytes!(concat!("../../assets/emojis/", $path))).await?,
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
    empty = "Empty", "Empty.png";

    chess_white_pawn   = "Chess_WhitePawn",   "chess/WhitePawn.png";
    chess_white_rook   = "Chess_WhiteRook",   "chess/WhiteRook.png";
    chess_white_bishop = "Chess_WhiteBishop", "chess/WhiteBishop.png";
    chess_white_knight = "Chess_WhiteKnight", "chess/WhiteKnight.png";
    chess_white_queen  = "Chess_WhiteQueen",  "chess/WhiteQueen.png";
    chess_white_king   = "Chess_WhiteKing",   "chess/WhiteKing.png";
    chess_black_pawn   = "Chess_BlackPawn",   "chess/BlackPawn.png";
    chess_black_rook   = "Chess_BlackRook",   "chess/BlackRook.png";
    chess_black_bishop = "Chess_BlackBishop", "chess/BlackBishop.png";
    chess_black_knight = "Chess_BlackKnight", "chess/BlackKnight.png";
    chess_black_queen  = "Chess_BlackQueen",  "chess/BlackQueen.png";
    chess_black_king   = "Chess_BlackKing",   "chess/BlackKing.png";

    hull_dd   = "Hull_DD",   "azur/Hull_DD.png"   if azur;
    hull_cl   = "Hull_CL",   "azur/Hull_CL.png"   if azur;
    hull_ca   = "Hull_CA",   "azur/Hull_CA.png"   if azur;
    hull_bc   = "Hull_BC",   "azur/Hull_BC.png"   if azur;
    hull_bb   = "Hull_BB",   "azur/Hull_BB.png"   if azur;
    hull_cvl  = "Hull_CVL",  "azur/Hull_CVL.png"  if azur;
    hull_cv   = "Hull_CV",   "azur/Hull_CV.png"   if azur;
    hull_ss   = "Hull_SS",   "azur/Hull_SS.png"   if azur;
    hull_bbv  = "Hull_BBV",  "azur/Hull_BBV.png"  if azur;
    hull_ar   = "Hull_AR",   "azur/Hull_AR.png"   if azur;
    hull_bm   = "Hull_BM",   "azur/Hull_BM.png"   if azur;
    hull_ssv  = "Hull_SSV",  "azur/Hull_SSV.png"  if azur;
    hull_cb   = "Hull_CB",   "azur/Hull_CB.png"   if azur;
    hull_ae   = "Hull_AE",   "azur/Hull_AE.png"   if azur;
    hull_ddgv = "Hull_DDGv", "azur/Hull_DDGv.png" if azur;
    hull_ddgm = "Hull_DDGm", "azur/Hull_DDGm.png" if azur;
    hull_ixs  = "Hull_IXs",  "azur/Hull_IXs.png"  if azur;
    hull_ixv  = "Hull_IXv",  "azur/Hull_IXv.png"  if azur;
    hull_ixm  = "Hull_IXm",  "azur/Hull_IXm.png"  if azur;
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
    use base64::engine::Config;
    use base64::prelude::*;

    const PREFIX: &str = "data:image/png;base64,";

    let engine = &BASE64_STANDARD;
    let size = base64::encoded_len(png.len(), engine.config().encode_padding())
        .and_then(|s| s.checked_add(PREFIX.len()))
        .expect("base64 emoji images should fit into memory");

    let mut res = String::with_capacity(size);
    res.push_str(PREFIX);
    engine.encode_string(png, &mut res);

    res
}

impl<'a> HAppEmojis<'a> {
    #[must_use]
    pub fn hull(self, hull_type: HullType) -> &'a ReactionType {
        let Some(s) = self.0 else {
            return &FALLBACK_EMOJI;
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
