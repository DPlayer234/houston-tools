use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::{fs, io};

use azur_lane::equip::*;
use azur_lane::juustagram::*;
use azur_lane::ship::*;
use bytes::Bytes;
use dashmap::DashMap;
use smallvec::{smallvec, SmallVec};
use utils::fuzzy::Search;

type IndexVec = SmallVec<[usize; 2]>;

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
    ship_id_to_augment_indices: HashMap<u32, IndexVec>,

    juustagram_chats: Vec<Chat>,
    juustagram_chat_id_to_index: HashMap<u32, usize>,
    ship_id_to_juustagram_chat_indices: HashMap<u32, IndexVec>,

    // use Bytes to avoid copying the data redundantly
    chibi_sprite_cache: DashMap<String, Option<Bytes>>,
}

impl HAzurLane {
    /// Constructs extended data from definitions.
    #[must_use]
    pub fn load_from(data_path: PathBuf) -> Self {
        // loads the actual definition file from disk
        // the error is just a short description of the error
        fn load_definitions(data_path: &Path) -> anyhow::Result<azur_lane::DefinitionData> {
            use anyhow::Context as _;
            let f = fs::File::open(data_path.join("main.json"))
                .context("Failed to read Azur Lane data.")?;
            let f = io::BufReader::new(f);
            let data = serde_json::from_reader(f).context("Failed to parse Azur Lane data.")?;
            Ok(data)
        }

        // this function should ensure we don't deal with empty paths, absolute or
        // rooted paths, or ones that refer to parent directories to detect
        // potential path traversal attacks when loading untrusted data. note:
        // we only log this, we don't abort.
        fn is_path_sus(path: &Path) -> bool {
            path.components()
                .any(|p| !matches!(p, Component::Normal(_)))
                || path.components().next().is_none()
        }

        fn verify_ship(ship: &ShipData) {
            for skin in &ship.skins {
                if is_path_sus(Path::new(&skin.image_key)) {
                    log::warn!(
                        "image_key '{}' for ship skin {} ({}) may be part of path traversal attack",
                        skin.image_key,
                        skin.skin_id,
                        skin.name,
                    );
                }
            }
        }

        let data = match load_definitions(&data_path) {
            Ok(data) => data,
            Err(err) => {
                log::error!("No Azur Lane data: {err:?}");
                return Self::default();
            },
        };

        let mut this = Self {
            data_path,
            ship_id_to_index: HashMap::with_capacity(data.ships.len()),
            equip_id_to_index: HashMap::with_capacity(data.equips.len()),
            augment_id_to_index: HashMap::with_capacity(data.augments.len()),
            ship_id_to_augment_indices: HashMap::with_capacity(data.augments.len()),
            juustagram_chat_id_to_index: HashMap::with_capacity(data.juustagram_chats.len()),
            ship_id_to_juustagram_chat_indices: HashMap::with_capacity(data.juustagram_chats.len()),
            ships: data.ships,
            equips: data.equips,
            augments: data.augments,
            juustagram_chats: data.juustagram_chats,
            ..Self::default()
        };

        // we trim away "hull_disallowed" equip values that never matter in practice to
        // give nicer outputs otherwise we'd have outputs that state that dive
        // bombers cannot be equipped to frigates. like, duh.
        let mut actual_equip_exist = HashSet::new();
        fn insert_equip_exist(
            actual_equip_exist: &mut HashSet<(EquipKind, HullType)>,
            data: &ShipData,
        ) {
            for equip_kind in data.equip_slots.iter().flat_map(|h| &h.allowed) {
                actual_equip_exist.insert((*equip_kind, data.hull_type));
            }

            for retrofit in &data.retrofits {
                insert_equip_exist(actual_equip_exist, retrofit);
            }
        }

        for (index, data) in this.ships.iter().enumerate() {
            verify_ship(data);

            this.ship_id_to_index.insert(data.group_id, index);
            this.ship_simsearch.insert(&data.name, ());

            // collect known "equip & hull" pairs
            insert_equip_exist(&mut actual_equip_exist, data);
        }

        for (index, data) in this.equips.iter_mut().enumerate() {
            this.equip_id_to_index.insert(data.equip_id, index);
            this.equip_simsearch.insert(
                &format!(
                    "{} {} {} {} {}",
                    data.name,
                    data.faction.name(),
                    data.faction.prefix().unwrap_or("EX"),
                    data.kind.name(),
                    data.rarity.name()
                ),
                (),
            );

            // trim away irrelevant disallowed hulls
            data.hull_disallowed
                .retain(|h| actual_equip_exist.contains(&(data.kind, *h)));
        }

        for (index, data) in this.augments.iter().enumerate() {
            this.augment_id_to_index.insert(data.augment_id, index);
            this.augment_simsearch.insert(&data.name, ());

            if let Some(ship_id) = data.usability.unique_ship_id() {
                this.ship_id_to_augment_indices
                    .entry(ship_id)
                    .and_modify(|v| v.push(index))
                    .or_insert_with(|| smallvec![index]);
            }
        }

        for (index, data) in this.juustagram_chats.iter().enumerate() {
            this.juustagram_chat_id_to_index.insert(data.chat_id, index);
            this.ship_id_to_juustagram_chat_indices
                .entry(data.group_id)
                .and_modify(|v| v.push(index))
                .or_insert_with(|| smallvec![index]);
        }

        this.ship_simsearch.shrink_to_fit();
        this.equip_simsearch.shrink_to_fit();
        this.augment_simsearch.shrink_to_fit();
        this
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

    /// Gets all known Juustagram chats.
    pub fn juustagram_chats(&self) -> &[Chat] {
        &self.juustagram_chats
    }

    /// Gets a ship by its ID.
    #[must_use]
    pub fn ship_by_id(&self, id: u32) -> Option<&ShipData> {
        let index = *self.ship_id_to_index.get(&id)?;
        self.ships.get(index)
    }

    /// Gets all ships by a name prefix.
    pub fn ships_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &ShipData> + use<'_> {
        self.ship_simsearch
            .search(prefix)
            .filter_map(|i| self.ships.get(i.index))
    }

    /// Gets an equip by its ID.
    #[must_use]
    pub fn equip_by_id(&self, id: u32) -> Option<&Equip> {
        let index = *self.equip_id_to_index.get(&id)?;
        self.equips.get(index)
    }

    /// Gets all equips by a name prefix.
    pub fn equips_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Equip> + use<'_> {
        self.equip_simsearch
            .search(prefix)
            .filter_map(|i| self.equips.get(i.index))
    }

    /// Gets an augment by its ID.
    #[must_use]
    pub fn augment_by_id(&self, id: u32) -> Option<&Augment> {
        let index = *self.augment_id_to_index.get(&id)?;
        self.augments.get(index)
    }

    /// Gets all augments by a name prefix.
    pub fn augments_by_prefix(&self, prefix: &str) -> impl Iterator<Item = &Augment> + use<'_> {
        self.augment_simsearch
            .search(prefix)
            .filter_map(|i| self.augments.get(i.index))
    }

    /// Gets unique augments by their associated ship ID.
    pub fn augments_by_ship_id(&self, ship_id: u32) -> impl Iterator<Item = &Augment> {
        self.ship_id_to_augment_indices
            .get(&ship_id)
            .into_iter()
            .flatten()
            .filter_map(|i| self.augments.get(*i))
    }

    /// Gets a Juustagram chat by its ID.
    pub fn juustagram_chat_by_id(&self, chat_id: u32) -> Option<&Chat> {
        let index = *self.juustagram_chat_id_to_index.get(&chat_id)?;
        self.juustagram_chats.get(index)
    }

    /// Gets all Juustagram chats by their associated ship ID.
    pub fn juustagram_chats_by_ship_id(&self, ship_id: u32) -> impl Iterator<Item = &Chat> {
        self.ship_id_to_juustagram_chat_indices
            .get(&ship_id)
            .into_iter()
            .flatten()
            .filter_map(|i| self.juustagram_chats.get(*i))
    }

    /// Gets a chibi's image data.
    #[must_use]
    pub fn get_chibi_image(&self, image_key: &str) -> Option<Bytes> {
        // Consult the cache first. If the image has been seen already, it will be
        // stored here. It may also have a None entry if the image was requested
        // but not found.
        match self.chibi_sprite_cache.get(image_key) {
            Some(entry) => entry.clone(),
            None => self.load_and_cache_chibi_image(image_key),
        }
    }

    #[cold]
    fn load_and_cache_chibi_image(&self, image_key: &str) -> Option<Bytes> {
        // IMPORTANT: the right-hand side of join may be absolute or relative and can
        // therefore read files outside of `data_path`. Currently, this doesn't
        // take user-input, but this should be considered for the future.
        let path = utils::join_path!(&self.data_path, "chibi", image_key; "webp");
        match fs::read(path) {
            Ok(data) => {
                // File read successfully, cache the data.
                use dashmap::mapref::entry::Entry;

                match self.chibi_sprite_cache.entry(image_key.to_owned()) {
                    // data race: loaded concurrently, too slow here. drop the newly read data.
                    Entry::Occupied(entry) => entry.get().clone(),
                    // still empty: wrap the current data and return it
                    Entry::Vacant(entry) => (*entry.insert(Some(Bytes::from(data)))).clone(),
                }
            },
            Err(err) => {
                // Reading failed. Check the error kind.
                use std::io::ErrorKind::*;

                match err.kind() {
                    // Most errors aren't interesting and may be transient issues.
                    // However, these ones imply permanent problems. Store None to prevent repeated
                    // attempts at loading the file.
                    NotFound | PermissionDenied => {
                        // insert, but do not replace a present entry
                        self.chibi_sprite_cache
                            .entry(image_key.to_owned())
                            .or_default();
                    },
                    _ => {
                        log::warn!("Failed to load chibi sprite '{image_key}': {err:?}");
                    },
                };

                None
            },
        }
    }
}
