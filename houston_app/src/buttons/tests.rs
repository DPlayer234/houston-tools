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
    View::builder().augment_source(1).back(TEST_NAV).build()
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

#[test]
#[cfg(not(target_pointer_width = "16"))]
fn ensure_compat() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Compat<'a> {
        a: i32,
        b: &'a str,
        c: i32,
    }

    impl ButtonValue for Compat<'_> {
        const ACTION: ButtonAction = ButtonAction {
            key: u32::MAX as usize,
            invoke_button: |_, _| unreachable!(),
            invoke_modal: |_, _| unreachable!(),
        };

        fn to_nav(&self) -> Nav<'_> {
            Nav::from_button_value(self)
        }
    }

    #[rustfmt::skip]
    // a steph-serialized version of `COMPAT_VAL`, including the action key
    //
    //                           u32::MAX          3usize    "hello world"
    //                              |                |  16i32  |          -16i32
    //                              |                |   |     |            |
    //                          [------------------][--][--][-------------][--]
    const COMPAT_BUF: &[u8] = b"\xFF\xFF\xFF\xFF\x0F\x03\x20\x0Bhello world\x1F";
    const COMPAT_BUF_STR: &str =
        "A\u{1007ff}\u{f17ff}\u{b2803}\u{6568}\u{f746c}\u{67f20}\u{c7a6f}\u{62764}&";

    const COMPAT_VAL: Compat<'static> = Compat {
        a: 16,
        b: "hello world",
        c: -16,
    };

    let val_custom_id = COMPAT_VAL.to_custom_id();
    let buf_custom_id = encoding::encode_custom_id(COMPAT_BUF);

    assert_eq!(
        val_custom_id, buf_custom_id,
        "expected both custom ids to be same\n\
         this test breaking implies the serialization format changed unexpectedly"
    );

    assert_eq!(
        val_custom_id, COMPAT_BUF_STR,
        "expected `COMPAT_BUF_STR` to match the encoded `COMPAT_BUF`\n\
         this test breaking implies the encoding format changed unexpectedly"
    );
}
