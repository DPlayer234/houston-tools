use azur_lane::ship::HullType;
use once_cell::sync::Lazy;
use serenity::all::{Http, ReactionType};

use super::HBotConfig;

macro_rules! generate {
    ({ $($key:ident = $name:literal $(if $condition:path)?;)* }) => {
        #[derive(Debug)]
        pub struct HAppEmojiStore {
            $(pub $key: ReactionType,)*
        }

        #[derive(Debug, Clone, Copy)]
        pub struct HAppEmojis<'a>(pub(super) Option<&'a HAppEmojiStore>);

        #[allow(dead_code)]
        impl<'a> HAppEmojis<'a> {
            $(
                #[must_use]
                pub fn $key(self) -> &'a ReactionType {
                    match self.0 {
                        Some(e) => &e.$key,
                        None => &*FALLBACK_EMOJI
                    }
                }
            )*
        }

        impl HAppEmojiStore {
            pub async fn load_and_update(config: &HBotConfig, ctx: &Http) -> anyhow::Result<HAppEmojiStore> {
                let emojis = load_emojis(ctx).await?;

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
    config.azur_lane_data.is_some()
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

static FALLBACK_EMOJI: Lazy<ReactionType> = Lazy::new(|| ReactionType::from('❔'));

async fn load_emojis(_ctx: &Http) -> anyhow::Result<Vec<EmojiTemp>> {
    // todo: request from context
    macro_rules! make {
        ($name:literal, $id:literal) => { EmojiTemp {
            id: $id,
            name: $name.to_owned(),
        } };
    }

    Ok(vec![
        make!("Hull_AE", 1265947482756481074),
        make!("Hull_AR", 1265947498216685581),
        make!("Hull_BB", 1265947510610989057),
        make!("Hull_BBV", 1265947521558122519),
        make!("Hull_BC", 1265947532480217171),
        make!("Hull_BM", 1265947543397732433),
        make!("Hull_CA", 1265947555150168114),
        make!("Hull_CB", 1265947565552042035),
        make!("Hull_CL", 1265947576251977789),
        make!("Hull_CV", 1265947588075458611),
        make!("Hull_CVL", 1265947599194816585),
        make!("Hull_DD", 1265947608384278590),
        make!("Hull_DDGm", 1265947617884635147),
        make!("Hull_DDGv", 1265947630853292072),
        make!("Hull_IXm", 1265947646967676999),
        make!("Hull_IXs", 1265947656530694196),
        make!("Hull_IXv", 1265947665338728449),
        make!("Hull_SS", 1265947673832325164),
        make!("Hull_SSV", 1265947682787037245),
    ])
}

// temp: has minimum required contract for `generate`
// to be removed when serenity supports requesting app emojis
struct EmojiTemp {
    id: u64,
    name: String,
}

impl EmojiTemp {
    fn into(self) -> ReactionType {
        ReactionType::Custom {
            animated: false,
            id: self.id.into(),
            name: Some(self.name)
        }
    }
}

async fn update_emoji(_ctx: &Http, _name: &str, _image_data: &[u8]) -> anyhow::Result<ReactionType> {
    // todo: update for app
    Ok(FALLBACK_EMOJI.clone())
}

impl<'a> HAppEmojis<'a> {
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
