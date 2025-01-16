#![windows_subsystem = "windows"]
mod demo_manager;
mod inspector;
mod rcon_manager;
mod settings;

mod util;

use relm4::RelmApp;
mod ui;
use ui::DemoPlayerModel;

mod load_icons {
    use relm4::{
        gtk,
        gtk::{gio, glib},
    };

    pub fn setup() {
        let bytes = glib::Bytes::from_static(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/resources.gresource"
        )));
        let resource = gio::Resource::from_data(&bytes).unwrap();
        gio::resources_register(&resource);

        gtk::init().unwrap();

        let display = gtk::gdk::Display::default().unwrap();
        let theme = gtk::IconTheme::for_display(&display);
        theme.add_resource_path("/com/github/nocrex/tf2demoplayer/icons");
    }
}

#[async_std::main]
async fn main() {
    env_logger::init();

    load_icons::setup();

    let app = RelmApp::new("com.github.nocrex.tf2demoplayer");
    app.run_async::<DemoPlayerModel>(());
}
