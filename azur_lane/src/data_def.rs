#[macro_export]
macro_rules! define_data_enum {
    ($v:vis enum $name:ident for $vd:vis $data:ident { $($data_vis:vis $data_name:ident : $data_type:ty),* ; $($field:ident $arg:tt),* }) => {
        #[derive(Debug, Clone)]
        $vd struct $data {
            $($data_vis $data_name : $data_type),*
        }

        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        $v enum $name {
            $($field),*
        }

        impl $name {
            #[must_use]
            $vd const fn data(self) -> &'static $data {
                const fn make_val($($data_name : $data_type),*) -> $data {
                    $data { $($data_name),* }
                }

                match self {
                    $(
                        $name::$field => {
                            const VAL: $data = make_val $arg;
                            &VAL
                        }
                    ),*
                }
            }

            $(
                #[must_use]
                $data_vis const fn $data_name (self) -> $data_type {
                    self.data().$data_name
                }
            )*
        }
    };
}

#[must_use]
pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
