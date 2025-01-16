use gtk::glib::Object;
use gtk::glib;

use crate::demo_manager::Event;

glib::wrapper!{
    pub struct EventObject(ObjectSubclass<imp::EventObject>);
}

impl EventObject {
    pub fn from(event: &Event) -> Self {
        Object::builder()
            .property("name", &event.value)
            .property("tick", event.tick)
            .property("bookmark-type", &event.name).build()
    }

    pub fn new(name: &str, bookmark_type: &str, tick: u32) -> Self {
        Object::builder()
            .property("name", name)
            .property("tick", tick)
            .property("bookmark-type", bookmark_type).build()
    }
}

impl Into<Event> for &EventObject {
    fn into(self) -> Event {
        Event { tick: self.tick(), value: self.name(), name: self.bookmark_type() }
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
    #[properties(wrapper_type = super::EventObject)]
    pub struct EventObject {
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get,set)]
        tick: Cell<u32>,
        #[property(get,set)]
        bookmark_type: RefCell<String>,
    }


    #[glib::object_subclass]
    impl ObjectSubclass for EventObject {
        const NAME: &'static str = "EventObject";
        type Type = super::EventObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EventObject {}
}