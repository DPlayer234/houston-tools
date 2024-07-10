macro_rules! define_data_enum {
    {
        $(#[$attr:meta])*
        $v:vis enum $name:ident for $vd:vis $data:ident {
            $($(#[$data_attr:meta])* $data_vis:vis $data_name:ident : $data_type:ty),* ;
            $($(#[$field_attr:meta])* $field:ident $arg:tt),*
        }
    } => {
        $(#[$attr])*
        #[derive(Debug, Clone)]
        $vd struct $data {
            $(
                $(#[$data_attr])*
                $data_vis $data_name : $data_type
            ),*
        }

        $(#[$attr])*
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        $v enum $name {
            $(
                $(#[$field_attr])*
                $field
            ),*
        }

        impl $name {
            /// Gets the entire associated data structure.
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
                $(#[$data_attr])*
                #[must_use]
                $data_vis const fn $data_name (self) -> $data_type {
                    self.data().$data_name
                }
            )*
        }
    };
}

pub(crate) use define_data_enum;

#[must_use]
pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
