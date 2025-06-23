use anyhow::Result;
use bitbuffer::BitRead;
use chrono::{Datelike, Timelike};
use glob::glob;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use std::{fs, io::Read};
use tf_demo_parser::demo::header::Header;
use trash;

#[derive(Serialize, Deserialize)]
struct EventContainer {
    events: Vec<Event>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Event {
    pub tick: u32,
    #[serde(rename = "value")]
    pub title: String,
    #[serde(rename = "name")]
    pub ev_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demo {
    pub path: std::path::PathBuf,
    pub filename: String,
    pub header: Option<Header>,
    pub events: Vec<Event>,
    pub notes: Option<String>,
    pub created: Option<SystemTime>,
    pub size: Option<u64>,
    #[serde(skip)]
    pub inspection: Option<Arc<crate::analyser::MatchState>>,
}

impl Demo {
    pub const TICKRATE: f32 = 66.667;
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        let path = path.into();
        Demo {
            filename: path.file_name().unwrap().to_str().unwrap().into(),
            path: path,
            header: None,
            events: Vec::new(),
            notes: None,
            created: None,
            size: None,
            inspection: None,
        }
    }

    pub fn read_data(&mut self) {
        if let Some(_) = self.header {
            return;
        }

        self.header = match (|| {
            let mut header = [0; 1080];
            let mut f = fs::File::open(&self.path)?;
            f.read_exact(&mut header)?;
            let demo = tf_demo_parser::Demo::new(&header);
            anyhow::Ok(Header::read(&mut demo.get_stream())?)
        })() {
            Ok(header) => Some(header),
            Err(e) => {
                log::warn!(
                    "Couldn't read demo header for {}, {}",
                    self.path.display(),
                    e
                );
                None
            }
        };

        let mut bookmark_file = self.path.clone();
        bookmark_file.set_extension("json");

        let file = fs::read(bookmark_file);
        if let Ok(char_bytes) = file {
            match serde_json::from_slice::<EventContainer>(&char_bytes) {
                Ok(parsed) => {
                    self.events = parsed.events;
                    self.events.sort_by_key(|e| e.tick);
                    self.notes = parsed.notes;
                }
                Err(e) => log::warn!(
                    "Failed to parse event file for {}, {}",
                    self.path.display(),
                    e
                ),
            }
        }

        let meta = fs::metadata(&self.path)
            .inspect_err(|e| {
                log::warn!("Failed reading metadata for {}, {}", self.path.display(), e)
            })
            .ok();

        self.size = meta.as_ref().map(|m| m.len());
        self.created = meta.and_then(|m| m.created().ok());
    }

    pub async fn full_analysis(&mut self) -> Result<Arc<crate::analyser::MatchState>> {
        let f = async_std::fs::read(&self.path).await?;
        let demo = tf_demo_parser::Demo::new(&f);
        let parser = tf_demo_parser::DemoParser::new_with_analyser(
            demo.get_stream(),
            crate::analyser::Analyser::new(),
        );

        let (_, state) = parser.parse()?;
        self.inspection = Some(Arc::new(state));
        Ok(self.inspection.as_ref().unwrap().clone())
    }

    pub async fn has_replay(&self, replays_folder: &async_std::path::Path) -> bool {
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
            let _ = fs::remove_file(&bookmark_file).inspect_err(|e| {
                log::info!(
                    "Couldn't delete bookmark file {}, {}",
                    bookmark_file.display(),
                    e
                )
            });
            return;
        }

        let mut events = self.events.clone();
        events.sort_by_key(|e| e.tick);

        let container = EventContainer {
            events: events,
            notes: notes,
        };
        let json = serde_json::to_string_pretty(&container).unwrap();

        let _ = fs::write(&bookmark_file, json).inspect_err(|e| {
            log::warn!(
                "Couldn't save bookmark file {}, {}",
                bookmark_file.display(),
                e
            )
        });
    }

    pub fn tps(&self) -> f32 {
        self.header
            .as_ref()
            .map(|h| h.ticks as f32 / h.duration)
            .unwrap_or(Demo::TICKRATE)
    }

    pub async fn convert_to_replay(
        &mut self,
        replays_folder: &async_std::path::Path,
        title: &str,
    ) -> Result<()> {
        create_replay_index_file(replays_folder).await?;

        let replay_demo_path = replays_folder.join(&self.filename);
        fs::copy(&self.path, &replay_demo_path)?;

        let mut replay_handle: u32 = rand::thread_rng().gen();
        while replays_folder
            .join(format!("replay_{replay_handle}.dmx"))
            .exists()
            .await
        {
            replay_handle = rand::thread_rng().gen();
        }

        let create_date: chrono::DateTime<chrono::Local> =
            chrono::DateTime::from(self.created.clone().unwrap_or(SystemTime::now()));

        let kv_date = (create_date.day() - 1)
            | ((create_date.month() - 1) << 5)
            | ((create_date.year() as u32 - 2009) << 9);
        let kv_time =
            create_date.hour() | (create_date.minute() << 5) | (create_date.second() << 11);

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
",
            self.header.as_ref().unwrap().map,
            self.filename,
            self.header.as_ref().unwrap().duration
        );

        fs::write(
            replays_folder.join(format!("replay_{replay_handle}.dmx")),
            dmx_file_content,
        )?;
        Ok(())
    }
}

async fn create_replay_index_file(replay_folder: &async_std::path::Path) -> Result<()> {
    let index_path = replay_folder.join("replays.dmx");
    if !index_path.exists().await {
        fs::write(index_path, "\"root\"\n{\n\t\"version\"\t\"0\"\n}")?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct DemoManager {
    cache: HashMap<std::path::PathBuf, Demo>,
    demos: HashMap<String, Demo>,
}

impl DemoManager {
    pub fn new() -> Self {
        let cache = (|| {
            if !std::fs::exists("demos.cache")? {
                Ok(HashMap::new())
            } else {
                let data = std::fs::read("demos.cache")?;
                Ok(bitcode::deserialize(&data)?)
            }
        })()
        .unwrap_or_else(|e: anyhow::Error| {
            log::warn!("Failed to load demo cache {:?}", e);
            HashMap::new()
        });
        Self {
            cache: cache,
            demos: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.demos.clear();
    }

    pub fn load_demos(&mut self, folder_path: impl Into<std::path::PathBuf>) {
        let folder_path: std::path::PathBuf = folder_path.into().canonicalize().unwrap();
        self.demos.clear();
        for path in glob(&format!("{}/*.dem", folder_path.display().to_string())).unwrap() {
            let path = path.unwrap();
            let d = self
                .cache
                .get(&path)
                .cloned()
                .unwrap_or_else(|| Demo::new(std::path::Path::new(&path)));
            self.demos.insert(d.filename.to_owned(), d);
        }
        for demo in &mut self.demos.values_mut() {
            demo.read_data();
            self.cache.entry(demo.path.clone()).or_insert(demo.clone());
        }
        if let Err(e) = fs::write("demos.cache", bitcode::serialize(&self.cache).unwrap()) {
            log::warn!("Failed to save cache file: {e:?}");
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

    pub async fn delete_demo(&mut self, name: &str) {
        let demo = self.demos.remove(name).unwrap();

        let mut bookmark_path = demo.path.clone();
        bookmark_path.set_extension("json");

        let _ = async_std::task::spawn_blocking(move || {
            if let Err(e) = trash::delete(demo.path.as_path()) {
                log::info!("Couldn't delete {}, {}", demo.path.display(), e);
            }

            if let Err(e) = trash::delete(bookmark_path.as_path()) {
                log::info!("Couldn't delete {}, {}", bookmark_path.display(), e);
            }
        })
        .await;
    }

    pub async fn delete_empty_demos(&mut self) {
        let empties: Vec<String> = self
            .demos
            .values()
            .filter(|d| d.header.as_ref().map_or(true, |h| h.duration < 0.5))
            .map(|d| d.filename.clone())
            .collect();
        for demo in empties {
            self.delete_demo(&demo).await;
        }
    }

    pub async fn delete_unmarked_demos(&mut self) {
        let unmarkeds: Vec<String> = self
            .demos
            .values()
            .filter(|d| d.events.is_empty())
            .map(|d| d.filename.clone())
            .collect();
        for demo in unmarkeds {
            self.delete_demo(&demo).await;
        }
    }
}
