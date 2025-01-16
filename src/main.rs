mod demo_manager;
use demo_manager::DemoManager;

mod settings;
use settings::Settings;

mod rcon_manager;
use rcon_manager::{Command, RconManager};

mod util;

mod ui;
use ui::UI;

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut settings = Settings::load();

    let mut rcon_manager = RconManager::new(settings.rcon_pw);

    let mut dem_mgr = DemoManager::new();
    dem_mgr.load_demos(&settings.demo_folder_path);

    /*let first_demo = dem_mgr.get_demos().get(0).unwrap();

    log::debug!("Attempting to play {}", first_demo.filename);

    let res = rcon_manager.send_command(Command::PlayDemo(first_demo)).await;

    log::debug!("{:?}", res);*/

    let ui = UI::new();

    ui.run();
}