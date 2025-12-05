use houston_btn::{Nav, encoding};
use serde::{Deserialize, Serialize};

use super::*;

macro_rules! to_custom_id_consistency {
    ($test_name:ident, $make:expr) => {
        #[test]
        fn $test_name() {
            check_consistency($make, &mut encoding::StackBuf::new());
        }
    };
}

fn check_consistency<'de, T>(value: T, buf: &'de mut encoding::StackBuf)
where
    T: Serialize + Deserialize<'de> + ButtonValue,
{
    let nav = value.to_nav();
    let custom_id = nav.to_custom_id();

    let mut decoder = encoding::decode_custom_id(buf, &custom_id).expect("must decode");

    let key = decoder.read_key().expect("must have a key");
    let re_value: T = decoder.into_button_value().expect("must decode to T");
    let re_custom_id = re_value.to_custom_id();

    assert_eq!(key, T::ACTION.key, "key must be the expected value");
    assert_eq!(
        custom_id, re_custom_id,
        "re-serialized value must have the same custom id"
    );
}

const TEST_NAV: Nav<'static> =
    Nav::from_slice(include_bytes!("tests.rs").first_chunk::<100>().unwrap());

to_custom_id_consistency!(
    check_noop,
    crate::modules::core::buttons::Noop::new(12345, 6789)
);
to_custom_id_consistency!(
    check_to_page,
    crate::modules::core::buttons::ToPage::new(TEST_NAV)
);

to_custom_id_consistency!(
    check_azur_ship,
    crate::modules::azur::buttons::ship::View::builder()
        .ship_id(9999)
        .build()
);
to_custom_id_consistency!(
    check_azur_augment,
    crate::modules::azur::buttons::augment::View::builder()
        .augment_id(9999)
        .build()
);
to_custom_id_consistency!(check_azur_skill, {
    use crate::modules::azur::buttons::skill::*;
    View::builder()
        .source(ViewSource::Augment(1))
        .back(TEST_NAV)
        .build()
});
to_custom_id_consistency!(
    check_azur_lines,
    crate::modules::azur::buttons::lines::View::builder()
        .ship_id(9999)
        .back(TEST_NAV)
        .build()
);
to_custom_id_consistency!(
    check_azur_equip,
    crate::modules::azur::buttons::equip::View::builder()
        .equip_id(9999)
        .build()
);

to_custom_id_consistency!(
    check_perks_shop,
    crate::modules::perks::buttons::shop::View::new()
);

#[test]
fn eq_direct_to_custom_id() {
    let view = crate::modules::azur::buttons::ship::View::builder()
        .ship_id(9999)
        .build();

    assert_eq!(view.to_custom_id(), view.to_nav().to_custom_id());
}
