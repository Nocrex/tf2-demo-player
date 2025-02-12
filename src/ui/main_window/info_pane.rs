use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use relm4::prelude::*;

use super::event_list::EventListModel;
use crate::demo_manager::Demo;
use crate::demo_manager::Event;
use crate::settings::Settings;

use super::controls::ControlsModel;
use super::controls::ControlsMsg;
use super::controls::ControlsOut;
use super::demo_infobox::DemoInfoboxModel;
use super::demo_infobox::DemoInfoboxMsg;
use super::demo_infobox::DemoInfoboxOut;
use super::event_dialog::EventDialogModel;
use super::event_dialog::EventDialogMsg;
use super::event_dialog::EventDialogOut;
use super::event_dialog::EventDialogParams;
use super::event_list::EventListMsg;
use super::event_list::EventListOut;
use super::RconAction;

#[derive(Debug)]
pub enum InfoPaneOut {
    Rcon(RconAction),
    Save(Demo),

    Update(Demo),
}

#[derive(Debug)]
pub enum InfoPaneMsg {
    Display(Option<Demo>, bool),
    Edited(bool),
    PlayheadTo(u32),
    SaveChanges,
    DiscardChanges,

    Rcon(RconAction),

    PlayheadMoved(u32),
    AddEvent,
    EditEvent(Event),

    DemoInspected(Demo),
}

pub struct InfoPaneModel {
    controls: AsyncController<ControlsModel>,
    infobox: Controller<DemoInfoboxModel>,
    event_list: Controller<EventListModel>,
    event_dialog: Controller<EventDialogModel>,

    demo: Option<Demo>,
    playhead_tick: u32,
}

#[relm4::component(pub)]
impl Component for InfoPaneModel {
    type Init = (adw::Window, Rc<RefCell<Settings>>);
    type Input = InfoPaneMsg;
    type Output = InfoPaneOut;
    type CommandOutput = ();

    view! {
        gtk::Box{
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,
            #[watch]
            set_sensitive: model.demo.is_some(),

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
        let controls =
            ControlsModel::builder()
                .launch(init.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    ControlsOut::Rcon(act) => InfoPaneMsg::Rcon(act),
                    ControlsOut::DemoInspected(dem) => InfoPaneMsg::DemoInspected(dem),

                    ControlsOut::SaveChanges => InfoPaneMsg::SaveChanges,
                    ControlsOut::DiscardChanges => InfoPaneMsg::DiscardChanges,
                    ControlsOut::PlayheadMoved(tick) => InfoPaneMsg::PlayheadMoved(tick),
                });

        let infobox = DemoInfoboxModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                DemoInfoboxOut::Dirty(state) => InfoPaneMsg::Edited(state),
            },
        );

        let event_list = EventListModel::builder().launch(init.0.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                EventListOut::JumpTo(event) => InfoPaneMsg::Rcon(RconAction::GotoEvent(event)),
                EventListOut::PlayheadTo(tick) => InfoPaneMsg::PlayheadTo(tick),
                EventListOut::AddEvent => InfoPaneMsg::AddEvent,
                EventListOut::EditEvent(event) => InfoPaneMsg::EditEvent(event),
                EventListOut::Dirty => InfoPaneMsg::Edited(true),
            },
        );

        let event_dialog = EventDialogModel::builder().launch(init.0).forward(
            event_list.sender(),
            |msg| match msg {
                EventDialogOut::Save(event, edit) => EventListMsg::Event(event, edit),
            },
        );

        let model = InfoPaneModel {
            demo: None,
            controls,
            infobox,
            event_list,
            event_dialog,
            playhead_tick: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match message {
            InfoPaneMsg::Display(demo, keep_playhead) => {
                self.demo = demo.clone();
                self.controls
                    .emit(ControlsMsg::SetDemo(demo.clone(), keep_playhead));
                self.infobox.emit(DemoInfoboxMsg::Display(demo.clone()));
                self.event_list.emit(EventListMsg::Display(demo));
            }
            InfoPaneMsg::Edited(dirty) => {
                self.controls.emit(ControlsMsg::SetDirty(dirty));
            }
            InfoPaneMsg::Rcon(act) => {
                let _ = sender.output(InfoPaneOut::Rcon(act));
            }
            InfoPaneMsg::PlayheadTo(tick) => {
                self.controls.emit(ControlsMsg::PlayheadMoved(tick as f64));
            }
            InfoPaneMsg::SaveChanges => {
                let mut demo = self.demo.clone().unwrap();
                demo.notes = self.infobox.model().notes.clone();
                demo.events = self.event_list.model().events();
                let _ = sender.output(InfoPaneOut::Save(demo));
            }
            InfoPaneMsg::DiscardChanges => {
                sender.input(InfoPaneMsg::Display(self.demo.clone(), true));
            }
            InfoPaneMsg::PlayheadMoved(tick) => self.playhead_tick = tick,
            InfoPaneMsg::AddEvent => {
                let mut event = Event::default();
                event.ev_type = "Bookmark".to_owned();
                event.tick = self.playhead_tick;

                let mut params = EventDialogParams::default();
                params.event = event;
                params.edit = false;
                params.length = self
                    .demo
                    .as_ref()
                    .and_then(|d| d.header.as_ref())
                    .map_or(u32::MAX, |h| h.ticks);
                self.event_dialog.emit(EventDialogMsg::Show(params))
            }
            InfoPaneMsg::EditEvent(event) => {
                let mut params = EventDialogParams::default();
                params.event = event;
                params.edit = true;
                params.length = self
                    .demo
                    .as_ref()
                    .and_then(|d| d.header.as_ref())
                    .map_or(u32::MAX, |h| h.ticks);
                self.event_dialog.emit(EventDialogMsg::Show(params))
            }
            InfoPaneMsg::DemoInspected(dem) => {
                if self
                    .demo
                    .as_ref()
                    .map_or(false, |d| d.filename == dem.filename)
                {
                    self.demo = Some(dem.clone());
                }
                let _ = sender.output(InfoPaneOut::Update(dem));
            }
        }
    }
}
