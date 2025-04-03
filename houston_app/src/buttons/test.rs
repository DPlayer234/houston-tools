use super::*;

macro_rules! round_trip_test {
    ($test_name:ident, $variant:ident => $make:expr) => {
        #[test]
        fn $test_name() {
            let args = $make;

            // ensure wrapped serializes data the same way
            let custom_data = args.as_custom_data();
            let wrapped_args = ButtonArgs::$variant(args.clone());

            let custom_id = custom_data.to_custom_id();
            let wrapped_custom_id = wrapped_args.to_custom_id();
            assert_eq!(custom_id, wrapped_custom_id);

            let mut buf = encoding::StackBuf::new();
            let re_args =
                encoding::decode_custom_id(&mut buf, &custom_id).expect("must be readable");

            let re_custom_id = re_args.to_custom_id();
            assert_eq!(re_custom_id, custom_id);
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
    assert_eq!(view.to_custom_id(), view.as_custom_data().to_custom_id());
}
