use std::fmt;

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString, ValidLength as _};

use crate::ship::{HullType, ShipRarity};
use crate::{Faction, GameServer};

/// Data for a ship skin. This may represent the default skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    /// The skin's ID. [`Ship::skin_by_id`](crate::ship::Ship::skin_by_id)
    /// searches for this.
    pub skin_id: u32,
    /// The image/asset key.
    ///
    /// Asset bundles and chibi sprites from the collector will use this as
    /// their filename.
    pub image_key: FixedString,
    /// The skin's display name.
    pub name: FixedString,
    /// The skin's description.
    pub description: FixedString,
    /// The default dialogue lines.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`SkinWords::server`].
    pub words: FixedArray<SkinWords>,
    /// Replacement dialogue lines, usually after oath.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`SkinWords::server`].
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub words_extra: FixedArray<SkinWords>,
}

/// The block of dialogue for a given skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinWords {
    /// The server with these words.
    pub server: GameServer,
    /// Voice lines played on the main screen when idle or tapped.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub main_screen: FixedArray<MainScreenLine>,
    /// Voices lines that may be played when sortieing other specific ships.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub couple_encourage: FixedArray<CoupleEncourage>,
    /// A map of other simple lines. Instead of accessing this, consider the
    /// helper properties like [`Self::touch`].
    #[serde(flatten)]
    pub other: SkinWordsMap,
}

/// Information about a ship line that may be displayed on the main screen.
///
/// Also see [`SkinWords::main_screen`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainScreenLine(usize, FixedString);

/// Data for voices lines that may be played when sortieing other specific
/// ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupleEncourage {
    /// The line to be played.
    pub line: FixedString,
    /// The amount of allies that need to match the condition.
    pub amount: u32,
    /// The condition rule.
    pub condition: CoupleCondition,
}

/// Condition for [`CoupleEncourage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoupleCondition {
    /// Triggered when other specific ships are present.
    /// Holds a vector of ship group IDs.
    ShipGroup(FixedArray<u32>),
    /// Triggered when ships of specified hull types are present.
    HullType(FixedArray<HullType>),
    /// Triggered when ships of a specified rarity are present.
    Rarity(FixedArray<ShipRarity>),
    /// Triggered when ships from a specified faction are present.
    Faction(FixedArray<Faction>),
    /// Triggered when ships from the same illustrator are present.
    ///
    /// Actual in-game data specifies which one, but it's only ever used to
    /// refer to the same one as the source ship's.
    Illustrator,
    /// Triggered based on team type. Unused?
    Team,
    /// Unknown trigger types.
    #[serde(other)]
    Unknown,
}

impl Skin {
    /// Get the words and extra words for a specific server.
    pub fn words(&self, server: GameServer) -> Option<(&SkinWords, Option<&SkinWords>)> {
        let main = self
            .words
            .iter()
            .find(|s| s.server == server)
            .or_else(|| self.words.first())?;

        let extra = self
            .words_extra
            .iter()
            .find(|s| s.server == server)
            .or_else(|| self.words_extra.first());

        Some((main, extra))
    }
}

impl MainScreenLine {
    /// Creates a new instance.
    #[must_use]
    pub fn new(index: usize, text: FixedString) -> Self {
        Self(index, text)
    }

    /// Gets the index for the line. Relevant for replacement.
    pub fn index(&self) -> usize {
        self.0
    }

    /// Gets the text associated with the line.
    pub fn text(&self) -> &str {
        &self.1
    }

    /// Sets the index for the line.
    #[must_use]
    pub fn with_index(self, index: usize) -> Self {
        Self(index, self.1)
    }
}

/// Declares [`SkinWordsKey`] and all associated helper methods.
macro_rules! ship_skin_words_key {
    (
        $(
            $(#[$attr:meta])*
            $label:ident,
        )*
    ) => {
        /// Key for ship skin words.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[expect(non_camel_case_types)]
        pub enum SkinWordsKey {
            $(
                $(#[$attr])*
                $label
            ),*
        }

        impl SkinWordsKey {
            /// The total count of known keys.
            const COUNT: usize = <[_]>::len(&[$(Self::$label),*]);

            /// Gets the stringified name of the active variant.
            ///
            /// This matches the name of the field this corresponds to in the
            /// serialized format.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$label => stringify!($label),)*
                }
            }
        }

        impl SkinWords {
            $(
                $(#[$attr])*
                #[must_use]
                pub fn $label(&self) -> Option<&str> {
                    self.other.get(SkinWordsKey::$label)
                }
            )*
        }
    };
}

ship_skin_words_key! {
    /// The skin's description.
    ///
    /// Note that [`Skin::description`] originates from the skin's template,
    /// whereas this field is actually part of the skin's words.
    description,
    /// The "introduction". In-game, this is the profile text in the archive.
    introduction,
    /// Voice line played when the ship is obtained.
    acquisition,
    /// Voice line played when the ship is the current secretary and logging
    /// in.
    login,
    /// Voice line played when viewing the ships details and tapping the ship
    /// after its affinity line has finished playing.
    details,
    /// Voice line randomly played in place of a main screen line when tapping
    /// the ship.
    touch,
    /// Voice line played when tapping the "special" area of the ship on the
    /// main screen.
    special_touch,
    /// Voice line played when tapping the "rub" area of the ship on the main
    /// screen. This line is not defined for every ship.
    rub,
    /// Voice line played in place of a main screen line when there are
    /// incomplete missions.
    mission_reminder,
    /// Voice line played in place of a main screen line when there are
    /// complete but unclaimed
    /// missions.
    mission_complete,
    /// Voice line played in place of a main screen line when there is unread
    /// mail.
    mail_reminder,
    /// Voice line played when the ship is the current secretary and the main
    /// screen is viewed for
    /// the first time since a battle.
    return_to_port,
    /// Voice line played in place of a main screen line when there are
    /// finished commissions.
    commission_complete,
    /// Voice line played when enhancing the ship.
    enhance,
    /// Voice line played at the start of a battle the ship is sortied in the
    /// front of the vanguard
    /// or as the flagship.
    flagship_fight,
    /// Voice line played when the ship is the MVP in a victory.
    victory,
    /// Voice line played when the ship is the MVP is a defeat.
    defeat,
    /// Voice line played when the ship activates a skill.
    skill,
    /// Voice line played when the ship drops to low HP.
    low_health,
    /// Affinity voice line for "Disappointed".
    disappointed,
    /// Affinity voice line for "Stranger".
    stranger,
    /// Affinity voice line for "Friendly".
    friendly,
    /// Affinity voice line for "Crush".
    crush,
    /// Affinity voice line for "Love".
    love,
    /// Voice line played in the oath scene.
    oath,
    /// Voice line played when receiving a preferred gift.
    gift_prefer,
    /// Voice line played when receiving a disliked gift.
    gift_dislike,
}

// avoid duplicating the `name` fn match
impl fmt::Debug for SkinWordsKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for SkinWordsKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

type SkinWordsEntry = (SkinWordsKey, FixedString);

/// A map of ship skin words.
///
/// This is optimized for memory rather than access speed.
//
// most `SkinWords` instances are for non-default skins that often lack about half the possible
// entries. subsequently it saves memory to "pack" the fields that _are_ present into an array.
#[derive(Clone)]
pub struct SkinWordsMap(
    /// A list of key-value pairs sorted by the key.
    FixedArray<SkinWordsEntry>,
);

fn key_fn(t: &SkinWordsEntry) -> SkinWordsKey {
    t.0
}

// no way offered to "unwrap" this struct since the assumption is that it's
// mostly used for borrowed data and rarely in an owned consumable form.
impl SkinWordsMap {
    /// Creates a new skin words map.
    ///
    /// This array is sorted by the key upon construction and does not need to
    /// be pre-sorted.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a key is duplicated.
    pub fn new(mut value: FixedArray<SkinWordsEntry>) -> Result<Self, SkinWordsMapError> {
        value.sort_unstable_by_key(key_fn);

        // ensure there are no duplicate keys provided. since it's already sorted by the
        // keys, comparing all pairs of adjacent keys is good enough to figure that out.
        for window in value.windows(2) {
            let [l, r] = window.as_array().expect("must be len 2");
            if l.0 == r.0 {
                return Err(SkinWordsMapError::DuplicateKey(l.0));
            }
        }

        Ok(Self(value))
    }

    /// The amount of lines stored.
    pub fn len(&self) -> usize {
        self.0.len().to_usize()
    }

    /// Whether this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets the line for a specific key, if present.
    pub fn get(&self, key: SkinWordsKey) -> Option<&str> {
        let slice = self.0.as_slice();
        let index = slice.binary_search_by_key(&key, key_fn).ok()?;
        Some(slice.get(index)?.1.as_str())
    }

    /// Iterates over all key-value pairs.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (SkinWordsKey, &str)> + ExactSizeIterator {
        self.0.iter().map(|(key, value)| (*key, value.as_str()))
    }
}

// debug and serde as a map
impl fmt::Debug for SkinWordsMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl Serialize for SkinWordsMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.iter())
    }
}

impl<'de> Deserialize<'de> for SkinWordsMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};

        struct ThisVisitor;

        impl<'de> Visitor<'de> for ThisVisitor {
            type Value = SkinWordsMap;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("ship skin words key-value pairs")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let count = map.size_hint().unwrap_or_default().max(SkinWordsKey::COUNT);

                let mut buf = Vec::with_capacity(count);
                while let Some(entry) = map.next_entry()? {
                    buf.push(entry);
                }

                let buf = buf.try_into().map_err(A::Error::custom)?;
                SkinWordsMap::new(buf).map_err(|SkinWordsMapError::DuplicateKey(k)| {
                    A::Error::duplicate_field(k.name())
                })
            }
        }

        deserializer.deserialize_map(ThisVisitor)
    }
}

/// Error when constructing a [`SkinWordsMap`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SkinWordsMapError {
    #[error("key {0:?} was duplicated")]
    DuplicateKey(SkinWordsKey),
}
