use adw::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum AboutMsg {
    Open,
    Close,
}

pub struct AboutModel {
    parent: adw::Window,
}

#[relm4::component(pub)]
impl Component for AboutModel {
    type Init = adw::Window;
    type Input = AboutMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::AboutDialog{
            set_application_name: "TF2 Demo Player",
            set_developer_name: "Nocrex",
            set_website: "https://github.com/Nocrex/tf2-demo-player",
            set_version: env!("CARGO_PKG_VERSION"),
            set_application_icon: "tf2demoplayer",

            connect_close_attempt[sender] => move |_|{
                sender.input(AboutMsg::Close);
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { parent: init };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AboutMsg::Open => root.present(Some(&self.parent)),
            AboutMsg::Close => {
                root.close();
            }
        }
    }
}
