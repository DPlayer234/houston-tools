pub mod ship;
pub mod augment;
pub mod skill;

macro_rules! error {
    ($type:ident : $message:literal) => {
        #[derive(Debug, Clone)]
        pub struct $type;

        impl std::error::Error for $type {}

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}

error!(ShipParseError: "Unknown skill.");
error!(AugmentParseError: "Unknown skill.");
error!(SkillParseError: "Unknown skill.");
