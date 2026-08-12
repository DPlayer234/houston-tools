use super::*;

macro_rules! impl_slash {
    ($l:lifetime $ty:ty => |$opt:ident ( $($resolved:pat),* )| $out:expr) => {
        impl<$l> SlashArg<$l> for $ty {
            fn extract(
                _ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                match *resolved {
                    ResolvedValue::$opt( $($resolved),* ) => Ok( $out ),
                    _ => Err(Error::structure_mismatch(concat!("expected ", stringify!($opt)))),
                }
            }

            fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
                option.kind(CommandOptionType::$opt)
            }
        }
    };
}

macro_rules! impl_user_context {
    ($l:lifetime $ty:ty => |$user:pat_param, $member:pat_param| $out:expr) => {
        impl<$l> UserContextArg<$l> for $ty {
            fn extract(
                _ctx: &crate::Context<$l>,
                $user: &$l User,
                $member: Option<&$l PartialMember>,
            ) -> Result<Self, crate::Error<$l>> {
                Ok($out)
            }
        }
    };
}

macro_rules! impl_message_context {
    ($l:lifetime $ty:ty => |$message:pat_param| $out:expr) => {
        impl<$l> MessageContextArg<$l> for $ty {
            fn extract(
                _ctx: &crate::Context<$l>,
                $message: &$l Message,
            ) -> Result<Self, crate::Error<$l>> {
                Ok($out)
            }
        }
    };
}

fn member_error<'a>() -> Error<'a> {
    Error::arg_invalid("unknown server member")
}

#[expect(clippy::cast_possible_truncation)]
const _: () = {
    impl_slash!('ctx f32 => |Number(x)| x as f32);
};
impl_slash!('ctx f64 => |Number(x)| x);
impl_slash!('ctx i64 => |Integer(x)| x);
impl_slash!('ctx bool => |Boolean(x)| x);
impl_slash!('ctx &'ctx str => |String(x)| x);
impl_slash!('ctx &'ctx User => |User(user, _)| user);
impl_slash!('ctx &'ctx PartialMember => |User(_, member)| member.ok_or_else(member_error)?);
impl_slash!('ctx &'ctx Role => |Role(role)| role);
impl_slash!('ctx &'ctx GenericInteractionChannel => |Channel(channel)| channel);
impl_slash!('ctx &'ctx Attachment => |Attachment(attachment)| attachment);

impl_slash!('ctx (&'ctx User, Option<&'ctx PartialMember>) => |User(user, member)| (user, member));
impl_slash!('ctx (&'ctx User, &'ctx PartialMember) => |User(user, member)| (user, member.ok_or_else(member_error)?));

impl_user_context!('ctx &'ctx User => |user, _| user);
impl_user_context!('ctx (&'ctx User, Option<&'ctx PartialMember>) => |user, member| (user, member));
impl_user_context!('ctx (&'ctx User, &'ctx PartialMember) => |user, member| (user, member.ok_or_else(member_error)?));

impl_message_context!('ctx &'ctx Message => |message| message);

macro_rules! impl_slash_int {
    ($($ty:ty)*) => { $(
        impl<'ctx> SlashArg<'ctx> for $ty {
            fn extract(
                _ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                match *resolved {
                    ResolvedValue::Integer(x) => x.try_into().map_err(|_| {
                        Error::structure_mismatch(concat!("received integer out of range for ", stringify!($ty)))
                    }),
                    _ => Err(Error::structure_mismatch("expected Integer")),
                }
            }

            fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
                option
                    .kind(CommandOptionType::Integer)
                    .min_number_value(const { f64::max(<$ty>::MIN as f64, -9007199254740991f64) })
                    .max_number_value(const { f64::min(<$ty>::MAX as f64, 9007199254740991f64) })
            }
        }
    )* };
}

impl_slash_int!(i8 i16 i32 u8 u16 u32 u64);
