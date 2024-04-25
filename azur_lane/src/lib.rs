use serde::*;

mod data_def;
pub mod ship;
pub mod skill;
pub mod equip;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionData {
    pub ships: Vec<ship::ShipData>
}

define_data_enum! {
    pub enum Faction for FactionData {
        pub name: &'static str,
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
