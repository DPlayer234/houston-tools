use super::*;

macro_rules! round_trip_test {
    ($test_name:ident, $variant:ident => $make:expr) => {
        #[test]
        fn $test_name() {
            let args = $make;
            let custom_data = args.to_custom_data();
            let re_args = custom_data.to_button_args().expect("just constructed from valid data");

            assert_eq!(re_args, ButtonArgs::$variant(args));
            assert_eq!(custom_data, re_args.to_custom_data());
        }
    };
}

round_trip_test!(round_trip_args_none, None => common::None::new(12345, 6789));
round_trip_test!(round_trip_args_ship, ViewShip => azur::ship::View::new(9999));
round_trip_test!(round_trip_args_augment, ViewAugment => azur::augment::View::new(9999));
round_trip_test!(round_trip_args_skill, ViewSkill => { use azur::skill::*; View::with_back(ViewSource::Augment(1), CustomData::EMPTY) });
round_trip_test!(round_trip_args_lines, ViewLines => azur::lines::View::new(9999));
round_trip_test!(round_trip_args_equip, ViewEquip => azur::equip::View::new(9999));
