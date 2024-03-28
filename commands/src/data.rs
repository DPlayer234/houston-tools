use crate::HContext;
use serenity::all::Color;
use serenity::model::id::UserId;
use poise::reply::CreateReply;

/// A general color that can be used for various embeds.
pub const DEFAULT_EMBED_COLOR: Color = Color::new(0xDD_A0_DD);

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

pub struct HBotData {
    user_data: chashmap::CHashMap<UserId, HUserData>
}

#[derive(Clone)]
pub struct HUserData {
    pub ephemeral: bool
}

#[derive(Debug, Clone)]
pub struct HArgError(pub &'static str);

impl std::error::Error for HArgError {}

impl std::fmt::Display for HArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Debug for HBotData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(HBotData)).finish()
    }
}

impl Default for HBotData {
    fn default() -> Self {
        HBotData {
            user_data: chashmap::CHashMap::new()
        }
    }
}

impl Default for HUserData {
    fn default() -> Self {
        HUserData {
            ephemeral: true
        }
    }
}

impl HBotData {
    pub fn get_user_data(&self, user_id: UserId) -> HUserData {
        match self.user_data.get(&user_id) {
            None => HUserData::default(),
            Some(guard) => guard.clone()
        }
    }

    pub fn set_user_data(&self,  user_id: UserId, data: HUserData) {
        self.user_data.insert(user_id, data);
    }
}

impl HUserData {
    pub fn create_reply(&self) -> CreateReply {
        CreateReply::default()
            .ephemeral(self.ephemeral)
    }
}

#[serenity::async_trait]
pub trait HContextExtensions {
    fn get_user_data(&self) -> HUserData;
    fn set_user_data(&self, data: HUserData);
    fn create_reply(&self) -> CreateReply;
    fn create_ephemeral_reply(&self) -> CreateReply;

    async fn defer_dynamic(&self) -> Result<(), serenity::Error>;
}

#[serenity::async_trait]
impl HContextExtensions for HContext<'_> {
    fn get_user_data(&self) -> HUserData {
        self.data().get_user_data(self.author().id)
    }
    
    fn set_user_data(&self, data: HUserData) {
        self.data().set_user_data(self.author().id, data)
    }

    fn create_reply(&self) -> CreateReply {
        self.get_user_data().create_reply()
    }

    fn create_ephemeral_reply(&self) -> CreateReply {
        CreateReply::default().ephemeral(true)
    }

    async fn defer_dynamic(&self) -> Result<(), serenity::Error> {
        if let Self::Application(ctx) = self {
            ctx.defer_response(self.get_user_data().ephemeral).await?;
        }

        Ok(())
    }
}
