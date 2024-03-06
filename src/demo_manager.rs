use tf_demo_parser::demo::header::Header;
use std::{fs::Metadata, path::{Path, PathBuf}};
use bitbuffer::BitRead;
use glob::glob;
use serde::{Serialize, Deserialize};
use tokio::fs;

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
        let container = EventContainer{
            events: self.events.clone(),
            notes: self.notes.clone(),
        };
        let json = serde_json::to_string_pretty(&container).unwrap();

        let mut bookmark_file = self.path.clone();
        bookmark_file.set_extension("json");

        let _ = fs::write(&bookmark_file, json).await
            .inspect_err(|e|log::warn!("Couldn't save bookmark file {}, {}", bookmark_file.display(), e));
    }
}

#[derive(Default)]
pub struct DemoManager {
    demos: Vec<Demo>,
}

impl DemoManager {
    pub fn new() -> Self {
        DemoManager::default()
    }

    pub async fn load_demos(&mut self, folder_path: &String){
        self.demos.clear();
        for path in glob(&format!("{}/*.dem",folder_path)).unwrap() {
            self.demos.push(Demo::new(path.unwrap().as_path()));
        }
        for demo in &mut self.demos {
            demo.read_data().await;
        }
    }

    pub fn get_demos(&self) -> &Vec<Demo> {
        &self.demos
    }

    pub async fn delete_demo(&mut self, demo: &Demo){
        if let Err(e) = fs::remove_file(demo.path.as_path()).await{
            log::info!("Couldn't delete {}, {}", demo.path.display(), e);
        }

        let mut bookmark_path = demo.path.clone();
        bookmark_path.set_extension("json");

        if let Err(e) = fs::remove_file(bookmark_path.as_path()).await{
            log::info!("Couldn't delete {}, {}", bookmark_path.display(), e);
        }

        self.demos.retain(|d|d.filename != demo.filename);
    }
}