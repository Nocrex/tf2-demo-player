use std::fs;

use serde::{Deserialize, Serialize};

use crate::util;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub demo_folder_path: String,
    pub tf_folder_path: String,
    pub rcon_pw: String,
    pub event_skip_predelay: f32,
    pub doubleclick_play: bool,
    pub recent_folders: Vec<String>,

    #[serde(skip)]
    pub first_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let tf_folder = util::steam::tf_folder().map(|p| p.join("tf"));
        Self {
            demo_folder_path: tf_folder.as_ref().map_or("".to_owned(), |p| {
                p.join("demos").to_str().unwrap().to_owned()
            }),
            tf_folder_path: tf_folder.map_or("".to_owned(), |p| p.to_str().unwrap().to_owned()),
            rcon_pw: Default::default(),
            event_skip_predelay: 30.0,
            doubleclick_play: false,
            recent_folders: Vec::new(),

            first_launch: false,
        }
    }
}

const SETTINGS_PATH: &str = "settings.json";

impl Settings {
    pub fn load() -> Self {
        match fs::read(SETTINGS_PATH) {
            Ok(content) => serde_json::from_slice::<Settings>(&content).unwrap_or_default(),
            Err(e) => {
                log::warn!("Couldn't load settings file, {}; Creating default", e);
                let mut s = Settings::default();
                s.first_launch = true;
                s
            }
        }
    }

    pub fn save(&self) {
        if let Err(e) = fs::write(SETTINGS_PATH, serde_json::to_string(self).unwrap()) {
            log::warn!("Couldn't save settings file, {}", e);
        }
    }

    pub fn folder_opened(&mut self, path: &str) {
        self.demo_folder_path = path.to_owned();
        self.recent_folders.retain(|p| *p != path);
        self.recent_folders.insert(0, path.to_owned());
        self.recent_folders.truncate(5);
    }
}
