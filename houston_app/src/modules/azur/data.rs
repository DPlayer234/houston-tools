use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::num::NonZero;
use std::path::{Component, Path};
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fs, io, slice};

use azur_lane::equip::*;
use azur_lane::juustagram::*;
use azur_lane::secretary::*;
use azur_lane::ship::*;
use bytes::Bytes;
use lru::LruCache;
use serenity::small_fixed_array::TruncatingInto as _;
use smallvec::{SmallVec, smallvec};
use utils::fuzzy::{Match, MatchIter, Search};

type IndexVec = SmallVec<[usize; 2]>;

// use Bytes to avoid copying the data redundantly
type LruBytes = LruCache<Box<str>, Option<Bytes>>;

/// Sufficient capacity that shouldn't lead to too much wasted space in the
/// underlying hash map based on `capacity_to_buckets` in hashbrown.
const CHIBI_SPRITE_CAP: NonZero<usize> = NonZero::new(28).unwrap();

/// Extended Azur Lane game data for quicker access.
#[derive(Debug)]
pub struct GameData {
    data_path: Arc<Path>,

    ships: Box<[Ship]>,
    equips: Box<[Equip]>,
    augments: Box<[Augment]>,
    juustagram_chats: Box<[Chat]>,
    special_secretaries: Box<[SpecialSecretary]>,

    ship_id_to_index: HashMap<u32, usize>,
    ship_simsearch: Search<()>,
    equip_id_to_index: HashMap<u32, usize>,
    equip_simsearch: Search<()>,
    augment_id_to_index: HashMap<u32, usize>,
    augment_simsearch: Search<()>,
    ship_id_to_augment_indices: HashMap<u32, IndexVec>,
    juustagram_chat_id_to_index: HashMap<u32, usize>,
    ship_id_to_juustagram_chat_indices: HashMap<u32, IndexVec>,
    special_secretary_id_to_index: HashMap<u32, usize>,
    special_secretary_simsearch: Search<()>,

    chibi_sprite_cache: Mutex<LruBytes>,
}

impl GameData {
    /// Constructs extended data from definitions.
    pub fn load_from(data_path: Arc<Path>) -> anyhow::Result<Self> {
        use anyhow::Context as _;

        // this function should ensure we don't deal with empty paths, absolute or
        // rooted paths, or ones that refer to parent directories to detect
        // potential path traversal attacks when loading untrusted data.
        // note: we only log this, we don't abort.
        fn is_path_sus(path: &Path) -> bool {
            path.components()
                .any(|p| !matches!(p, Component::Normal(_)))
                || path.components().next().is_none()
        }

        fn verify_ship(ship: &Ship) {
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

        // loads the actual definition file from disk
        let data: azur_lane::DefinitionData = {
            let f = fs::File::open(data_path.join("main.json")).context("cannot read file")?;
            let f = io::BufReader::new(f);
            serde_json::from_reader(f).context("cannot parse file")?
        };

        let ships = data.ships.into_boxed_slice();
        let equips = data.equips.into_boxed_slice();
        let augments = data.augments.into_boxed_slice();
        let juustagram_chats = data.juustagram_chats.into_boxed_slice();
        let special_secretaries = data.special_secretaries.into_boxed_slice();

        let mut this = Self {
            data_path,
            // pre-allocate maps with appropriate capacities
            ship_id_to_index: HashMap::with_capacity(ships.len()),
            equip_id_to_index: HashMap::with_capacity(equips.len()),
            augment_id_to_index: HashMap::with_capacity(augments.len()),
            ship_id_to_augment_indices: HashMap::new(),
            juustagram_chat_id_to_index: HashMap::with_capacity(juustagram_chats.len()),
            ship_id_to_juustagram_chat_indices: HashMap::new(),
            special_secretary_id_to_index: HashMap::with_capacity(special_secretaries.len()),
            // move in vecs
            ships,
            equips,
            augments,
            juustagram_chats,
            special_secretaries,
            // default the rest of the fields
            ship_simsearch: Search::new(),
            equip_simsearch: Search::new(),
            augment_simsearch: Search::new(),
            special_secretary_simsearch: Search::new(),
            chibi_sprite_cache: Mutex::new(LruCache::new(CHIBI_SPRITE_CAP)),
        };

        // we trim away "hull_disallowed" equip values that never matter in practice to
        // give nicer outputs otherwise we'd have outputs that state that dive
        // bombers cannot be equipped to frigates. like, duh.
        let mut actual_equip_exist = HashSet::new();
        fn insert_equip_exist(
            actual_equip_exist: &mut HashSet<(EquipKind, HullType)>,
            data: &BaseShip,
        ) {
            for equip_kind in data.equip_slots.iter().flat_map(|h| &h.allowed) {
                actual_equip_exist.insert((*equip_kind, data.hull_type));
            }
        }

        for (index, data) in this.ships.iter().enumerate() {
            verify_ship(data);

            this.ship_id_to_index.insert(data.base.group_id, index);
            this.ship_simsearch.insert(&data.base.name, ());

            // collect known "equip & hull" pairs
            insert_equip_exist(&mut actual_equip_exist, &data.base);
            for retrofit in &data.retrofits {
                insert_equip_exist(&mut actual_equip_exist, &retrofit.base);
            }
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
            let mut hull_disallowed = take(&mut data.hull_disallowed).into_vec();
            hull_disallowed.retain(|h| actual_equip_exist.contains(&(data.kind, *h)));
            data.hull_disallowed = hull_disallowed.trunc_into();
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

        for (index, data) in this.special_secretaries.iter_mut().enumerate() {
            data.name = format!("{} ({})", data.name, data.kind).trunc_into();
            this.special_secretary_id_to_index.insert(data.id, index);
            this.special_secretary_simsearch.insert(&data.name, ());
        }

        // these are probably the wrong size
        this.ship_id_to_augment_indices.shrink_to_fit();
        this.ship_id_to_juustagram_chat_indices.shrink_to_fit();

        // these are probably also the wrong size, in several ways
        this.ship_simsearch.shrink_to_fit();
        this.equip_simsearch.shrink_to_fit();
        this.augment_simsearch.shrink_to_fit();
        this.special_secretary_simsearch.shrink_to_fit();
        Ok(this)
    }

    /// Gets all known ships.
    pub fn ships(&self) -> &[Ship] {
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

    /// Gets all known special secretaries.
    pub fn special_secretaries(&self) -> &[SpecialSecretary] {
        &self.special_secretaries
    }

    /// Gets a ship by its ID.
    #[must_use]
    pub fn ship_by_id(&self, id: u32) -> Option<&Ship> {
        let index = *self.ship_id_to_index.get(&id)?;
        self.ships.get(index)
    }

    /// Gets all ships by a name prefix.
    pub fn ships_by_prefix(&self, prefix: &str) -> ByPrefixIter<'_, Ship> {
        ByPrefixIter::new(&self.ship_simsearch, &self.ships, prefix)
    }

    /// Gets an equip by its ID.
    #[must_use]
    pub fn equip_by_id(&self, id: u32) -> Option<&Equip> {
        let index = *self.equip_id_to_index.get(&id)?;
        self.equips.get(index)
    }

    /// Gets all equips by a name prefix.
    pub fn equips_by_prefix(&self, prefix: &str) -> ByPrefixIter<'_, Equip> {
        ByPrefixIter::new(&self.equip_simsearch, &self.equips, prefix)
    }

    /// Gets an augment by its ID.
    #[must_use]
    pub fn augment_by_id(&self, id: u32) -> Option<&Augment> {
        let index = *self.augment_id_to_index.get(&id)?;
        self.augments.get(index)
    }

    /// Gets all augments by a name prefix.
    pub fn augments_by_prefix(&self, prefix: &str) -> ByPrefixIter<'_, Augment> {
        ByPrefixIter::new(&self.augment_simsearch, &self.augments, prefix)
    }

    /// Gets unique augments by their associated ship ID.
    pub fn augments_by_ship_id(&self, ship_id: u32) -> ByLookupIter<'_, Augment> {
        ByLookupIter::new(
            self.ship_id_to_augment_indices.get(&ship_id),
            &self.augments,
        )
    }

    /// Gets a Juustagram chat by its ID.
    pub fn juustagram_chat_by_id(&self, chat_id: u32) -> Option<&Chat> {
        let index = *self.juustagram_chat_id_to_index.get(&chat_id)?;
        self.juustagram_chats.get(index)
    }

    /// Gets all Juustagram chats by their associated ship ID.
    pub fn juustagram_chats_by_ship_id(&self, ship_id: u32) -> ByLookupIter<'_, Chat> {
        ByLookupIter::new(
            self.ship_id_to_juustagram_chat_indices.get(&ship_id),
            &self.juustagram_chats,
        )
    }

    /// Gets a special secretary by its ID.
    pub fn special_secretary_by_id(&self, id: u32) -> Option<&SpecialSecretary> {
        let index = *self.special_secretary_id_to_index.get(&id)?;
        self.special_secretaries.get(index)
    }

    /// Gets all special secretaries by a name prefix.
    pub fn special_secretaries_by_prefix(
        &self,
        prefix: &str,
    ) -> ByPrefixIter<'_, SpecialSecretary> {
        ByPrefixIter::new(
            &self.special_secretary_simsearch,
            &self.special_secretaries,
            prefix,
        )
    }

    /// Gets a chibi's image data.
    #[must_use]
    pub fn get_chibi_image(&self, image_key: &str) -> Option<Bytes> {
        // Consult the cache first. If the image has been seen already, it will be
        // stored here. It may also have a None entry if the image was requested
        // but not found.
        // Must drop the guard before entering `load_and_cache_chibi_image`!
        { self.chibi_sprite_cache().get(image_key).cloned() }
            .unwrap_or_else(|| self.load_and_cache_chibi_image(image_key))
    }

    fn chibi_sprite_cache(&self) -> MutexGuard<'_, LruBytes> {
        self.chibi_sprite_cache
            .lock()
            .expect("chibi image cache never poisoned")
    }

    #[cold]
    fn load_and_cache_chibi_image(&self, image_key: &str) -> Option<Bytes> {
        log::trace!("Loading chibi sprite: '{image_key}'");

        // IMPORTANT: the right-hand side of join may be absolute or relative and can
        // therefore read files outside of `data_path`. Currently, this doesn't
        // take user-input, but this should be considered for the future.
        let path = utils::join_path!(&*self.data_path, "chibi", image_key; "webp");
        match fs::read(path) {
            Ok(data) => {
                // File read successfully, cache the data.
                // If we were slower than a concurrent caller for the same key, drops the newly
                // read data and returns the one that was loaded first.
                self.chibi_sprite_cache()
                    .get_or_insert(image_key.into(), || {
                        // convert the data `Vec<u8>` to a `Box<[u8]>` first so we can be sure it
                        // doesn't end up caching with excess capacity. this is usually a noop since
                        // `fs::read` should preallocate the correct size, and `Bytes` would do this
                        // itself if the capacity is exact, but we'll just make sure with this.
                        Some(Bytes::from(data.into_boxed_slice()))
                    })
                    .clone()
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
                        self.chibi_sprite_cache()
                            .get_or_insert(image_key.into(), || None)
                            .clone()
                    },
                    _ => {
                        log::warn!("Failed to load chibi sprite '{image_key}': {err:?}");
                        None
                    },
                }
            },
        }
    }
}

macro_rules! common_iter {
    ($Ty:ident, $mapper:expr) => {
        impl<'a, T> Iterator for $Ty<'a, T> {
            type Item = &'a T;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map($mapper(self.items))
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth(n).map($mapper(self.items))
            }

            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }

            fn count(self) -> usize {
                self.inner.count()
            }

            fn fold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.inner.map($mapper(self.items)).fold(init, f)
            }
        }

        impl<T> DoubleEndedIterator for $Ty<'_, T> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.next_back().map($mapper(self.items))
            }

            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth_back(n).map($mapper(self.items))
            }

            fn rfold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.inner.map($mapper(self.items)).rfold(init, f)
            }
        }

        impl<T> ExactSizeIterator for $Ty<'_, T> {
            fn len(&self) -> usize {
                self.inner.len()
            }
        }
    };
}

pub struct ByPrefixIter<'a, T> {
    inner: MatchIter<'a, ()>,
    items: &'a [T],
}

impl<'a, T> ByPrefixIter<'a, T> {
    fn new(search: &'a Search<()>, items: &'a [T], prefix: &str) -> Self {
        let inner = search.search(prefix);
        Self { inner, items }
    }
}

fn prefix_mapper<'a, T>(items: &'a [T]) -> impl Fn(Match<'a, ()>) -> &'a T {
    // the used indices should always be in range
    |i| &items[i.index]
}

common_iter!(ByPrefixIter, prefix_mapper);

pub struct ByLookupIter<'a, T> {
    inner: slice::Iter<'a, usize>,
    items: &'a [T],
}

impl<'a, T> ByLookupIter<'a, T> {
    pub fn new(lookup: Option<&'a IndexVec>, items: &'a [T]) -> Self {
        let inner = lookup
            .map_or_else(<&[usize]>::default, IndexVec::as_slice)
            .iter();
        Self { inner, items }
    }
}

fn lookup_mapper<'a, T>(items: &'a [T]) -> impl Fn(&usize) -> &'a T {
    // the used indices should always be in range
    |i| &items[*i]
}

common_iter!(ByLookupIter, lookup_mapper);
