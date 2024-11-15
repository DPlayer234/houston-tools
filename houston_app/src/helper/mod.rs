pub mod discord;
pub mod future;

macro_rules! bson_id {
    ($expr:expr) => {{
        #[allow(clippy::cast_possible_wrap)]
        let value = $expr.get() as i64;
        value
    }};
}

pub(crate) use bson_id;
