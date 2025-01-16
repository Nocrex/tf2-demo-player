use adw::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use relm4::prelude::*;

use crate::demo_manager::Demo;
use crate::ui::event_list::EventListModel;

use super::controls::ControlsModel;
use super::controls::ControlsMsg;
use super::controls::ControlsOut;
use super::demo_infobox::DemoInfoboxModel;
use super::demo_infobox::DemoInfoboxMsg;
use super::demo_infobox::DemoInfoboxOut;
use super::event_list::EventListMsg;
use super::window::RconAction;

#[derive(Debug)]
pub enum InfoPaneOut {
    Rcon(RconAction),
}

#[derive(Debug)]
pub enum InfoPaneMsg {
    Display(Option<Demo>),
    DemoEdited(Demo),
}

pub struct InfoPaneModel {
    controls: Controller<ControlsModel>,
    infobox: Controller<DemoInfoboxModel>,
    event_list: Controller<EventListModel>,

    displayed_demo: Option<Demo>,
    edited_demo: Option<Demo>,
}

#[relm4::component(pub)]
impl Component for InfoPaneModel {
    type Init = ();
    type Input = InfoPaneMsg;
    type Output = InfoPaneOut;
    type CommandOutput = ();

    view! {
        gtk::Box{
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,
            #[watch]
            set_sensitive: model.displayed_demo.is_some(),

            model.controls.widget(),

            gtk::Paned{
                set_orientation: gtk::Orientation::Horizontal,
                set_position: 500,
                set_shrink_end_child: false,
                set_shrink_start_child: false,

                #[wrap(Some)]
                set_start_child = model.infobox.widget(),

                #[wrap(Some)]
                set_end_child = model.event_list.widget(),
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let controls = ControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ControlsOut::Rcon(act) => todo!(),
                ControlsOut::ConvertReplay(name) => todo!(),
                ControlsOut::Inspect(name) => todo!(),

                ControlsOut::SaveChanges => todo!(),
                ControlsOut::DiscardChanges => todo!(),
            });

        let infobox = DemoInfoboxModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                DemoInfoboxOut::Dirty(dem) => InfoPaneMsg::DemoEdited(dem),
            },
        );

        let event_list = EventListModel::builder()
            .launch(())
            .forward(sender.input_sender(), |_| todo!());

        let model = InfoPaneModel {
            displayed_demo: None,
            edited_demo: None,
            controls,
            infobox,
            event_list,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            InfoPaneMsg::Display(demo) => {
                self.displayed_demo = demo.clone();
                self.edited_demo = demo.clone();
                self.controls.emit(ControlsMsg::SetDemo(demo.clone()));
                self.infobox.emit(DemoInfoboxMsg::Display(demo.clone()));
                self.event_list.emit(EventListMsg::Display(demo));
            }
            InfoPaneMsg::DemoEdited(demo) => {
                self.edited_demo = Some(demo.clone());
            }
        }
    }
}
