//! Defines a data model that a subset of Azur Lane's game data can be represented as.

use serde::*;

mod data_def;
pub mod equip;
pub mod ship;
pub mod skill;

/// Definition data to be saved/loaded in bulk.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DefinitionData {
    /// All known ships.
    pub ships: Vec<ship::ShipData>,
    /// All known augments.
    pub augments: Vec<equip::Augment>
}

define_data_enum! {
    /// A game faction/nation.
    pub enum Faction for FactionData {
        /// The display name of the faction.
        pub name: &'static str,
        /// The prefix usually used by ships of the faction.
        pub prefix: Option<&'static str>;

        Unknown("Unknown", None),
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
        Tempesta("Tempesta", Some("MOT")),
        META("META", None),
        CollabNeptunia("Neptunia", None),
        CollabBilibili("Bilibili", None),
        CollabUtawarerumono("Utawarerumono", None),
        CollabKizunaAI("Kizuna AI", None),
        CollabHololive("Hololive", None),
        CollabVenusVacation("Venus Vacation", None),
        CollabIdolmaster("Idolm@ster", None),
        CollabSSSS("SSSS", None),
        CollabAtelierRyza("Atelier Ryza", None),
        CollabSenranKagura("Senran Kagura", None)
    }
}
