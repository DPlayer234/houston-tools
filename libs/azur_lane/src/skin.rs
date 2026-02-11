use std::fmt;

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString, ValidLength as _};

use crate::ship::{HullType, ShipRarity};
use crate::{Faction, GameServer};

/// Data for a ship skin. This may represent the default skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSkin {
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
    /// is indicated by [`ShipSkinWords::server`].
    pub words: FixedArray<ShipSkinWords>,
    /// Replacement dialogue lines, usually after oath.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`ShipSkinWords::server`].
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub words_extra: FixedArray<ShipSkinWords>,
}

/// The block of dialogue for a given skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSkinWords {
    /// The server with these words.
    pub server: GameServer,
    /// Voice lines played on the main screen when idle or tapped.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub main_screen: FixedArray<ShipMainScreenLine>,
    /// Voices lines that may be played when sortieing other specific ships.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub couple_encourage: FixedArray<ShipCoupleEncourage>,
    /// Sparse collection of other simple lines. Instead of accessing this,
    /// consider the helper properties like [`Self::touch`].
    #[serde(flatten)]
    pub sparse: SparseShipSkinWords,
}

/// Information about a ship line that may be displayed on the main screen.
///
/// Also see [`ShipSkinWords::main_screen`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipMainScreenLine(usize, FixedString);

/// Data for voices lines that may be played when sortieing other specific
/// ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipCoupleEncourage {
    /// The line to be played.
    pub line: FixedString,
    /// The amount of allies that need to match the condition.
    pub amount: u32,
    /// The condition rule.
    pub condition: ShipCouple,
}

/// Condition for [`ShipCoupleEncourage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShipCouple {
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

impl ShipSkin {
    pub fn words(&self, server: GameServer) -> Option<(&ShipSkinWords, Option<&ShipSkinWords>)> {
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

impl ShipMainScreenLine {
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

/// Declares [`ShipSkinWordKey`] and all associated helper methods.
macro_rules! ship_skin_word_key {
    (
        $(
            $(#[$attr:meta])*
            $label:ident,
        )*
    ) => {
        /// Key for sparse ship skin words.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[expect(non_camel_case_types)]
        pub enum ShipSkinWordKey {
            $(
                $(#[$attr])*
                $label
            ),*
        }

        impl ShipSkinWordKey {
            /// The total count of known keys.
            const COUNT: usize = <[_]>::len(&[$(Self::$label),*]);

            fn name(self) -> &'static str {
                match self {
                    $(Self::$label => stringify!($label),)*
                }
            }
        }

        impl ShipSkinWords {
            $(
                $(#[$attr])*
                pub fn $label(&self) -> Option<&str> {
                    self.sparse.get(ShipSkinWordKey::$label)
                }
            )*
        }
    };
}

ship_skin_word_key! {
    /// The skin's description.
    ///
    /// Note that [`ShipSkin::description`] originates from the skin's template,
    /// whereas this field is actually part of the skin's words.
    description,
    /// The "introduction". In-game, this is the profile text in the archive.
    introduction,
    /// Dialogue played when the ship is obtained.
    acquisition,
    login,
    details,
    touch,
    special_touch,
    rub,
    mission_reminder,
    mission_complete,
    mail_reminder,
    return_to_port,
    commission_complete,
    enhance,
    flagship_fight,
    victory,
    defeat,
    skill,
    low_health,
    disappointed,
    stranger,
    friendly,
    crush,
    love,
    oath,
    gift_prefer,
    gift_dislike,
}

// avoid duplicating the `name` fn match
impl fmt::Debug for ShipSkinWordKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A sparse set of ship skin words.
#[derive(Clone)]
pub struct SparseShipSkinWords(
    /// A list of key-value pairs sorted by the key.
    FixedArray<(ShipSkinWordKey, FixedString)>,
);

// no way to "unwrap" this struct since the assumption is that it's mostly used
// for borrowed data and rarely in an owned consumable form.
impl SparseShipSkinWords {
    /// Creates a new sparse ship skin word set.
    ///
    /// This array is sorted by the key upon construction and does not need to
    /// be pre-sorted.
    pub fn new(mut value: FixedArray<(ShipSkinWordKey, FixedString)>) -> Self {
        value.sort_by_key(|x| x.0);
        Self(value)
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
    pub fn get(&self, key: ShipSkinWordKey) -> Option<&str> {
        let slice = self.0.as_slice();
        let index = slice.binary_search_by_key(&key, |x| x.0).ok()?;
        Some(slice.get(index)?.1.as_str())
    }

    /// Iterates over all key-value pairs.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (ShipSkinWordKey, &str)> + ExactSizeIterator {
        self.0.iter().map(|(key, value)| (*key, value.as_str()))
    }
}

impl fmt::Debug for SparseShipSkinWords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl Serialize for SparseShipSkinWords {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.iter())
    }
}

impl<'de> Deserialize<'de> for SparseShipSkinWords {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};

        struct Visit;

        impl<'de> Visitor<'de> for Visit {
            type Value = SparseShipSkinWords;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("ship skin words key-value pairs")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let count = map
                    .size_hint()
                    .unwrap_or_default()
                    .max(ShipSkinWordKey::COUNT);

                let mut buf = Vec::with_capacity(count);
                while let Some((key, value)) = map.next_entry()? {
                    buf.push((key, value));
                }

                let buf = buf.try_into().map_err(A::Error::custom)?;
                Ok(SparseShipSkinWords::new(buf))
            }
        }

        deserializer.deserialize_map(Visit)
    }
}
