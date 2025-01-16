use tf_demo_parser::demo::header::Header;
use std::{fs, path::{Path, PathBuf}};
use bitbuffer::BitRead;
use glob::glob;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct EventContainer {
    events: Vec<Event>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    tick: u32,
    value: String,
    name: String,
}

#[derive(Debug)]
pub struct Demo {
    path: PathBuf,
    pub filename: String,
    pub header: Option<Header>,
    pub events: Vec<Event>,
}

impl Demo {
    pub fn new(path: &Path) -> Self {
        Demo{
            path: path.into(),
            filename: path.file_name().unwrap().to_str().unwrap().into(),
            header: None,
            events: Vec::new(),
        }
    }

    pub fn read_data(&mut self) {
        if let Some(_) = self.header {
            return;
        }
        let f = fs::read(&self.path).unwrap();

        let demo = tf_demo_parser::Demo::new(&f);
        self.header = Some(Header::read(&mut demo.get_stream()).unwrap());

        let mut bookmark_file = self.path.clone();
        bookmark_file.set_extension("json");

        let file = fs::read(bookmark_file);
        if let Ok(char_bytes) = file{
            let parsed: EventContainer = serde_json::from_slice(&char_bytes).unwrap();
            self.events = parsed.events;
        }
    }

    pub fn get_path(&self) -> String {
        self.path.display().to_string()
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

    pub fn load_demos(&mut self, folder_path: &String){
        self.demos.clear();
        for path in glob(&format!("{}/*.dem",folder_path)).unwrap() {
            self.demos.push(Demo::new(path.unwrap().as_path()));
        }
        self.demos.iter_mut().for_each(|d| d.read_data());
        log::debug!("{:#?}", self.demos);
    }

    pub fn get_demos(&self) -> &Vec<Demo> {
        &self.demos
    }
}