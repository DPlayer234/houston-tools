use crate::HContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chashmap::CHashMap;
use once_cell::sync::Lazy;
use serenity::all::Color;
use serenity::model::id::UserId;
use poise::reply::CreateReply;
use utils::prefix_map::PrefixMap;
use azur_lane::equip::*;
use azur_lane::ship::*;

/// A general color that can be used for various embeds.
pub const DEFAULT_EMBED_COLOR: Color = Color::new(0xDD_A0_DD);

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// The global bot data. Only one instance exists per bot.
pub struct HBotData {
    /// A concurrent hash map to user data.
    user_data: CHashMap<UserId, HUserData>,
    /// Lazily initialized Azur Lane data.
    azur_lane: Lazy<HAzurLane, Box<dyn Send + FnOnce() -> HAzurLane>>
}

/// User-specific data.
#[derive(Debug, Clone)]
pub struct HUserData {
    pub ephemeral: bool
}

/// Extended Azur Lane game data for quicker access.
#[derive(Debug)]
pub struct HAzurLane {
    data_path: PathBuf,
    pub ship_list: Vec<ShipData>,
    pub augment_list: Vec<Augment>,
    ship_id_to_index: HashMap<u32, usize>,
    ship_name_to_index: HashMap<String, usize>,
    ship_prefix_map: PrefixMap<usize>,
    augment_id_to_index: HashMap<u32, usize>,
    ship_id_to_augment_index: HashMap<u32, usize>,
    chibi_sprite_cache: CHashMap<String, Option<Arc<[u8]>>>
}

/// A simple error that can return any error message.
#[derive(Debug, Clone)]
pub struct HArgError(
    /// The error message
    pub &'static str
);

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

impl HBotData {
    /// Creates a new instance.
    pub fn at<P: AsRef<Path>>(data_path: P) -> Self {
        let data_path = data_path.as_ref().to_owned();
        HBotData {
            user_data: chashmap::CHashMap::new(),
            azur_lane: Lazy::new(Box::new(move || HAzurLane::load_from(data_path)))
        }
    }

    /// Forces initialization of held lazy data.
    pub fn force_init(&self) {
        let _ = self.azur_lane();
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
    /// Gets a copy of the user data for the specified user.
    pub fn get_user_data(&self, user_id: UserId) -> HUserData {
        match self.user_data.get(&user_id) {
            None => HUserData::default(),
            Some(guard) => guard.clone()
        }
    }

    /// Replaces the user data for the specified user.
    pub fn set_user_data(&self, user_id: UserId, data: HUserData) {
        self.user_data.insert(user_id, data);
    }

    /// Gets the Azur Lane game data.
    pub fn azur_lane(&self) -> &HAzurLane {
        Lazy::force(&self.azur_lane)
    }
}

impl HUserData {
    /// Creates a reply matching the user data.
    pub fn create_reply(&self) -> CreateReply {
        CreateReply::default()
            .ephemeral(self.ephemeral)
    }
}

/// Extension trait for the poise context.
#[serenity::async_trait]
pub trait HContextExtensions {
    /// Gets a copy of the user data for the current user.
    fn get_user_data(&self) -> HUserData;
    /// Replaces the user data for the current user.
    fn set_user_data(&self, data: HUserData);
    /// Creates a reply matching the user data.
    fn create_reply(&self) -> CreateReply;
    /// Always creates an ephemeral reply.
    fn create_ephemeral_reply(&self) -> CreateReply;

    /// Defers the result.
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

impl HAzurLane {
    /// Constructs extended data from definitions.
    fn load_from(data_path: PathBuf) -> Self {
        let f = std::fs::File::open(data_path.join("main.json")).expect("Failed to read Azur Lane data.");
        let data: azur_lane::DefinitionData = simd_json::from_reader(f).expect("Failed to parse Azur Lane data.");

        let mut ship_id_to_index = HashMap::with_capacity(data.ships.len());
        let mut ship_name_to_index = HashMap::with_capacity(data.ships.len());
        let mut ship_prefix_map = PrefixMap::new();

        let mut augment_id_to_index = HashMap::with_capacity(data.augments.len());
        let mut ship_id_to_augment_index = HashMap::with_capacity(data.augments.len());

        for (index, data) in data.ships.iter().enumerate() {
            ship_id_to_index.insert(data.group_id, index);
            ship_name_to_index.insert(data.name.clone(), index);
            if !ship_prefix_map.insert(&data.name, index) {
                panic!("Duplicate name {} @ id {}", data.name, data.group_id);
            }
        }

        for (index, augment) in data.augments.iter().enumerate() {
            augment_id_to_index.insert(augment.augment_id, index);
            if let Some(ship_id) = augment.unique_ship_id {
                ship_id_to_augment_index.insert(ship_id, index);
            }
        }

        HAzurLane {
            data_path,
            ship_list: data.ships,
            augment_list: data.augments,
            ship_id_to_index,
            ship_name_to_index,
            ship_prefix_map,
            augment_id_to_index,
            ship_id_to_augment_index,
            chibi_sprite_cache: CHashMap::new()
        }
    }

    /// Gets a ship by its ID.
    pub fn ship_by_id(&self, id: u32) -> Option<&ShipData> {
        let index = *self.ship_id_to_index.get(&id)?;
        self.ship_list.get(index)
    }

    /// Gets a ship by its name.
    pub fn ship_by_name(&self, name: &str) -> Option<&ShipData> {
        let index = *self.ship_name_to_index.get(name)?;
        self.ship_list.get(index)
    }

    /// Gets all ships by a name prefix.
    pub fn ships_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &ShipData> {
        self.ship_prefix_map.find(prefix).flat_map(|i| self.ship_list.get(*i))
    }

    /// Gets an augment by its ID.
    pub fn augment_by_id(&self, id: u32) -> Option<&Augment> {
        let index = *self.augment_id_to_index.get(&id)?;
        self.augment_list.get(index)
    }

    /// Gets a unique augment by its associated ship ID.
    pub fn augment_by_ship_id(&self, ship_id: u32) -> Option<&Augment> {
        let index = *self.ship_id_to_augment_index.get(&ship_id)?;
        self.augment_list.get(index)
    }

    pub fn get_chibi_image(&self, image_key: &str) -> Option<Arc<[u8]>> {
        // Consult the cache first. If the image has been seen already, it will be stored here.
        // It may also have a None entry if the image was requested but not found.
        if let Some(entry) = self.chibi_sprite_cache.get(image_key) {
            return entry.clone();
        }

        // IMPORTANT: the right-hand side of join may be absolute or relative and can therefore read
        // files outside of `data_path`. Currently, this doesn't take user-input, but this should
        // be considered for the future.
        match std::fs::read(self.data_path.join("chibi").join(image_key)) {
            Ok(data) => {
                // File read successfully, cache the data.
                let data = Arc::from(data);
                self.chibi_sprite_cache.insert(image_key.to_owned(), Some(Arc::clone(&data)));
                Some(data)
            },
            Err(err) => {
                // Reading failed. Check the error kind.
                use std::io::ErrorKind::*;
                match err.kind() {
                    // Most errors aren't interesting and may be transient issues.
                    // However, these ones imply permanent problems. Store None to prevent repeated attempts.
                    NotFound | PermissionDenied => { self.chibi_sprite_cache.insert(image_key.to_owned(), None); },
                    _ => ()
                };

                None
            }
        }
    }
}
