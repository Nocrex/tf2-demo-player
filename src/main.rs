mod demo_manager;
use demo_manager::DemoManager;

mod settings;
use settings::Settings;

mod rcon_manager;
use rcon_manager::RconManager;

mod util;

mod ui;
use ui::UI;

use gtk::gio;
use async_std::task;
fn main() {
    env_logger::init();
    gio::resources_register_include!("ui.gresource")
        .expect("Failed to register resources.");
    let settings = Settings::load();

    let rcon_manager = RconManager::new(settings.rcon_pw.clone());

    let mut dem_mgr = DemoManager::new();
    task::block_on(dem_mgr.load_demos(&settings.demo_folder_path));

    let ui = UI::new(rcon_manager, dem_mgr, settings);

    ui.run();
}