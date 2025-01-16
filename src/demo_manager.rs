use chrono::{Datelike, Timelike};
use tf_demo_parser::demo::header::Header;
use std::{collections::HashMap, fs::Metadata, time::SystemTime};
use bitbuffer::BitRead;
use glob::glob;
use serde::{Serialize, Deserialize};
use async_std::{fs, io, path::{Path, PathBuf}, task};
use trash;
use rand::Rng;

#[derive(Serialize, Deserialize)]
struct EventContainer {
    events: Vec<Event>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    pub async fn has_replay(&self, replays_folder: &Path) -> bool {
        return replays_folder.join(&self.filename).exists().await;
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

        let mut events = self.events.clone();
        events.sort_by_key(|e|e.tick);

        let container = EventContainer{
            events: events,
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


async fn create_replay_index_file(replay_folder: &Path) -> io::Result<()>{
    let index_path = replay_folder.join("replays.dmx");
    if !index_path.exists().await {
        fs::write(index_path, "\"root\"\n{\n\t\"version\"\t\"0\"\n}").await?;
    }
    Ok(())
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
            let d = Demo::new(Path::new(path.unwrap().to_str().unwrap()));
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

        let _ = task::spawn_blocking(move ||{
            if let Err(e) = trash::delete(demo.path.as_path()){
                log::info!("Couldn't delete {}, {}", demo.path.display(), e);
            }

            if let Err(e) = trash::delete(bookmark_path.as_path()){
                log::info!("Couldn't delete {}, {}", bookmark_path.display(), e);
            }

        }).await;
        
        self.demos.remove(&demo.filename);
    }

    pub async fn delete_empty_demos(&mut self) {
        let empties: Vec<String> = self.demos.values().filter(|d|d.header.as_ref().map_or(true, |h|h.duration < 0.5)).map(|d|d.filename.clone()).collect();
        for demo in empties {
            self.delete_demo(&demo).await;
        }
    }

    pub async fn delete_unmarked_demos(&mut self) {
        let unmarkeds: Vec<String> = self.demos.values().filter(|d|d.events.is_empty()).map(|d|d.filename.clone()).collect();
        for demo in unmarkeds {
            self.delete_demo(&demo).await;
        }
    }

    pub async fn convert_to_replay(&self, replays_folder: &Path, demo: &mut Demo, title: &str) -> io::Result<()> {
        create_replay_index_file(replays_folder).await?;

        let replay_demo_path = replays_folder.join(&demo.filename);
        fs::copy(&demo.path, &replay_demo_path).await?;

        let mut replay_handle: u32 = rand::thread_rng().gen();
        while replays_folder.join(format!("replay_{replay_handle}.dmx")).exists().await {
            replay_handle = rand::thread_rng().gen();
        }

        let create_date: chrono::DateTime<chrono::Local> = chrono::DateTime::from(
            demo.metadata.as_ref()
            .map(|m|m.created().ok())
            .map_or(None, |d|d)
            .unwrap_or(SystemTime::now())
        );

        let kv_date = (create_date.day() - 1) | ((create_date.month() - 1) << 5) | ((create_date.year() as u32 - 2009) << 9);
        let kv_time = create_date.hour() | (create_date.minute() << 5) | (create_date.second() << 11);

        demo.read_data().await;
        let dmx_file_content = format!(
"replay_{replay_handle}
{{
\t\"handle\"\t\"{replay_handle}\"
\t\"map\"\t\"{0}\"
\t\"complete\"\t\"1\"
\t\"title\"\t\"{title}\"
\t\"recon_filename\"\t\"{1}\"
\t\"spawn_tick\"\t\"-1\"
\t\"death_tick\"\t\"-1\"
\t\"status\"\t\"3\"
\t\"length\"\t\"{2}\"
\t\"record_time\"
\t{{
\t\t\"date\"\t\"{kv_date}\"
\t\t\"time\"\t\"{kv_time}\"
\t}}
}}
" , demo.header.as_ref().unwrap().map, demo.filename , demo.header.as_ref().unwrap().duration);
        
        fs::write(replays_folder.join(format!("replay_{replay_handle}.dmx")), dmx_file_content).await?;
        Ok(())
    }
}