use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use utils::fuzzy::Search;

use azur_lane::equip::*;
use azur_lane::ship::*;

/// Extended Azur Lane game data for quicker access.
#[derive(Debug, Default)]
pub struct HAzurLane {
    data_path: PathBuf,
    ships: Vec<ShipData>,
    equips: Vec<Equip>,
    augments: Vec<Augment>,
    ship_id_to_index: HashMap<u32, usize>,
    ship_simsearch: Search<()>,
    equip_id_to_index: HashMap<u32, usize>,
    equip_simsearch: Search<()>,
    augment_id_to_index: HashMap<u32, usize>,
    augment_simsearch: Search<()>,
    ship_id_to_augment_index: HashMap<u32, Vec<usize>>,
    chibi_sprite_cache: DashMap<String, Option<Arc<[u8]>>>,
}

impl HAzurLane {
    /// Constructs extended data from definitions.
    #[must_use]
    pub fn load_from(data_path: PathBuf) -> Self {
        // loads the actual definition file from disk
        // the error is just a short description of the error
        fn load_definitions(data_path: &Path) -> anyhow::Result<azur_lane::DefinitionData> {
            use anyhow::Context;
            let f = std::fs::File::open(data_path.join("main.json")).context("Failed to read Azur Lane data.")?;
            let data = simd_json::from_reader(f).context("Failed to parse Azur Lane data.")?;
            Ok(data)
        }

        // this function should ensure we don't deal with empty paths, absolute or rooted paths,
        // or ones that refer to parent directories to detect potential path traversal attacks
        // when loading untrusted data. note: we only log this, we don't abort.
        fn is_path_sus(path: &Path) -> bool {
            path.components().any(|p| !matches!(p, std::path::Component::Normal(_))) ||
            path.components().next().is_none()
        }

        fn verify_ship(ship: &ShipData) {
            for skin in &ship.skins {
                if is_path_sus(Path::new(&skin.image_key)) {
                    log::warn!("image_key '{}' for ship skin {} ({}) may be part of path traversal attack", skin.image_key, skin.skin_id, skin.name);
                }
            }
        }

        let mut data = match load_definitions(&data_path) {
            Ok(data) => data,
            Err(err) => {
                log::error!("No Azur Lane data: {err:?}");
                return Self::default();
            }
        };

        let mut ship_id_to_index = HashMap::with_capacity(data.ships.len());
        let mut ship_simsearch = Search::new();

        let mut equip_id_to_index = HashMap::with_capacity(data.equips.len());
        let mut equip_simsearch = Search::new();

        let mut augment_id_to_index = HashMap::with_capacity(data.augments.len());
        let mut augment_simsearch = Search::new();
        let mut ship_id_to_augment_index = HashMap::<u32, Vec<usize>>::with_capacity(data.augments.len());

        // we trim away "hull_disallowed" equip values that never matter in practice to give nicer outputs
        // otherwise we'd have outputs that state that dive bombers cannot be equipped to frigates. like, duh.
        let mut actual_equip_exist = HashSet::new();
        fn insert_equip_exist(actual_equip_exist: &mut HashSet<(EquipKind, HullType)>, data: &ShipData) {
            for equip_kind in data.equip_slots.iter().flat_map(|h| &h.allowed) {
                actual_equip_exist.insert((*equip_kind, data.hull_type));
            }

            for retrofit in &data.retrofits {
                insert_equip_exist(actual_equip_exist, retrofit);
            }
        }

        for (index, data) in data.ships.iter().enumerate() {
            verify_ship(data);

            ship_id_to_index.insert(data.group_id, index);
            ship_simsearch.insert(&data.name, ());

            // collect known "equip & hull" pairs
            insert_equip_exist(&mut actual_equip_exist, data);
        }

        for (index, data) in data.equips.iter_mut().enumerate() {
            equip_id_to_index.insert(data.equip_id, index);
            equip_simsearch.insert(&format!(
                "{} {} {} {} {}",
                data.name,
                data.faction.name(), data.faction.prefix().unwrap_or("EX"),
                data.kind.name(),
                data.rarity.name()
            ), ());

            // trim away irrelevant disallowed hulls
            data.hull_disallowed.retain(|h| actual_equip_exist.contains(&(data.kind, *h)));
        }

        for (index, data) in data.augments.iter().enumerate() {
            augment_id_to_index.insert(data.augment_id, index);
            augment_simsearch.insert(&data.name, ());

            if let Some(ship_id) = data.usability.unique_ship_id() {
                ship_id_to_augment_index.entry(ship_id)
                    .and_modify(|v| v.push(index))
                    .or_insert(vec![index]);
            }
        }

        ship_simsearch.shrink_to_fit();
        equip_simsearch.shrink_to_fit();
        augment_simsearch.shrink_to_fit();

        HAzurLane {
            data_path,
            ships: data.ships,
            equips: data.equips,
            augments: data.augments,
            ship_id_to_index,
            ship_simsearch,
            equip_id_to_index,
            equip_simsearch,
            augment_id_to_index,
            augment_simsearch,
            ship_id_to_augment_index,
            chibi_sprite_cache: DashMap::new()
        }
    }

    /// Gets all known ships.
    pub fn ships(&self) -> &[ShipData] {
        &self.ships
    }

    /// Gets all known equipments.
    pub fn equips(&self) -> &[Equip] {
        &self.equips
    }

    /// Gets all known augment modules.
    pub fn augments(&self) -> &[Augment] {
        &self.augments
    }

    /// Gets a ship by its ID.
    pub fn ship_by_id(&self, id: u32) -> Option<&ShipData> {
        let index = *self.ship_id_to_index.get(&id)?;
        self.ships.get(index)
    }

    /// Gets all ships by a name prefix.
    pub fn ships_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &ShipData> {
        self.ship_simsearch.search(prefix).into_iter().filter_map(|i| self.ships.get(i.index))
    }

    /// Gets an equip by its ID.
    pub fn equip_by_id(&self, id: u32) -> Option<&Equip> {
        let index = *self.equip_id_to_index.get(&id)?;
        self.equips.get(index)
    }

    /// Gets all equips by a name prefix.
    pub fn equips_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Equip> {
        self.equip_simsearch.search(prefix).into_iter().filter_map(|i| self.equips.get(i.index))
    }

    /// Gets an augment by its ID.
    pub fn augment_by_id(&self, id: u32) -> Option<&Augment> {
        let index = *self.augment_id_to_index.get(&id)?;
        self.augments.get(index)
    }

    /// Gets all augments by a name prefix.
    pub fn augments_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Augment> {
        self.augment_simsearch.search(prefix).into_iter().filter_map(|i| self.augments.get(i.index))
    }

    /// Gets unique augments by their associated ship ID.
    pub fn augments_by_ship_id(&self, ship_id: u32) -> impl Iterator<Item = &Augment> {
        self.ship_id_to_augment_index.get(&ship_id).into_iter().flatten().filter_map(|i| self.augments.get(*i))
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
