use gtk::gio::ApplicationFlags;
use gtk::{prelude::*, ApplicationWindow};
use gtk::{glib, Application};

pub struct UI {
    app: Application,
}

impl UI {
    pub fn new() -> Self {
        let ui = UI {
            app: Application::new(None::<String>, ApplicationFlags::empty()),
        };
        ui.app.connect_activate(Self::build_ui);
        ui
    }

    fn build_ui(app: &Application){
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Demo Player").build();

        window.present();
    }

    pub fn run(&self) {
        self.app.run();
    }
}