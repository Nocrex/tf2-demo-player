#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod analyser;
mod demo_manager;
mod rcon_manager;
mod settings;

mod util;

use relm4::RelmApp;
mod ui;
use simplelog::{Config, TermLogger, WriteLogger};
use ui::DemoPlayerModel;

mod load_icons {
    use relm4::{
        gtk,
        gtk::{gio, glib},
    };

    pub fn setup() {
        let bytes = glib::Bytes::from_static(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/demoplayer.gresource"
        )));
        let resource = gio::Resource::from_data(&bytes).unwrap();
        gio::resources_register(&resource);

        gtk::init().unwrap();

        let display = gtk::gdk::Display::default().unwrap();
        let theme = gtk::IconTheme::for_display(&display);
        theme.set_search_path(&[]);
        theme.add_resource_path("/com/github/nocrex/tf2demoplayer/icons");

        relm4_icons::initialize_icons();
    }
}

#[async_std::main]
async fn main() {
    simplelog::CombinedLogger::init(if cfg!(debug_assertions) {
        vec![simplelog::TermLogger::new(
            log::LevelFilter::max(),
            Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        )]
    } else {
        vec![
            TermLogger::new(
                log::LevelFilter::Info,
                Config::default(),
                simplelog::TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            ),
            WriteLogger::new(
                log::LevelFilter::Info,
                Config::default(),
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("log.txt")
                    .unwrap(),
            ),
        ]
    })
    .unwrap();
    log::info!("Started");

    let panic_hndlr = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        log::error!("Rust panicked! -----\n{i}\n-------");
        panic_hndlr(i);
    }));

    load_icons::setup();

    let app = RelmApp::new("com.github.nocrex.tf2demoplayer");
    app.run_async::<DemoPlayerModel>(());
    log::info!("Exited")
}
