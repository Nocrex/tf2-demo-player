use std::time::{Duration, SystemTime};

use chrono::TimeZone;
use gtk::glib::Object;
use gtk::glib;

use crate::demo_manager::Demo;

glib::wrapper!{
    pub struct DemoObject(ObjectSubclass<imp::DemoObject>);
}

impl DemoObject {
    pub fn new(demo: &Demo) -> Self {
        let mut b = Object::builder()
            .property("name", demo.filename.to_owned())
            .property("bookmarks", demo.events.len() as u32);

        if let Some(header) = &demo.header{
            b = b.property("map", header.map.to_owned())
                .property("username", header.nick.to_owned())
                .property("duration", humantime::format_duration(Duration::from_secs(header.duration as u64)).to_string());
        }

        if let Some(meta) = &demo.metadata {
            b = b.property("created", meta.created().map_or("".to_owned(), |t|
                chrono::Local.timestamp_millis_opt(t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64).unwrap().format("%Y-%m-%d %H:%M:%S").to_string()
                ))
                .property("size", format!("{:.2}B", size_format::SizeFormatterBinary::new(meta.len())));
        }

        b.build()
    }
}

mod imp {
    use std::cell::RefCell;
    use std::cell::Cell;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::DemoObject)]
    pub struct DemoObject {
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        map: RefCell<String>,
        #[property(get, set)]
        username: RefCell<String>,
        #[property(get, set)]
        duration: RefCell<String>,
        #[property(get, set)]
        bookmarks: Cell<u32>,
        #[property(get, set)]
        size: RefCell<String>,
        #[property(get, set)]
        created: RefCell<String>,
    }


    #[glib::object_subclass]
    impl ObjectSubclass for DemoObject {
        const NAME: &'static str = "DemoObject";
        type Type = super::DemoObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DemoObject {}
}