use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub demo_folder_path: String,
    pub tf_folder_path: String,
    pub rcon_pw: String,
    pub event_skip_predelay: f32,
    pub doubleclick_play: bool,

    #[serde(skip)]
    pub first_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            demo_folder_path: Default::default(),
            tf_folder_path: Default::default(),
            rcon_pw: Default::default(),
            event_skip_predelay: 30.0,
            doubleclick_play: false,
            
            first_launch: false
        }
    }
}

const SETTINGS_PATH: &str = "settings.json";

impl Settings {
    pub fn load() -> Self {
        match fs::read(SETTINGS_PATH){
            Ok(content) => {
                serde_json::from_slice::<Settings>(&content).unwrap_or_default()
            },
            Err(e) => {
                log::warn!("Couldn't load settings file, {}; Creating default", e);
                let mut s = Settings::default();
                s.first_launch = true;
                s
            },
        }
    }

    pub fn save(&self) {
        if let Err(e) = fs::write(SETTINGS_PATH, serde_json::to_string(self).unwrap()){
            log::warn!("Couldn't save settings file, {}", e);
        }
    }
}