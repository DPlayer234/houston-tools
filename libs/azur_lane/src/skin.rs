//! Data structures modelling ship skins.

use std::fmt;

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::private::thin_bmap::{ThinBMap, ThinBMapKey};
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

        impl ThinBMapKey for SkinWordsKey {
            const COUNT: usize = <[_]>::len(&[$(Self::$label),*]);

            fn name(self) -> &'static str {
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

/// Error when constructing a [`SkinWordsMap`].
pub type SkinWordsMapError = crate::private::thin_bmap::ThinBMapError<SkinWordsKey>;

/// A map of ship skin words.
///
/// This is optimized for memory rather than access speed.
//
// most `SkinWords` instances are for non-default skins that often lack about half the possible
// entries. subsequently it saves memory to "pack" the fields that _are_ present into an array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinWordsMap(
    /// A list of key-value pairs sorted by the key.
    ThinBMap<SkinWordsKey, FixedString>,
);

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
    pub fn new(value: FixedArray<(SkinWordsKey, FixedString)>) -> Result<Self, SkinWordsMapError> {
        ThinBMap::new(value).map(Self)
    }

    /// The amount of lines stored.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets the line for a specific key, if present.
    pub fn get(&self, key: SkinWordsKey) -> Option<&str> {
        self.0.get(key).map(|x| &**x)
    }

    /// Iterates over all key-value pairs.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (SkinWordsKey, &str)> + ExactSizeIterator {
        self.0.iter().map(|(key, value)| (key, &**value))
    }
}
