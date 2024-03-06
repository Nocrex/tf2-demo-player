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
                .property("duration", header.duration);
        }

        if let Some(meta) = &demo.metadata {
            b = b.property("created", meta.created().map_or(0, |t|
                t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as i64
                ))
                .property("size", meta.len());
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
        duration: Cell<f32>,
        #[property(get, set)]
        bookmarks: Cell<u32>,
        #[property(get, set)]
        size: Cell<u64>,
        #[property(get, set)]
        created: Cell<i64>,
    }


    #[glib::object_subclass]
    impl ObjectSubclass for DemoObject {
        const NAME: &'static str = "DemoObject";
        type Type = super::DemoObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DemoObject {}
}