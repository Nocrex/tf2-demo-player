use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::util;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub demo_folder_path: Option<PathBuf>,
    pub tf_folder_path: Option<PathBuf>,
    pub rcon_pw: String,
    pub rcon_port: u16,
    pub event_skip_predelay: f32,
    pub doubleclick_play: bool,
    pub pause_after_seek: bool,
    pub favorited_folders: Vec<PathBuf>,

    #[serde(skip)]
    pub first_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let tf_folder = util::steam::tf_folder();
        Self {
            demo_folder_path: tf_folder.clone().map(|p| p.join("tf/demos")),
            tf_folder_path: tf_folder.clone(),
            rcon_pw: Default::default(),
            rcon_port: 27015,
            event_skip_predelay: 30.0,
            doubleclick_play: false,
            pause_after_seek: true,
            favorited_folders: tf_folder.map_or_else(||Vec::new(), |f|vec![f]),

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

    pub fn folder_opened(&mut self, path: &PathBuf) {
        self.demo_folder_path = Some(path.into());
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(path) = &self.demo_folder_path {
            if self.favorited_folders.contains(path) {
                self.favorited_folders.retain(|p| *p != *path);
            } else {
                self.favorited_folders.insert(0, path.clone());
            }
            self.save();
        }
    }
    
    pub fn favorited(&self) -> bool {
        if let Some(path) = &self.demo_folder_path {
            self.favorited_folders.contains(path)
        }else{
            false
        }
    }

    pub fn replays_folder(&self) -> Option<PathBuf> {
        self.tf_folder_path
            .as_ref()
            .map(|p| p.join("tf/replay/client/replays"))
    }
}
