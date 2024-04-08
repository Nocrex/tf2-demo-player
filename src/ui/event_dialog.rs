use adw::prelude::*;
use gtk::{glib, glib::*};
use gtk::Button;
use crate::ui::Window;
use gtk_macros::get_widget;

use super::event_object::EventObject;

pub struct EventDialog {
    pub widget: adw::Dialog,
    parent: Window,
    event: EventObject,
    demo_length: u32,

    builder: gtk::Builder,
}

impl EventDialog {
    pub fn new(window: &Window, title: &str, event: &EventObject, demo_length: u32) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/github/nocrex/tfdemoplayer/event_dialog.ui");
        get_widget!(builder, adw::Dialog, event_dialog);

        let s = Self {
            widget: event_dialog,
            builder,
            parent: window.clone(),
            event: event.clone(),
            demo_length
        };
        get_widget!(s.builder, adw::PreferencesGroup, group);
        group.set_title(title);

        s.insert_event_data();
        s.connect_callbacks();
        s
    }

    pub fn callback(&self, cb: impl Fn(&str, &str, u32) + 'static) -> &Self{  
        get_widget!(self.builder, Button, save_button);
        get_widget!(self.builder, adw::EntryRow, name_row);
        get_widget!(self.builder, adw::EntryRow, type_row);
        get_widget!(self.builder, adw::SpinRow, tick_row);
        save_button.connect_clicked(clone!(@weak self.widget as dialog, @weak name_row, @weak type_row, @weak tick_row => move |_|{
            dialog.close();
            cb(name_row.text().as_str(), type_row.text().as_str(), tick_row.value() as u32);
        }));
        self
    }

    pub fn show(&self) {
        self.widget.present(&self.parent);
    }

    fn insert_event_data(&self){
        get_widget!(self.builder, adw::EntryRow, name_row);
        get_widget!(self.builder, adw::EntryRow, type_row);
        get_widget!(self.builder, adw::SpinRow, tick_row);
        name_row.set_text(&self.event.name());
        type_row.set_text(&self.event.bookmark_type());
        tick_row.set_range(0.0, self.demo_length as f64);
        tick_row.set_value(self.event.tick() as f64);
    }

    fn connect_callbacks(&self){
        get_widget!(self.builder, Button, cancel_button);
        cancel_button.connect_clicked(clone!(@weak self.widget as dialog => move |_|{
            dialog.close();
        }));
    }
}
