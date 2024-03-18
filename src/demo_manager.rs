use tf_demo_parser::demo::header::Header;
use std::{collections::HashMap, fs::Metadata, path::{Path, PathBuf}};
use bitbuffer::BitRead;
use glob::glob;
use serde::{Serialize, Deserialize};
use tokio::fs;
use trash;

#[derive(Serialize, Deserialize)]
struct EventContainer {
    events: Vec<Event>,
    notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub tick: u32,
    pub value: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Demo {
    pub path: PathBuf,
    pub filename: String,
    pub header: Option<Header>,
    pub events: Vec<Event>,
    pub notes: Option<String>,
    pub metadata: Option<Metadata>,
}

impl Demo {
    pub const TICKRATE: f32 = 66.667;
    pub fn new(path: &Path) -> Self {
        Demo{
            path: path.into(),
            filename: path.file_name().unwrap().to_str().unwrap().into(),
            header: None,
            events: Vec::new(),
            notes: None,
            metadata: None,
        }
    }

    pub async fn read_data(&mut self) {
        if let Some(_) = self.header {
            return;
        }
        let f = fs::read(&self.path).await.unwrap();

        let demo = tf_demo_parser::Demo::new(&f);
        self.header = Header::read(&mut demo.get_stream()).map_or(None, |r|Some(r));

        let mut bookmark_file = self.path.clone();
        bookmark_file.set_extension("json");

        let file = fs::read(bookmark_file).await;
        if let Ok(char_bytes) = file{
            let parsed: EventContainer = serde_json::from_slice(&char_bytes).unwrap();
            self.events = parsed.events;
            self.notes = parsed.notes;
        }

        self.metadata = fs::metadata(&self.path).await.inspect_err(|e|log::warn!("Failed reading metadata for {}, {}", self.path.display(), e)).ok();
    }

    pub fn get_path(&self) -> String {
        self.path.display().to_string()
    }

    pub async fn save_json(&self) {
        let mut bookmark_file = self.path.clone();
        bookmark_file.set_extension("json");

        let mut notes = self.notes.clone();
        if let Some(s) = &self.notes {
            if s.is_empty() {
                notes = None;
            }
        }
        if notes.is_none() && self.events.is_empty() {
            let _ = fs::remove_file(&bookmark_file).await
            .inspect_err(|e|log::info!("Couldn't delete bookmark file {}, {}", bookmark_file.display(), e));
            return;
        }

        let container = EventContainer{
            events: self.events.clone(),
            notes: notes,
        };
        let json = serde_json::to_string_pretty(&container).unwrap();

        

        let _ = fs::write(&bookmark_file, json).await
            .inspect_err(|e|log::warn!("Couldn't save bookmark file {}, {}", bookmark_file.display(), e));
    }

    pub fn tps(&self) -> Option<f32> {
        Some(self.header.as_ref()?.ticks as f32/self.header.as_ref()?.duration)
    }
}

#[derive(Default, Clone)]
pub struct DemoManager {
    demos: HashMap<String, Demo>,
}

impl DemoManager {
    pub fn new() -> Self {
        DemoManager::default()
    }

    pub async fn load_demos(&mut self, folder_path: &String){
        self.demos.clear();
        for path in glob(&format!("{}/*.dem",folder_path)).unwrap() {
            let d = Demo::new(path.unwrap().as_path());
            self.demos.insert(d.filename.to_owned(), d);
        }
        for demo in &mut self.demos.values_mut() {
            demo.read_data().await;
        }
    }

    pub fn get_demo(&self, name: &str) -> Option<&Demo> {
        self.demos.get(name)
    }

    pub fn get_demos(&self) -> &HashMap<String, Demo> {
        &self.demos
    }

    pub fn get_demos_mut(&mut self) -> &mut HashMap<String, Demo> {
        &mut self.demos
    }

    pub async fn delete_demo(&mut self, name: &str){
        let demo = self.demos.get(name).unwrap().to_owned();

        let mut bookmark_path = demo.path.clone();
        bookmark_path.set_extension("json");

        let _ = tokio::task::spawn_blocking(move ||{
            if let Err(e) = trash::delete(demo.path.as_path()){
                log::info!("Couldn't delete {}, {}", demo.path.display(), e);
            }

            if let Err(e) = trash::delete(bookmark_path.as_path()){
                log::info!("Couldn't delete {}, {}", bookmark_path.display(), e);
            }

        }).await;
        
        self.demos.remove(&demo.filename);
    }
}