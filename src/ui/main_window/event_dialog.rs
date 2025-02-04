use adw::prelude::*;
use relm4::prelude::*;

use crate::demo_manager::Event;

#[derive(Debug)]
pub enum EventDialogOut {
    Save(Event, bool),
}

#[derive(Debug)]
pub enum EventDialogMsg {
    Save,
    Cancel,

    TitleChanged(String),
    TypeChanged(String),
    TickChanged(u32),

    Show(EventDialogParams),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct EventDialogParams {
    pub event: Event,
    pub edit: bool,
    pub length: u32,
}

pub struct EventDialogModel {
    params: EventDialogParams,
    title: String,
    ev_type: String,
    tick: u32,
    changed: bool,

    parent: adw::Window,
}

#[relm4::component(pub)]
impl Component for EventDialogModel {
    type Init = adw::Window;
    type Input = EventDialogMsg;
    type Output = EventDialogOut;
    type CommandOutput = ();

    view! {
        adw::Dialog{
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                set_margin_start: 10,
                set_margin_end: 10,

                #[wrap(Some)]
                set_content = &adw::PreferencesGroup{
                    #[watch]
                    set_title: if model.params.edit {"Edit event"} else {"Add event"},
                    adw::EntryRow{
                        set_title: "Title",
                        #[track = "model.changed"]
                        set_text: &model.params.event.title,
                        connect_changed[sender] => move |row| {
                            sender.input(EventDialogMsg::TitleChanged(row.text().to_string()));
                        }
                    },
                    adw::EntryRow{
                        set_title: "Type",
                        #[track = "model.changed"]
                        set_text: &model.params.event.ev_type,
                        connect_changed[sender] => move |row|{
                            sender.input(EventDialogMsg::TypeChanged(row.text().to_string()));
                        }
                    },
                    #[name="tick_row"]
                    adw::SpinRow{
                        set_title: "Tick",
                        #[wrap(Some)]
                        set_adjustment = &gtk::Adjustment{
                            set_page_increment: 10.0,
                            set_step_increment: 1.0,
                            #[track = "model.changed"]
                            set_value: model.params.event.tick.into(),
                            #[track = "model.changed"]
                            set_upper:model.params.length as f64,
                            set_lower: 0.0,
                            connect_value_changed[sender] => move |adj|{
                                sender.input(EventDialogMsg::TickChanged(adj.value() as u32));
                            },
                        }
                    }
                },

                add_bottom_bar = &gtk::ActionBar{
                    pack_end = &gtk::Button{
                        set_label: "Save",
                        connect_clicked => EventDialogMsg::Save,
                    },
                    pack_end = &gtk::Button{
                        set_label: "Cancel",
                        connect_clicked => EventDialogMsg::Cancel,
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<EventDialogModel>,
    ) -> ComponentParts<EventDialogModel> {
        let model = EventDialogModel {
            parent: init,
            params: EventDialogParams::default(),
            title: String::new(),
            ev_type: String::new(),
            tick: 0,
            changed: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<EventDialogModel>,
        root: &Self::Root,
    ) {
        self.changed = false;
        //log::debug!("{:?}", message);
        match message {
            EventDialogMsg::Show(params) => {
                root.present(Some(&self.parent));
                self.params = params;
                self.changed = true;
            }
            EventDialogMsg::Save => {
                let _ = sender.output(EventDialogOut::Save(
                    Event {
                        tick: self.tick.clone(),
                        title: self.title.clone(),
                        ev_type: self.ev_type.clone(),
                    },
                    self.params.edit,
                ));
                root.close();
            }
            EventDialogMsg::Cancel => {
                root.close();
            }
            EventDialogMsg::TitleChanged(title) => self.title = title,
            EventDialogMsg::TypeChanged(ev_type) => self.ev_type = ev_type,
            EventDialogMsg::TickChanged(tick) => self.tick = tick,
        }
        self.update_view(widgets, sender);
        if self.changed {
            // idk, this bs is needed to make the tick row show the correct tick on first open
            widgets.tick_row.set_value(self.params.event.tick as f64);
        }
    }
}
