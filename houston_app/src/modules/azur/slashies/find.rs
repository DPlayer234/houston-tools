use azur_lane::equip::{Augment, Equip};
use azur_lane::ship::ShipData;

use crate::data::{HArgError, HBotData};

fn parse_id_input(input: &str) -> Option<u32> {
    input.strip_prefix("/id:")?.parse().ok()
}

macro_rules! make_find {
    ($fn_name:ident -> $T:ty, $by_id:ident, $by_prefix:ident, $error:literal) => {
        pub fn $fn_name<'a>(data: &'a HBotData, name: &str) -> anyhow::Result<&'a $T> {
            let azur_lane = data.azur_lane();
            parse_id_input(name).map(|id| azur_lane.$by_id(id))
                .unwrap_or_else(|| azur_lane.$by_prefix(name).next())
                .ok_or(HArgError::new_const($error).into())
        }
    };
}

make_find!(ship -> ShipData, ship_by_id, ships_by_prefix, "Unknown ship.");
make_find!(equip -> Equip, equip_by_id, equips_by_prefix, "Unknown equipment.");
make_find!(augment -> Augment, augment_by_id, augments_by_prefix, "Unknown augment module.");
