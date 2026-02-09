//! Private helpers for the [`crate::modal_parser`] macro.

use std::convert::Infallible;
use std::marker::PhantomData;
use std::str::FromStr;

use serenity::model::application::{CommandDataResolved, Label, LabelComponent, SelectMenu};
use serenity::model::channel::GenericInteractionChannel;
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

/// Shared trait for modal errors to simplify macro emit.
pub trait ModalError: Sized {
    const INVALID: Self;
}

/// Error type for `text_ref` kind [`crate::modal_parser`] fields.
///
/// This type is uninhabited and cannot occur.
#[derive(Debug, thiserror::Error)]
#[error("unreachable")]
pub struct TextRefError<T> {
    /// Public field so this type being uninhabited is recognized for pattern
    /// matching. This avoids the user having to match on the variant with this
    /// type.
    pub never: Infallible,
    marker: PhantomData<T>,
}

/// The outer result type for [`FromSelectMenu`].
pub enum ModalResult<T, E> {
    /// Success.
    Ok(T),
    /// Implementation-specific error.
    Err(E),
    /// The data is invalid. Mapped to [`ModalError::INVALID`].
    Invalid,
}

/// Allows conversion from select menu values.
pub trait FromSelectMenu<'a>: Sized {
    type Err;

    fn from_menu(
        menu: &'a SelectMenu,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err>;
}

fn select_resolved_single<I: FromStr, T>(
    values: &[String],
    f: impl Fn(I) -> Option<T>,
) -> ModalResult<T, Infallible> {
    if let [value] = values
        && let Ok(value) = value.parse()
        && let Some(value) = f(value)
    {
        ModalResult::Ok(value)
    } else {
        ModalResult::Invalid
    }
}

fn select_resolved_into_vec<I: FromStr, T>(
    values: &[String],
    f: impl Fn(I) -> Option<T>,
) -> ModalResult<Vec<T>, Infallible> {
    match values.iter().map(|s| s.parse().ok().and_then(&f)).collect() {
        Some(v) => ModalResult::Ok(v),
        None => ModalResult::Invalid,
    }
}

impl<'a> FromSelectMenu<'a> for &'a str {
    type Err = Infallible;

    fn from_menu(
        menu: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        if let [value] = menu.values.as_slice() {
            ModalResult::Ok(value.as_str())
        } else {
            ModalResult::Invalid
        }
    }
}

impl<'a> FromSelectMenu<'a> for Vec<&'a str> {
    type Err = Infallible;

    fn from_menu(
        menu: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        ModalResult::Ok(menu.values.iter().map(String::as_str).collect())
    }
}

macro_rules! impl_from_select_menu_resolved {
    ($Ty:ty, $data:ident, $select:expr) => {
        impl<'a> FromSelectMenu<'a> for $Ty {
            type Err = Infallible;

            fn from_menu(
                menu: &'a SelectMenu,
                $data: &'a CommandDataResolved,
            ) -> ModalResult<Self, Self::Err> {
                select_resolved_single(&menu.values, $select)
            }
        }

        impl<'a> FromSelectMenu<'a> for Vec<$Ty> {
            type Err = Infallible;

            fn from_menu(
                menu: &'a SelectMenu,
                $data: &'a CommandDataResolved,
            ) -> ModalResult<Self, Self::Err> {
                select_resolved_into_vec(&menu.values, $select)
            }
        }
    };
}

impl_from_select_menu_resolved!(&'a User, data, |id| data.users.get(&id));
impl_from_select_menu_resolved!(User, data, |id| data.users.get(&id).cloned());
impl_from_select_menu_resolved!(&'a PartialMember, data, |id| data.members.get(&id));
impl_from_select_menu_resolved!(PartialMember, data, |id| data.members.get(&id).cloned());
impl_from_select_menu_resolved!(&'a Role, data, |id| data.roles.get(&id));
impl_from_select_menu_resolved!(Role, data, |id| data.roles.get(&id).cloned());
impl_from_select_menu_resolved!(&'a GenericInteractionChannel, data, |id| data
    .channels
    .get(&id));
impl_from_select_menu_resolved!(GenericInteractionChannel, data, |id| data
    .channels
    .get(&id)
    .cloned());

impl_from_select_menu_resolved!((&'a User, &'a PartialMember), data, |id| data
    .users
    .get(&id)
    .zip(data.members.get(&id)));
impl_from_select_menu_resolved!((User, PartialMember), data, |id| data
    .users
    .get(&id)
    .zip(data.members.get(&id))
    .map(|(u, m)| (u.clone(), m.clone())));

pub fn text_field<T, E>(
    label: &Label,
    key: &str,
    err: impl FnOnce(T::Err) -> E,
) -> Result<Option<T>, E>
where
    T: FromStr,
    E: ModalError,
{
    if let LabelComponent::InputText(input) = &label.component
        && input.custom_id == key
    {
        input
            .value
            .as_deref()
            .map(|v| T::from_str(v).map_err(err))
            .transpose()
    } else {
        Err(E::INVALID)
    }
}

pub fn text_ref_field<'a, E>(label: &'a Label, key: &str) -> Result<Option<&'a str>, E>
where
    E: ModalError,
{
    if let LabelComponent::InputText(input) = &label.component
        && input.custom_id == key
    {
        Ok(input.value.as_deref())
    } else {
        Err(E::INVALID)
    }
}

pub fn select_field<'a, T, E>(
    label: &'a Label,
    data: &'a CommandDataResolved,
    key: &str,
    err: impl FnOnce(T::Err) -> E,
) -> Result<Option<T>, E>
where
    T: FromSelectMenu<'a>,
    E: ModalError,
{
    if let LabelComponent::SelectMenu(menu) = &label.component
        && menu.custom_id == key
    {
        if menu.values.is_empty() {
            Ok(None)
        } else {
            match T::from_menu(menu, data) {
                ModalResult::Ok(v) => Ok(Some(v)),
                ModalResult::Err(e) => Err(err(e)),
                ModalResult::Invalid => Err(E::INVALID),
            }
        }
    } else {
        Err(E::INVALID)
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
/// | Kind       | Supported Types                    |
/// |:---------- |:---------------------------------- |
/// | `text`     | Any type implementing [`FromStr`]. |
/// | `text_ref` | [`&str`](str)                      |
/// | `select`   | [`&str`](str), [`User`], [`PartialMember`], `(User, PartialMember)`, [`Role`], [`GenericInteractionChannel`], references to these, or a [`Vec`] of any those types if accepting more than 1 value. |
///
/// Generally, it is recommended to take `select` values by reference since it
/// needs to clone them internally. Similarly, `text` values of type `String`
/// should instead use `text_ref` and `&str`.
///
/// # Examples
///
/// Case without lifetimes:
/// ```
/// houston_btn::modal_parser! {
///     required text page: u16,
/// }
/// ```
///
/// With lifetimes:
/// ```
/// use serenity::model::user::User;
/// houston_btn::modal_parser! { 'a =>
///     required text_ref label: &'a str,
///     required select invitees: Vec<&'a User>,
/// }
/// ```
#[macro_export]
macro_rules! modal_parser {
    (@field_ty required $Ty:ty) => { $Ty };
    (@field_ty optional $Ty:ty) => { ::std::option::Option<$Ty> };

    (@error_ty text $life:lifetime => $Ty:ty) => { <$Ty as ::std::str::FromStr>::Err };
    (@error_ty text_ref $life:lifetime => $Ty:ty) => { $crate::private::modal::TextRefError<$Ty> };
    (@error_ty select $life:lifetime => $Ty:ty) => { <$Ty as $crate::private::modal::FromSelectMenu<$life>>::Err };

    (@parse required $any:ident $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::required_field(
            $crate::modal_parser!(@parse optional $any $key : $Ty, $interaction)
        )
    };

    (@parse optional text $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::text_field::<$Ty, Error>($key, ::std::stringify!($key), Error::$key)
    };
    (@parse optional text_ref $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::text_ref_field::<Error>($key, ::std::stringify!($key))
    };
    (@parse optional select $key:ident : $Ty:ty, $interaction:ident) => {
        $crate::private::modal::select_field::<$Ty, Error>($key, &$interaction.data.resolved, ::std::stringify!($key), Error::$key)
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
