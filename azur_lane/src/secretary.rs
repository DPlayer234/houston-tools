use serde::{Deserialize, Serialize};

use crate::ship::ShipMainScreenLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialSecretary {
    pub id: u32,
    pub name: String,
    pub kind: String,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub main_screen: Vec<ShipMainScreenLine>, // main
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch: Option<String>, // touch
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_reminder: Option<String>, // mission
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_complete: Option<String>, // mission_complete
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail_reminder: Option<String>, // mail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_port: Option<String>, // home
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_complete: Option<String>, // expedition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub christmas: Option<String>, // shengdan
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_eve: Option<String>, // chuxi
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_years_day: Option<String>, // xinnian
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valentines: Option<String>, // qingrenjie
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid_autumn_festival: Option<String>, // zhongqiu
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halloween: Option<String>, // wansheng
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_reminder: Option<String>, // huodong
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_module: Option<String>, // genghuan
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chime: Option<Box<[String; 24]>>, // chime_0 - chime_23
}
