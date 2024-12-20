use adw::prelude::*;
use gtk::{glib, glib::*};
use gtk::Button;
use crate::rcon_manager::RconManager;
use crate::ui::Window;
use gtk_macros::get_widget;

pub struct SettingsWindow {
    pub widget: adw::PreferencesWindow,
    parent: Window,

    builder: gtk::Builder,
}

impl SettingsWindow {
    pub fn new(window: &Window) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/github/nocrex/tfdemoplayer/settings_window.ui");
        get_widget!(builder, adw::PreferencesWindow, settings_window);

        settings_window.set_transient_for(Some(window));

        let window_self = Self {
            widget: settings_window,
            builder,
            parent: window.clone()
        };

        window_self.insert_settings();
        window_self.connect_callbacks();
        window_self
    }

    pub fn show(&self) {
        self.widget.set_visible(true);
    }

    fn insert_settings(&self){
        get_widget!(self.builder, adw::PasswordEntryRow, rcon_pw_entry);
        get_widget!(self.builder, adw::SpinRow, event_predelay_entry);
        get_widget!(self.builder, adw::SwitchRow, doubleclick_play_switch);
        get_widget!(self.builder, adw::EntryRow, tf_folder_entry);

        let settings = self.parent.settings();
        rcon_pw_entry.set_text(&settings.borrow().rcon_pw);
        event_predelay_entry.set_value(settings.borrow().event_skip_predelay.into());
        doubleclick_play_switch.set_active(settings.borrow().doubleclick_play);
        tf_folder_entry.set_text(&settings.borrow().tf_folder_path);
    }

    fn connect_callbacks(&self){
        get_widget!(self.builder, adw::PasswordEntryRow, rcon_pw_entry);
        get_widget!(self.builder, Button, connection_test_button);
        get_widget!(self.builder, adw::ActionRow, connection_test_row);
        get_widget!(self.builder, adw::SpinRow, event_predelay_entry);
        get_widget!(self.builder, adw::SwitchRow, doubleclick_play_switch);
        get_widget!(self.builder, adw::EntryRow, tf_folder_entry);
        connection_test_button.connect_clicked(clone!(@weak connection_test_row, @weak rcon_pw_entry => move |b| {
            glib::spawn_future_local(clone!(@weak connection_test_row, @weak rcon_pw_entry, @weak b => async move {
                b.set_sensitive(false);
                let mut man = RconManager::new(rcon_pw_entry.text().to_string());
                let res = man.connect().await;
                let msg = match res  {
                    Ok(_) => "Connection Successful!".to_owned(),
                    Err(e) => match e {
                        rcon::Error::Auth => "Authorization failed, probably incorrect password".to_owned(),
                        rcon::Error::CommandTooLong => "Command too long?".to_owned(),
                        rcon::Error::Io(e) => format!("Connection error: {:?}",e)
                    }
                };
                connection_test_row.set_subtitle(&msg);
                b.set_sensitive(true);
            }));
        }));

        self.widget.connect_destroy(clone!(@weak self.parent as wnd, @weak rcon_pw_entry, @weak event_predelay_entry, @weak doubleclick_play_switch, @weak tf_folder_entry => move |_|{
            let settings = wnd.settings();
            settings.borrow_mut().rcon_pw = rcon_pw_entry.text().to_string();
            settings.borrow_mut().event_skip_predelay = event_predelay_entry.value() as f32;
            settings.borrow_mut().doubleclick_play = doubleclick_play_switch.is_active();
            settings.borrow_mut().tf_folder_path = tf_folder_entry.text().to_string();
            settings.borrow().save();

            let rcon = wnd.rcon_manager();
            rcon.replace(RconManager::new(settings.borrow().rcon_pw.clone()));
        }));

        tf_folder_entry.connect_changed(clone!(@weak tf_folder_entry, @weak self.widget as settings_wnd => move |_|{
           glib::spawn_future_local(clone!(@weak tf_folder_entry, @weak settings_wnd => async move {
                if !async_std::path::PathBuf::from(tf_folder_entry.text().to_string()).join("tf").is_dir().await {
                    tf_folder_entry.set_title("TF2 folder (invalid)");
                }else{
                    tf_folder_entry.set_title("TF2 folder");
                }
           })); 
        }));
    }
}
