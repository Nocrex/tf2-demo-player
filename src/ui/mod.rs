use gtk::gio::ApplicationFlags;
use gtk::{prelude::*, Application};

use crate::demo_manager::DemoManager;
use crate::rcon_manager::RconManager;
use crate::settings::Settings;

mod demo_object;

mod event_object;

mod window;
use window::Window;

mod settings_window;

pub struct UI {
    app: Application,
}

impl UI {
    pub fn new(rcon: RconManager, demos: DemoManager, settings: Settings) -> UI {
        let ui = UI {
            app: Application::new(Some("com.github.nocrex.tfdemoplayer"), ApplicationFlags::empty()),
        };

        unsafe{
            ui.app.set_data("demo_manager", demos);
            ui.app.set_data("settings", settings);
            ui.app.set_data("rcon_manager", rcon);
        }

        ui.app.connect_activate(|app|{
            let wnd = Window::new(app);
            wnd.present();
        });
        ui
    }

    pub fn run(&self) {
        self.app.run();
    }
}