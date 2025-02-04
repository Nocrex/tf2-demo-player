use gtk::glib;
use gtk::glib::Object;
use relm4::prelude::*;

use crate::demo_manager::Event;

glib::wrapper! {
    pub struct EventObject(ObjectSubclass<imp::EventObject>);
}

impl EventObject {
    pub fn from(event: &Event, tps: f32) -> Self {
        Object::builder()
            .property("name", &event.title)
            .property("tick", event.tick)
            .property("bookmark-type", &event.ev_type)
            .property("tps", tps)
            .build()
    }

    pub fn new(name: &str, bookmark_type: &str, tick: u32, tps: f32) -> Self {
        Object::builder()
            .property("name", name)
            .property("tick", tick)
            .property("bookmark-type", bookmark_type)
            .property("tps", tps)
            .build()
    }

    pub fn time(&self) -> f32 {
        crate::util::ticks_to_sec(self.tick(), self.tps())
    }
}

impl Into<Event> for &EventObject {
    fn into(self) -> Event {
        Event {
            tick: self.tick(),
            title: self.name(),
            ev_type: self.bookmark_type(),
        }
    }
}

impl Into<Event> for EventObject {
    fn into(self) -> Event {
        (&self).into()
    }
}

mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use relm4::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EventObject)]
    pub struct EventObject {
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        tick: Cell<u32>,
        #[property(get, set)]
        bookmark_type: RefCell<String>,
        #[property[get,set]]
        tps: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EventObject {
        const NAME: &'static str = "EventObject";
        type Type = super::EventObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EventObject {}
}
