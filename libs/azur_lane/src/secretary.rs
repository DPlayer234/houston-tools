use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::ship::ShipMainScreenLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialSecretary {
    pub id: u32,
    pub name: FixedString,
    pub kind: FixedString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<FixedString>, // login
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub main_screen: FixedArray<ShipMainScreenLine>, // main
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch: Option<FixedString>, // touch
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_reminder: Option<FixedString>, // mission
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_complete: Option<FixedString>, // mission_complete
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail_reminder: Option<FixedString>, // mail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_port: Option<FixedString>, // home
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_complete: Option<FixedString>, // expedition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub christmas: Option<FixedString>, // shengdan
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_eve: Option<FixedString>, // chuxi
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_day: Option<FixedString>, // xinnian
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valentines: Option<FixedString>, // qingrenjie
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid_autumn_festival: Option<FixedString>, // zhongqiu
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halloween: Option<FixedString>, // wansheng
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_reminder: Option<FixedString>, // huodong
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_module: Option<FixedString>, // genghuan
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chime: Option<Box<[FixedString; 24]>>, // chime_0 - chime_23
}
