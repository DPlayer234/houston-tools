macro_rules! define_data_enum {
    // exclude `Unknown` from the `ALL` constant
    (@all_item [$($all:tt)*] Unknown $($rest:tt)*) => {
        $crate::define_data_enum!(@all_item [$($all)*] $($rest)*)
    };
    (@all_item [$($all:tt)*] $variant:ident $($rest:tt)*) => {
        $crate::define_data_enum!(@all_item [$($all)* Self::$variant,] $($rest)*)
    };
    (@all_item [$($all:tt)*]) => {
        &[$($all)*]
    };

    // actual entry point
    {
        $(#[$container_attr:meta])*
        $enum_vis:vis enum $Enum:ident for $data_vis:vis $Data:ident {
            $($(#[$data_field_attr:meta])* $data_field_vis:vis $data_field:ident : $DataFieldTy:ty),* ;
            $($(#[$variant_attr:meta])* $variant:ident $arg:tt),* $(,)?
        }
    } => {
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        $data_vis struct $Data {
            $(
                $(#[$data_field_attr])*
                $data_field_vis $data_field : $DataFieldTy
            ),*
        }

        $(#[$container_attr])*
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        $enum_vis enum $Enum {
            $(
                $(#[$variant_attr])*
                $variant
            ),*
        }

        impl $Enum {
            /// All known values of this enumeration.
            pub const ALL: &[$Enum] = $crate::define_data_enum!(@all_item [] $($variant)*);

            /// Gets the entire associated data structure.
            #[must_use]
            $data_vis const fn data(self) -> &'static $Data {
                const fn make_val($($data_field : $DataFieldTy),*) -> $Data {
                    $Data { $($data_field),* }
                }

                match self {
                    $(
                        Self::$variant => const { &make_val $arg }
                    ),*
                }
            }

            $(
                $(#[$data_field_attr])*
                #[must_use]
                #[inline]
                $data_field_vis const fn $data_field (self) -> $DataFieldTy {
                    self.data().$data_field
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
