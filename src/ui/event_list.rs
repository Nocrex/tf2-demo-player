use adw::prelude::*;
use gtk::gio;
use relm4::prelude::*;

use crate::demo_manager::{Demo, Event};
use crate::icon_names;
use crate::ui::event_object::EventObject;

#[derive(Debug)]
pub enum EventListOut {
    JumpTo(Event),
    PlayheadTo(u32),

    AddEvent,
    EditEvent(Event),
    Dirty,
}

#[derive(Debug)]
pub enum EventListMsg {
    Display(Option<Demo>),
    Event(Event, bool),

    Edit,
    Delete,
}

pub struct EventListModel {
    list_model: gio::ListStore,
    selection_model: gtk::SingleSelection,

    demo: Option<Demo>,
}

impl EventListModel {
    pub fn events(&self) -> Vec<Event> {
        self.list_model
            .iter::<gtk::glib::Object>()
            .map(|evob| evob.unwrap().downcast_ref::<EventObject>().unwrap().into())
            .collect::<Vec<Event>>()
    }
}

#[relm4::component(pub)]
impl SimpleComponent for EventListModel {
    type Init = adw::Window;
    type Input = EventListMsg;
    type Output = EventListOut;

    view! {
        adw::ToolbarView {
            #[wrap(Some)]
            set_content = &gtk::ScrolledWindow{
                #[name="list_view"]
                gtk::ListView{
                    set_show_separators: true,
                    connect_activate[sender] => move|list_view,ind|{
                        let ev = list_view.model().unwrap().item(ind).and_downcast::<EventObject>().unwrap();
                        let _ = sender.output(EventListOut::PlayheadTo(ev.tick()));
                    },
                    set_model: Some(&model.selection_model),

                    #[wrap(Some)]
                    set_factory = &gtk::SignalListItemFactory{
                        connect_setup[sender] => move |_,li|{
                            let list_item = li.downcast_ref::<gtk::ListItem>().unwrap();

                            let name_label = gtk::Label::builder().halign(gtk::Align::Start).build();
                            list_item.property_expression("item").chain_property::<EventObject>("name").bind(&name_label, "label", gtk::Widget::NONE);

                            let seek_button = gtk::Button::builder().icon_name(icon_names::PIN_LOCATION).tooltip_text("Jump to this event").vexpand(false).valign(gtk::Align::Center).build();
                            let button_list_item = list_item.clone();
                            let button_sender = sender.clone();
                            seek_button.connect_clicked(move |_|{
                                let _ = button_sender.output(EventListOut::JumpTo(button_list_item.property::<EventObject>("item").into()));
                            });

                            let start_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(10).margin_start(10).margin_end(20).build();
                            start_box.append(&seek_button);
                            start_box.append(&name_label);

                            let type_label = gtk::Label::builder().halign(gtk::Align::Center).justify(gtk::Justification::Center).build();
                            list_item.property_expression("item").chain_property::<EventObject>("bookmark-type").bind(&type_label, "label", gtk::Widget::NONE);

                            let time_label = gtk::Label::builder().halign(gtk::Align::End).justify(gtk::Justification::Right).margin_end(20).margin_start(20).build();
                            list_item.property_expression("item").chain_closure_with_callback(move |v|{
                                match v[1].get::<EventObject>(){
                                    Ok(evob) => format!("{} ({})", crate::util::sec_to_timestamp(evob.time()), evob.tick()),
                                    Err(_) => "".to_owned(),
                                }
                            }).bind(&time_label, "label", gtk::Widget::NONE);

                            let cbox = gtk::CenterBox::builder().start_widget(&start_box).center_widget(&type_label).end_widget(&time_label).hexpand(true).height_request(40).build();
                            list_item.set_child(Some(&cbox));
                        }
                    },
                }
            },

            add_bottom_bar = &gtk::ActionBar{
                pack_start = &gtk::Button{
                    set_icon_name: icon_names::PLUS,
                    set_tooltip_text: Some("Add new event"),
                    #[watch]
                    set_sensitive: model.demo.is_some(),
                    connect_clicked[sender] => move|_|{
                        let _ = sender.output(EventListOut::AddEvent);
                    },
                },

                pack_start = &gtk::Button{
                    set_icon_name: icon_names::MINUS,
                    set_tooltip_text: Some("Remove selected event"),
                    #[watch]
                    set_sensitive: model.selection_model.selected_item().is_some(),
                    connect_clicked => EventListMsg::Delete,
                },

                pack_start = &gtk::Button{
                    set_icon_name: icon_names::EDIT,
                    set_tooltip_text: Some("Edit selected event"),
                    #[watch]
                    set_sensitive: model.selection_model.selected_item().is_some(),
                    connect_clicked => EventListMsg::Edit,
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_model = gio::ListStore::new::<EventObject>();

        let model = EventListModel {
            selection_model: gtk::SingleSelection::new(Some(list_model.clone())),
            list_model,
            demo: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        //log::debug!("{:?}", message);
        match message {
            EventListMsg::Display(dem) => {
                self.list_model.remove_all();
                if let Some(demo) = &dem {
                    let tps = demo.tps();
                    for event in &demo.events {
                        self.list_model.append(&EventObject::from(event, tps));
                    }
                }
                self.demo = dem;
            }
            EventListMsg::Delete => {
                self.list_model.remove(self.selection_model.selected());
                //self.selection_model
                //    .emit_by_name::<()>("selection-changed", &[&0u32, &0u32]);
                let _ = sender.output(EventListOut::Dirty);
            }
            EventListMsg::Edit => {
                let _ = sender.output(EventListOut::EditEvent(
                    self.selection_model
                        .selected_item()
                        .and_downcast_ref::<EventObject>()
                        .unwrap()
                        .into(),
                ));
            }
            EventListMsg::Event(event, edit) => {
                if edit {
                    self.list_model.remove(self.selection_model.selected());
                }
                self.list_model.insert_sorted(
                    &EventObject::from(&event, self.demo.as_ref().unwrap().tps()),
                    |e1, e2| {
                        e1.downcast_ref::<EventObject>()
                            .unwrap()
                            .tick()
                            .cmp(&e2.downcast_ref::<EventObject>().unwrap().tick())
                    },
                );
                let _ = sender.output(EventListOut::Dirty);
            }
        }
    }
}
