macro_rules! define_data_enum {
    {
        $(#[$container_attr:meta])*
        $v:vis enum $Enum:ident for $vd:vis $Data:ident {
            $($(#[$data_field_attr:meta])* $data_field_vis:vis $data_field:ident : $DataFieldTy:ty),* ;
            $($(#[$variant_attr:meta])* $variant:ident $arg:tt),* $(,)?
        }
    } => {
        $(#[$container_attr])*
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        $vd struct $Data {
            $(
                $(#[$data_field_attr])*
                $data_field_vis $data_field : $DataFieldTy
            ),*
        }

        $(#[$container_attr])*
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        $v enum $Enum {
            $(
                $(#[$variant_attr])*
                $variant
            ),*
        }

        impl $Enum {
            pub const ALL: &[$Enum] = &[
                $(Self::$variant),*
            ];

            /// Gets the entire associated data structure.
            #[must_use]
            $vd const fn data(self) -> &'static $Data {
                const fn make_val($($data_field : $DataFieldTy),*) -> $Data {
                    $Data { $($data_field),* }
                }

                match self {
                    $(
                        $Enum::$variant => const { &make_val $arg }
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
