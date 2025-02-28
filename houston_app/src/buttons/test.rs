use super::*;

macro_rules! round_trip_test {
    ($test_name:ident, $variant:ident => $make:expr) => {
        #[test]
        fn $test_name() {
            let args = $make;
            let custom_data = args.to_custom_data();
            let wrapped_args = ButtonArgs::$variant(args);

            let re_args = custom_data
                .to_button_args()
                .expect("just constructed from valid data");

            assert_eq!(re_args, wrapped_args);
            assert_eq!(custom_data, re_args.to_custom_data());

            let custom_id = custom_data.to_custom_id();
            let re_args = ButtonArgs::from_custom_id(&custom_id).expect("must be valid data");

            assert_eq!(re_args, wrapped_args);
            assert_eq!(custom_data, re_args.to_custom_data());

            let wrapped_custom_id = wrapped_args.to_custom_id();
            assert_eq!(custom_id, wrapped_custom_id);
        }
    };
}

round_trip_test!(round_trip_args_none, Noop => core_mod::buttons::Noop::new(12345, 6789));
round_trip_test!(round_trip_args_ship, AzurShip => azur::buttons::ship::View::new(9999));
round_trip_test!(round_trip_args_augment, AzurAugment => azur::buttons::augment::View::new(9999));
round_trip_test!(round_trip_args_skill, AzurSkill => { use azur::buttons::skill::*; View::with_back(ViewSource::Augment(1), CustomData::EMPTY) });
round_trip_test!(round_trip_args_lines, AzurLines => azur::buttons::lines::View::with_back(9999, CustomData::EMPTY));
round_trip_test!(round_trip_args_equip, AzurEquip => azur::buttons::equip::View::new(9999));

#[test]
fn eq_direct_to_custom_id() {
    let view = azur::buttons::ship::View::new(9999);
    assert_eq!(view.to_custom_id(), view.to_custom_data().to_custom_id());
}
