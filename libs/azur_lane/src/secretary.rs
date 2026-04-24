//! Data model for special secretaries (f.e. TB).

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::GameServer;
use crate::skin::MainScreenLine;

/// Data for a special secretary (f.e. TB).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialSecretary {
    /// The ID of this secretary.
    pub id: u32,
    /// The name of this secretary.
    pub name: FixedString,
    /// The "kind" of this secretary.
    ///
    /// This is either the personality type or skin name.
    pub kind: FixedString,
    /// The dialogue for this secretary.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`SpecialSecretaryWords::server`].
    pub words: FixedArray<SpecialSecretaryWords>,
}

/// The block of dialogue for a given special secretary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialSecretaryWords {
    /// The server with these words.
    pub server: GameServer,
    /// Voice line played when the special secretary is the current secretary
    /// and logging in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<FixedString>, // login
    /// Voice lines played on the main screen when idle or tapped.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub main_screen: FixedArray<MainScreenLine>, // main
    /// Voice line randomly played in place of a main screen line when tapping
    /// the special secretary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch: Option<FixedString>, // touch
    /// Voice line played in place of a main screen line when there are
    /// incomplete missions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_reminder: Option<FixedString>, // mission
    /// Voice line played in place of a main screen line when there are
    /// complete but unclaimed missions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_complete: Option<FixedString>, // mission_complete
    /// Voice line played in place of a main screen line when there is unread
    /// mail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail_reminder: Option<FixedString>, // mail
    /// Voice line played when the special secretary is the current secretary
    /// and the main screen is viewed for the first time since a battle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_port: Option<FixedString>, // home
    /// Voice line played in place of a main screen line when there are
    /// finished commissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_complete: Option<FixedString>, // expedition
    /// Voice line played in place of a main screen line during christmas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub christmas: Option<FixedString>, // shengdan
    /// Voice line played in place of a main screen line on new years eye.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_eve: Option<FixedString>, // chuxi
    /// Voice line played in place of a main screen line on new years day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_day: Option<FixedString>, // xinnian
    /// Voice line played in place of a main screen line on valentines day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valentines: Option<FixedString>, // qingrenjie
    /// Voice line played in place of a main screen line during the mid autumn
    /// festival time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid_autumn_festival: Option<FixedString>, // zhongqiu
    /// Voice line played in place of a main screen line on halloween.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halloween: Option<FixedString>, // wansheng
    /// Voice line played in place of a main screen line during an active event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_reminder: Option<FixedString>, // huodong
    /// Voice line played when switching to this special secretary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_module: Option<FixedString>, // genghuan
    /// Voice line played when reaching the corresponding hour of the day.
    ///
    /// The indices match when the line is played, so f.e. `chime[0]` plays at
    /// `00:00` and `chime[14]` plays at `14:00`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chime: Option<Box<[FixedString; 24]>>, // chime_0 - chime_23
}

impl SpecialSecretary {
    /// Get the words for a specific server.
    pub fn words(&self, server: GameServer) -> Option<&SpecialSecretaryWords> {
        let main = self
            .words
            .iter()
            .find(|s| s.server == server)
            .or_else(|| self.words.first())?;

        Some(main)
    }
}
