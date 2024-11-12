pub mod poise_command_builder;
pub mod discord;

#[cfg_attr(not(feature = "db"), expect(unused_macro))]
macro_rules! bson_id {
    ($expr:expr) => {{
        #[allow(clippy::cast_possible_wrap)]
        let value = $expr.get() as i64;
        value
    }};
}

pub(crate) use bson_id;
