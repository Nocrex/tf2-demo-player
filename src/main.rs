#![windows_subsystem = "windows"]
mod demo_manager;
mod rcon_manager;
mod settings;

mod util;

use relm4::RelmApp;
mod ui;
use ui::DemoPlayerModel;

#[async_std::main]
async fn main() {
    env_logger::init();

    let app = RelmApp::new("com.github.nocrex.tfdemoplayer");
    app.run_async::<DemoPlayerModel>(());
}
