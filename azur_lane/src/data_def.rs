#[macro_export]
macro_rules! define_data_enum {
    ($v:vis enum $name:ident for $data:ident { $($data_vis:vis $data_name:ident : $data_type:ty),* ; $($field:ident $arg:tt),* }) => {
        #[derive(Debug, Clone)]
        $v struct $data {
            $($data_vis $data_name : $data_type),*
        }

        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        $v enum $name {
            $($field),*
        }

        impl $data {
            const fn new_auto_data($($data_name : $data_type),*) -> $data {
                $data { $($data_name),* }
            }
        }

        impl $name {
            pub fn data(self) -> &'static $data {
                match self {
                    $(
                        $name::$field => {
                            const VAL: $data = $data::new_auto_data $arg;
                            &VAL
                        }
                    ),*
                }
            }
        }
    };
}

pub fn make_empty_vec<T>() -> Vec<T> {
    Vec::new()
}

pub fn is_empty_vec<T>(arc: &Vec<T>) -> bool {
    arc.is_empty()
}

pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
