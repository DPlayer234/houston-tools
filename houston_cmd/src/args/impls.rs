use super::*;

macro_rules! impl_slash {
    ($l:lifetime $ty:ty => |$ctx:pat_param, $opt:ident ( $($resolved:pat),* )| $out:expr) => {
        impl<$l> SlashArg<$l> for $ty {
            fn extract(
                ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                let $ctx = ctx;
                match *resolved {
                    ResolvedValue::$opt( $($resolved),* ) => Ok( $out ),
                    _ => Err(Error::structure_mismatch(*ctx, concat!("expected ", stringify!($opt)))),
                }
            }

            fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
                option.kind(CommandOptionType::$opt)
            }
        }
    };
}

macro_rules! impl_user_context {
    ($l:lifetime $ty:ty => |$ctx:pat_param, $user:pat_param, $member:pat_param| $out:expr) => {
        $crate::impl_user_context_arg!($l $ty);
        impl<$l> UserContextArg<$l> for $ty {
            fn extract(
                $ctx: &crate::Context<$l>,
                $user: &$l User,
                $member: Option<&$l serenity::model::prelude::PartialMember>,
            ) -> Result<Self, crate::Error<$l>> {
                Ok($out)
            }
        }
    };
}

macro_rules! impl_message_context {
    ($l:lifetime $ty:ty => |$ctx:pat_param, $message:pat_param| $out:expr) => {
        $crate::impl_message_context_arg!($l $ty);
        impl<$l> MessageContextArg<$l> for $ty {
            fn extract(
                $ctx: &crate::Context<$l>,
                $message: &$l Message,
            ) -> Result<Self, crate::Error<$l>> {
                Ok($out)
            }
        }
    };
}

impl_slash!('ctx f32 => |_, Number(x)| x as f32);
impl_slash!('ctx f64 => |_, Number(x)| x);
impl_slash!('ctx i64 => |_, Integer(x)| x);
impl_slash!('ctx bool => |_, Boolean(x)| x);
impl_slash!('ctx &'ctx str => |_, String(x)| x);
impl_slash!('ctx &'ctx User => |_, User(user, _)| user);
impl_slash!('ctx &'ctx PartialMember => |ctx, User(_, member)| member.ok_or_else(|| Error::arg_invalid(*ctx, "unknown server member"))?);
impl_slash!('ctx &'ctx Role => |_, Role(role)| role);
impl_slash!('ctx &'ctx PartialChannel => |_, Channel(channel)| channel);
impl_slash!('ctx &'ctx Attachment => |_, Attachment(attachment)| attachment);

impl_slash!('ctx (&'ctx User, Option<&'ctx PartialMember>) => |_, User(user, member)| (user, member));

impl_user_context!('ctx &'ctx User => |_, user, _| user);
impl_user_context!('ctx (&'ctx User, Option<&'ctx PartialMember>) => |_, user, member| (user, member));

impl_message_context!('ctx &'ctx Message => |_, message| message);

macro_rules! impl_slash_int {
    ($($ty:ty)*) => { $(
        impl<'ctx> SlashArg<'ctx> for $ty {
            fn extract(
                ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                match *resolved {
                    ResolvedValue::Integer(x) => x.try_into()
                        .map_err(|_| Error::structure_mismatch(*ctx, "received integer out of range")),
                    _ => Err(Error::structure_mismatch(*ctx, concat!("expected integer"))),
                }
            }

            fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
                option.kind(CommandOptionType::Integer)
                    .min_number_value(f64::max(<$ty>::MIN as f64, -9007199254740991f64))
                    .max_number_value(f64::min(<$ty>::MAX as f64, 9007199254740991f64))
            }
        }
    )* };
}

impl_slash_int!(i8 i16 i32 u8 u16 u32 u64);