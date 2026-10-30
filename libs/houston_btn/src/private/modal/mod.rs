//! Private helpers for the [`crate::modal_parser`] macro.

use std::convert::Infallible;

use serenity::model::application::{CommandDataResolved, Label};

mod impl_direct;
mod impl_parse;
pub mod marker;

/// Shared trait for modal errors to simplify macro emit.
pub trait ModalError: Sized {
    const INVALID: Self;
}

/// The outer result type for [`FromField`].
pub enum ModalResult<T, E> {
    /// Success.
    Ok(T),
    /// Implementation-specific error.
    Err(E),
    /// The data is invalid. Mapped to [`ModalError::INVALID`].
    Invalid,
}

impl<T, E> From<Result<T, E>> for ModalResult<T, E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(t) => Self::Ok(t),
            Err(e) => Self::Err(e),
        }
    }
}

impl<T> From<Option<T>> for ModalResult<T, Infallible> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(t) => Self::Ok(t),
            None => Self::Invalid,
        }
    }
}

/// Allows conversion from a modal item.
pub trait FromField<'a, M: marker::Field + 'a>: Sized {
    type Err;

    fn from_field(
        field: &'a M::Actual,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err>;
}

pub fn dispatch_item<'a, M, T, E>(
    label: &'a Label,
    data: &'a CommandDataResolved,
    key: &str,
    err: impl FnOnce(T::Err) -> E,
) -> Result<Option<T>, E>
where
    M: marker::Field + 'a,
    T: FromField<'a, M>,
    E: ModalError,
{
    match M::unwrap_item(label, key) {
        ModalResult::Ok(item) => match T::from_field(item, data) {
            ModalResult::Ok(ok) => Ok(Some(ok)),
            ModalResult::Err(why) => Err(err(why)),
            ModalResult::Invalid => Err(E::INVALID),
        },
        ModalResult::Err(marker::Empty) => Ok(None),
        ModalResult::Invalid => Err(E::INVALID),
    }
}

pub fn required_field<T, E>(r: Result<Option<T>, E>) -> Result<T, E>
where
    E: ModalError,
{
    match r {
        Ok(Some(v)) => Ok(v),
        Ok(None) => Err(E::INVALID),
        Err(err) => Err(err),
    }
}

/// Emits utility items to parse a specified modal structure.
///
/// The emitted items are:
/// - `Fields`: The successful return type matching the macro input syntax.
/// - `Error`: An error type for failures.
/// - `parse`: The entry point function.
///
/// Because it emits these items directly in the invoking scope, it is
/// recommended to either call this within a function or in its own module.
///
/// The syntax accepted by this macro is:
///
/// `$($life =>)? $( required/optional $kind $name: $Ty ),*`
///
/// That is, an optional lifetime (if specified followed by `=>`), then the list
/// of fields and their type. The field names must match the custom IDs of the
/// modal components.
///
/// The following kinds are accepted:
///
/// | Kind            | Supported Types                                           |
/// |:--------------- |:--------------------------------------------------------- |
/// | `InputText`     | [`&str`]                                                  |
/// | `SelectMenu`    | Array or [`Vec`] of [`&str`], [`User`], [`PartialMember`], `(User, PartialMember)`, [`Role`], [`GenericInteractionChannel`], or references to these. |
/// | `FileUpload`    | Array or [`Vec`] of [`Attachment`] or references to them. |
/// | `RadioGroup`    | [`&str`]                                                  |
/// | `CheckboxGroup` | Array or [`Vec`] of [`&str`]                              |
/// | `Checkbox`      | [`bool`]                                                  |
///
/// Generally, it is recommended to prefer references whenever possible.
///
/// Specifying an array implies that the modal enforces that specific amount. That means f.e.
/// accepting `[&User; 2]` means that the modal can only be submitted if two users are selected.
///
/// Additionally, each of these kinds may be prefixed with `Parse` (f.e. `ParseTextInput`),
/// which allows replacing the [`&str`] and instead accepting any [`FromStr`] type.
///
/// # Examples
///
/// Case without lifetimes:
/// ```
/// houston_btn::modal_parser! {
///     required ParseInputText page: u16,
/// }
/// ```
///
/// With lifetimes:
/// ```
/// use serenity::model::user::User;
/// houston_btn::modal_parser! { 'a =>
///     required InputText label: &'a str,
///     required SelectMenu invitees: Vec<&'a User>,
/// }
/// ```
///
/// [`&str`]: str
/// [`FromStr`]: std::str::FromStr
/// [`Attachment`]: serenity::model::channel::Attachment
/// [`User`]: serenity::model::user::User
/// [`PartialMember`]: serenity::model::guild::PartialMember
/// [`Role`]: serenity::model::guild::Role
/// [`GenericInteractionChannel`]: serenity::model::channel::GenericInteractionChannel
#[macro_export]
macro_rules! modal_parser {
    (@field_ty required $Ty:ty) => { $Ty };
    (@field_ty optional $Ty:ty) => { ::std::option::Option<$Ty> };

    (@error_ty $marker:ident $life:lifetime => $Ty:ty) => {
        <$Ty as $crate::private::modal::FromField<$life, $crate::private::modal::marker::$marker>>::Err
    };

    (@parse required $any:ident $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::required_field(
            $crate::modal_parser!(@parse optional $any $key : $Ty, $interaction)
        )
    };

    (@parse optional $marker:ident $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::dispatch_item::<$crate::private::modal::marker::$marker, $Ty, Error>($key, &$interaction.data.resolved, ::std::stringify!($key), Error::$key)
    };

    (@error_main [$($t:tt)*] $life:lifetime => $( $required:ident $kind:ident $key:ident : $Ty:ty ),*) => {
        /// An error for [`parse`].
        #[derive(::std::fmt::Debug, $crate::private::thiserror::Error)]
        #[expect(non_camel_case_types)]
        pub enum $($t)* {
            /// The interaction does not match the expected modal shape.
            #[error("interaction does not match expected modal")]
            Invalid,
            $(
                /// Parsing the field failed.
                #[error("field {}: {}", ::std::stringify!($key), _0)]
                $key($crate::modal_parser!(@error_ty $kind $life => $Ty))
            ),*
        }
    };
    (@error $life:lifetime => $( $required:ident $kind:ident $key:ident : $Ty:ty ),*) => {
        $crate::modal_parser!(@error_main [Error<$life>] $life => $($required $kind $key : $Ty),*);
    };
    (@error $( $required:ident $kind:ident $key:ident : $Ty:ty ),*) => {
        $crate::modal_parser!(@error_main [Error] 'static => $($required $kind $key : $Ty),*);
    };

    (
        $($life:lifetime =>)?
        $( $required:ident $kind:ident $key:ident : $Ty:ty ),*
        $(,)?
    ) => {
        /// The fields returned and loaded by [`parse`].
        pub struct Fields$(<$life>)? {
            $(pub $key: $crate::modal_parser!(@field_ty $required $Ty)),*
        }

        $crate::modal_parser!(@error $($life =>)? $($required $kind $key : $Ty),*);

        impl$(<$life>)? $crate::private::modal::ModalError for Error$(<$life>)? {
            const INVALID: Self = Self::Invalid;
        }

        /// Parses a modal interaction and returns the loaded fields.
        pub fn parse$(<$life>)?(interaction: &$($life)? $crate::private::serenity::ModalInteraction) -> ::std::result::Result<Fields$(<$life>)?, Error$(<$life>)?> {
            let [$($crate::private::serenity::ModalComponent::Label($key)),*] = interaction.data.components.as_slice() else {
                return ::std::result::Result::Err(Error::Invalid);
            };

            $(
                let $key = $crate::modal_parser!(@parse $required $kind $key: $Ty, interaction)?;
            )*

            ::std::result::Result::Ok(Fields {
                $($key),*
            })
        }
    };
}
