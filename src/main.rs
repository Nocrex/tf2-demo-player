#![windows_subsystem = "windows"]
mod demo_manager;
mod inspector;
mod rcon_manager;
mod settings;

mod util;

use relm4::RelmApp;
mod ui;
use ui::DemoPlayerModel;

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

#[async_std::main]
async fn main() {
    env_logger::init();
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);

    let app = RelmApp::new("com.github.nocrex.tf2demoplayer");
    app.run_async::<DemoPlayerModel>(());
}
