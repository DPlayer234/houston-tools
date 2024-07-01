use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use poise::reply::CreateReply;
use serenity::all::{Color, UserId};
use simsearch::SimSearch;

use azur_lane::equip::*;
use azur_lane::ship::*;

/// A general color that can be used for various embeds.
pub const DEFAULT_EMBED_COLOR: Color = Color::new(0xDD_A0_DD);

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// The error type used for the poise context.
pub type HError = anyhow::Error;
/// The full poise context type.
pub type HContext<'a> = poise::Context<'a, Arc<HBotData>, HError>;
/// The poise command result type.
pub type HResult = Result<(), HError>;

/// The global bot data. Only one instance exists per bot.
pub struct HBotData {
    /// A concurrent hash map to user data.
    user_data: DashMap<UserId, HUserData>,
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
    pub equip_list: Vec<Equip>,
    pub augment_list: Vec<Augment>,
    ship_id_to_index: HashMap<u32, usize>,
    ship_simsearch: SimSearch<usize>,
    equip_id_to_index: HashMap<u32, usize>,
    equip_simsearch: SimSearch<usize>,
    augment_id_to_index: HashMap<u32, usize>,
    ship_id_to_augment_index: HashMap<u32, usize>,
    chibi_sprite_cache: DashMap<String, Option<Arc<[u8]>>>,
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
            user_data: DashMap::new(),
            azur_lane: Lazy::new(Box::new(move || HAzurLane::load_from( data_path)))
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
pub trait HContextExtensions {
    /// Gets a copy of the user data for the current user.
    fn get_user_data(&self) -> HUserData;
    /// Replaces the user data for the current user.
    fn set_user_data(&self, data: HUserData);
    /// Creates a reply matching the user data.
    fn create_reply(&self) -> CreateReply;
    /// Always creates an ephemeral reply.
    fn create_ephemeral_reply(&self) -> CreateReply;
}

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
}

impl HAzurLane {
    /// Constructs extended data from definitions.
    fn load_from(data_path: PathBuf) -> Self {
        let data = Self::load_definitions(&data_path).unwrap_or_else(|err| {
            eprintln!("No Azur Lane data: {err}");
            Default::default()
        });

        let prefix_options = simsearch::SearchOptions::new()
            .threshold(0.9);

        let mut ship_id_to_index = HashMap::with_capacity(data.ships.len());
        let mut ship_simsearch = SimSearch::new_with(prefix_options.clone());

        let mut equip_id_to_index = HashMap::with_capacity(data.equips.len());
        let mut equip_simsearch = SimSearch::new_with(prefix_options);

        let mut augment_id_to_index = HashMap::with_capacity(data.augments.len());
        let mut ship_id_to_augment_index = HashMap::with_capacity(data.augments.len());

        for (index, data) in data.ships.iter().enumerate() {
            ship_id_to_index.insert(data.group_id, index);
            ship_simsearch.insert_tokens(index, &[
                &data.name,
                data.faction.name(), data.faction.prefix().unwrap_or("EX"),
                data.hull_type.name(), data.hull_type.designation(),
                data.rarity.name()
            ]);
        }

        for (index, data) in data.equips.iter().enumerate() {
            equip_id_to_index.insert(data.equip_id, index);
            equip_simsearch.insert_tokens(index, &[
                &data.name,
                data.faction.name(), data.faction.prefix().unwrap_or("EX"),
                data.kind.name(),
                data.rarity.name()
            ]);
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
            equip_list: data.equips,
            augment_list: data.augments,
            ship_id_to_index,
            ship_simsearch,
            equip_id_to_index,
            equip_simsearch,
            augment_id_to_index,
            ship_id_to_augment_index,
            chibi_sprite_cache: DashMap::new()
        }
    }

    fn load_definitions(data_path: &Path) -> Result<azur_lane::DefinitionData, &'static str> {
        let f = std::fs::File::open(data_path.join("main.json")).map_err(|_| "Failed to read Azur Lane data.")?;
        let data = simd_json::from_reader(f).map_err(|_| "Failed to parse Azur Lane data.")?;
        Ok(data)
    }

    /// Gets a ship by its ID.
    pub fn ship_by_id(&self, id: u32) -> Option<&ShipData> {
        let index = *self.ship_id_to_index.get(&id)?;
        self.ship_list.get(index)
    }

    /// Gets all ships by a name prefix.
    pub fn ships_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &ShipData> {
        self.ship_simsearch.search(prefix).into_iter().flat_map(|i| self.ship_list.get(i))
    }

    /// Gets an equip by its ID.
    pub fn equip_by_id(&self, id: u32) -> Option<&Equip> {
        let index = *self.equip_id_to_index.get(&id)?;
        self.equip_list.get(index)
    }

    /// Gets all equips by a name prefix.
    pub fn equips_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Equip> {
        self.equip_simsearch.search(prefix).into_iter().flat_map(|i| self.equip_list.get(i))
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
        let path = utils::join_path![&self.data_path, "chibi", image_key; "webp"];
        match std::fs::read(path) {
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
