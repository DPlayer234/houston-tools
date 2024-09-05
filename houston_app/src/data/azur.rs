use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use simsearch::SimSearch;

use azur_lane::equip::*;
use azur_lane::ship::*;

/// Extended Azur Lane game data for quicker access.
#[derive(Debug, Default)]
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
    ship_id_to_augment_index: HashMap<u32, Vec<usize>>,
    chibi_sprite_cache: DashMap<String, Option<Arc<[u8]>>>,
}

impl HAzurLane {
    /// Constructs extended data from definitions.
    #[must_use]
    pub fn load_from(data_path: PathBuf) -> Self {
        let data = match Self::load_definitions(&data_path) {
            Ok(data) => data,
            Err(err) => {
                log::error!("No Azur Lane data: {err}");
                return Self::default();
            }
        };

        let prefix_options = simsearch::SearchOptions::new()
            .threshold(0.9);

        let mut ship_id_to_index = HashMap::with_capacity(data.ships.len());
        let mut ship_simsearch = SimSearch::new_with(prefix_options.clone());

        let mut equip_id_to_index = HashMap::with_capacity(data.equips.len());
        let mut equip_simsearch = SimSearch::new_with(prefix_options);

        let mut augment_id_to_index = HashMap::with_capacity(data.augments.len());
        let mut ship_id_to_augment_index = HashMap::<u32, Vec<usize>>::with_capacity(data.augments.len());

        for (index, data) in data.ships.iter().enumerate() {
            ship_id_to_index.insert(data.group_id, index);
            ship_simsearch.insert(index, &data.name);
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
                ship_id_to_augment_index.entry(ship_id)
                    .and_modify(|v| v.push(index))
                    .or_insert(vec![index]);
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
        self.ship_simsearch.search(prefix).into_iter().filter_map(|i| self.ship_list.get(i))
    }

    /// Gets an equip by its ID.
    pub fn equip_by_id(&self, id: u32) -> Option<&Equip> {
        let index = *self.equip_id_to_index.get(&id)?;
        self.equip_list.get(index)
    }

    /// Gets all equips by a name prefix.
    pub fn equips_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Equip> {
        self.equip_simsearch.search(prefix).into_iter().filter_map(|i| self.equip_list.get(i))
    }

    /// Gets an augment by its ID.
    pub fn augment_by_id(&self, id: u32) -> Option<&Augment> {
        let index = *self.augment_id_to_index.get(&id)?;
        self.augment_list.get(index)
    }

    /// Gets unique augments by their associated ship ID.
    pub fn augments_by_ship_id(&self, ship_id: u32) -> impl Iterator<Item = &Augment> {
        self.ship_id_to_augment_index.get(&ship_id).into_iter().flatten().filter_map(|i| self.augment_list.get(*i))
    }

    /// Gets a chibi's image data.
    pub fn get_chibi_image(&self, image_key: &str) -> Option<Arc<[u8]>> {
        // Consult the cache first. If the image has been seen already, it will be stored here.
        // It may also have a None entry if the image was requested but not found.
        match self.chibi_sprite_cache.get(image_key) {
            Some(entry) => Option::clone(&entry),
            _ => self.load_and_cache_chibi_image(image_key),
        }
    }

    #[cold]
    fn load_and_cache_chibi_image(&self, image_key: &str) -> Option<Arc<[u8]>> {
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
