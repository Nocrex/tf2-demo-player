use gtk::glib::Object;
use gtk::glib;

use crate::demo_manager::Event;

glib::wrapper!{
    pub struct EventObject(ObjectSubclass<imp::EventObject>);
}

impl EventObject {
    pub fn new(event: &Event) -> Self {
        Object::builder()
            .property("name", &event.value)
            .property("tick", event.tick)
            .property("bookmark-type", &event.name).build()
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