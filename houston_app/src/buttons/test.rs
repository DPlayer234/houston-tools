use super::*;

macro_rules! to_custom_id_consistency {
    ($test_name:ident, $variant:ident => $make:expr) => {
        #[test]
        fn $test_name() {
            let args = $make;
            let nav = args.to_nav();
            let nav_custom_id = nav.to_custom_id();
            let direct_custom_id = args.to_custom_id();
            assert_eq!(nav_custom_id, direct_custom_id);
        }
    };
}

const TEST_NAV: Nav<'static> =
    Nav::from_slice(include_bytes!("test.rs").first_chunk::<100>().unwrap());

to_custom_id_consistency!(round_trip_args_none, Noop => crate::modules::core::buttons::Noop::new(12345, 6789));
to_custom_id_consistency!(round_trip_args_ship, AzurShip => crate::modules::azur::buttons::ship::View::new(9999));
to_custom_id_consistency!(round_trip_args_augment, AzurAugment => crate::modules::azur::buttons::augment::View::new(9999));
to_custom_id_consistency!(round_trip_args_skill, AzurSkill => { use crate::modules::azur::buttons::skill::*; View::with_back(ViewSource::Augment(1), TEST_NAV) });
to_custom_id_consistency!(round_trip_args_lines, AzurLines => crate::modules::azur::buttons::lines::View::with_back(9999, TEST_NAV));
to_custom_id_consistency!(round_trip_args_equip, AzurEquip => crate::modules::azur::buttons::equip::View::new(9999));

#[test]
fn eq_direct_to_custom_id() {
    let view = crate::modules::azur::buttons::ship::View::new(9999);
    assert_eq!(view.to_custom_id(), view.to_nav().to_custom_id());
}
