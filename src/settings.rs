use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub demo_folder_path: String,
    pub rcon_pw: String,
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
                let s = Settings::default();
                s.save();
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