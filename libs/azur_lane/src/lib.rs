//! Defines a data model that a subset of Azur Lane's game data can be
//! represented as.
#![allow(clippy::upper_case_acronyms)]

use std::str::FromStr;

use data_def::define_data_enum;
use serde::{Deserialize, Serialize};
use small_fixed_array::FixedArray;

mod data_def;
pub mod equip;
pub mod juustagram;
pub mod secretary;
pub mod ship;
pub mod skill;
pub mod skin;

/// Definition data to be saved/loaded in bulk.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct DefinitionData {
    /// All known ships.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub ships: FixedArray<ship::Ship>,
    /// All known equips.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub equips: FixedArray<equip::Equip>,
    /// All known augments.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub augments: FixedArray<equip::Augment>,
    /// All known Juustagram chats.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub juustagram_chats: FixedArray<juustagram::Chat>,
    /// All special secretary variants.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub special_secretaries: FixedArray<secretary::SpecialSecretary>,
}

define_data_enum! {
    /// The supported game servers.
    #[derive(Default)]
    pub enum GameServer for GameServerData {
        pub label: &'static str;

        EN("EN"),
        JP("JP"),
        CN("CN"),
        KR("KR"),
        TW("TW"),
        #[serde(other)]
        #[default]
        Unknown("--"),
    }
}

/// Error when converting a string to a [`GameServer`].
#[derive(Debug, thiserror::Error)]
#[error("unrecognized game server label")]
pub struct GameServerFromStrError(());

impl FromStr for GameServer {
    type Err = GameServerFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|k| s.eq_ignore_ascii_case(k.label()))
            .ok_or(GameServerFromStrError(()))
    }
}

define_data_enum! {
    /// A game faction/nation.
    pub enum Faction for FactionData {
        /// The display name of the faction.
        pub name: &'static str,
        /// The prefix usually used by ships of the faction.
        pub prefix: Option<&'static str>;

        Universal("Universal", Some("UNIV")),
        EagleUnion("Eagle Union", Some("USS")),
        RoyalNavy("Royal Navy", Some("HMS")),
        SakuraEmpire("Sakura Empire", Some("IJN")),
        IronBlood("Iron Blood", Some("KMS")),
        DragonEmpery("Dragon Empery", Some("ROC")),
        SardegnaEmpire("Sardegna Empire", Some("RN")),
        NorthernParliament("Northern Parliament", Some("SN")),
        IrisLibre("Iris Libre", Some("FFNF")),
        VichyaDominion("Vichya Dominion", Some("MNF")),
        IrisOrthodoxy("Iris Orthodoxy", Some("NF")),
        KingdomOfTulipa("Kingdom of Tulipa", Some("HNLMS")),
        Tempesta("Tempesta", Some("MOT")),
        Meta("META", Some("META")),
        Siren("Siren", None),
        CollabNeptunia("Neptunia", None),
        CollabBilibili("Bilibili", None),
        CollabUtawarerumono("Utawarerumono", None),
        CollabKizunaAI("Kizuna AI", None),
        CollabHololive("Hololive", None),
        CollabVenusVacation("Venus Vacation", None),
        CollabIdolmaster("Idolm@ster", None),
        CollabSSSS("SSSS", None),
        CollabAtelierRyza("Atelier Ryza", None),
        CollabSenranKagura("Senran Kagura", None),
        CollabToLoveRu("To LOVE-Ru", None),
        CollabBlackRockShooter("BLACK★ROCK SHOOTER", None),
        CollabAtelierYumia("Atelier Yumia", None),
        CollabDanmachi("Danmachi", None),
        CollabDateALive("Date A Live", None),
        #[serde(other)]
        Unknown("Unknown", None),
    }
}
