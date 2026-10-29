use houston_btn::{Nav, encoding};
use serde::{Deserialize, Serialize};

use super::*;

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

fn random_vec() -> Vec<u8> {
    use rand::prelude::*;

    let len = rand::rng().random_range(16..128);
    let mut data = vec![0u8; len];
    rand::rng().fill_bytes(&mut data);
    data
}

#[test]
fn check_noop() {
    check_consistency(
        crate::modules::core::buttons::Noop::new(12345, 6789),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_to_page() {
    check_consistency(
        crate::modules::core::buttons::ToPage::new(Nav::from_slice(&random_vec())),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_azur_ship() {
    check_consistency(
        crate::modules::azur::buttons::ship::View::builder()
            .ship_id(9999)
            .build(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_azur_augment() {
    check_consistency(
        crate::modules::azur::buttons::augment::View::builder()
            .augment_id(9999)
            .build(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_azur_skill() {
    use crate::modules::azur::buttons::skill::*;

    check_consistency(
        View::builder()
            .source(ViewSource::Augment(1))
            .back(Nav::from_slice(&random_vec()))
            .build(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_azur_lines() {
    check_consistency(
        crate::modules::azur::buttons::lines::View::builder()
            .ship_id(9999)
            .back(Nav::from_slice(&random_vec()))
            .build(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_azur_equip() {
    check_consistency(
        crate::modules::azur::buttons::equip::View::builder()
            .equip_id(9999)
            .build(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn check_perks_shop() {
    check_consistency(
        crate::modules::perks::buttons::shop::View::new(),
        &mut encoding::StackBuf::new(),
    );
}

#[test]
fn eq_direct_to_custom_id() {
    let view = crate::modules::azur::buttons::ship::View::builder()
        .ship_id(9999)
        .build();

    assert_eq!(view.to_custom_id(), view.to_nav().to_custom_id());
}
