use serde::{Deserialize, Serialize};

use super::*;

#[test]
fn ensure_compat() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Compat<'a> {
        a: i32,
        b: &'a str,
        c: i32,
    }

    impl ButtonValue for Compat<'_> {
        const ACTION: ButtonAction = ButtonAction {
            key: u16::MAX as usize,
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
    //                           u16::MAX  3usize    "hello world"
    //                              |        |  16i32  |          -16i32
    //                              |        |   |     |            |
    //                          [----------][--][--][-------------][--]
    const COMPAT_BUF: &[u8] = b"\xFF\xFF\x03\x03\x20\x0Bhello world\x1F";
    const COMPAT_BUF_STR: &str =
        "C\u{407ff}\u{2003}\u{5700b}\u{6746c}\u{7286f}\u{77a6f}\u{f6c6c}\u{10800}&";

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
