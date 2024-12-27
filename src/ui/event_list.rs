use adw::prelude::*;
use gtk::gio;
use relm4::prelude::*;

use crate::ui::event_object::EventObject;
use crate::demo_manager::{Demo, Event};

#[derive(Debug)]
pub enum EventListOut{
    JumpTo(Event),
}

#[derive(Debug)]
pub enum EventListMsg{
    Display(Option<Demo>),
}

pub struct EventListModel{
    list_model: gio::ListStore,
    selection_model: gtk::SingleSelection,
    
    demo: Option<Demo>
}

#[relm4::component(pub)]
impl SimpleComponent for EventListModel {
    type Init = ();
    type Input = EventListMsg;
    type Output = EventListOut;
    
    view!{
        adw::ToolbarView {
            #[wrap(Some)]
            set_content = &gtk::ScrolledWindow{
                #[name="list_view"]
                gtk::ListView{
                    set_show_separators: true,
                    #[wrap(Some)]
                    set_model = &gtk::SingleSelection{
                        set_model: Some(&model.list_model)
                    },
                    #[wrap(Some)]
                    set_factory = &gtk::SignalListItemFactory{
                        connect_setup[sender] => move |_,li|{
                            let list_item = li.downcast_ref::<gtk::ListItem>().unwrap();

                            let name_label = gtk::Label::builder().halign(gtk::Align::Start).build();
                            list_item.property_expression("item").chain_property::<EventObject>("name").bind(&name_label, "label", gtk::Widget::NONE);
                            
                            let seek_button = gtk::Button::builder().icon_name("find-location-symbolic").tooltip_text("Jump to this event").vexpand(false).valign(gtk::Align::Center).build();
                            let button_list_item = list_item.clone();
                            let button_sender = sender.clone();
                            seek_button.connect_clicked(move |b|{
                                let _ = button_sender.output(EventListOut::JumpTo(button_list_item.property::<EventObject>("item").into()));
                            });

                            let start_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(10).margin_start(10).margin_end(20).build();
                            start_box.append(&seek_button);
                            start_box.append(&name_label);

                            let type_label = gtk::Label::builder().halign(gtk::Align::Center).justify(gtk::Justification::Center).build();
                            list_item.property_expression("item").chain_property::<EventObject>("bookmark-type").bind(&type_label, "label", gtk::Widget::NONE);
                            
                            let time_label = gtk::Label::builder().halign(gtk::Align::End).justify(gtk::Justification::Right).margin_end(20).margin_start(20).build();
                            list_item.property_expression("item").chain_closure_with_callback(move |v|{
                                if ! v[1].is::<EventObject>(){
                                    return "".to_owned();
                                }

                                let evob: EventObject = v[1].get().unwrap();
                                format!("{} ({})", crate::util::sec_to_timestamp(evob.time()), evob.tick())
                            }).bind(&time_label, "label", gtk::Widget::NONE);

                            let cbox = gtk::CenterBox::builder().start_widget(&start_box).center_widget(&type_label).end_widget(&time_label).hexpand(true).height_request(40).build();
                            list_item.set_child(Some(&cbox));
                        }
                    },
                }
            },

            add_bottom_bar = &gtk::ActionBar{
                pack_start = &gtk::Button{
                    set_icon_name: "list-add-symbolic",
                    set_tooltip_text: Some("Add new event"),
                    #[watch]
                    set_sensitive: model.demo.is_some(),
                },

                pack_start = &gtk::Button{
                    set_icon_name: "list-remove-symbolic",
                    set_tooltip_text: Some("Remove selected event"),
                    #[watch]
                    set_sensitive: model.selection_model.selected_item().is_some(),
                },

                pack_start = &gtk::Button{
                    set_icon_name: "document-edit-symbolic",
                    set_tooltip_text: Some("Edit selected event"),
                    #[watch]
                    set_sensitive: model.selection_model.selected_item().is_some(),
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

        let model = EventListModel{
            selection_model: gtk::SingleSelection::new(Some(list_model.clone())),
            list_model,
            demo: None,
        };
        
        let widgets = view_output!();

        ComponentParts{model, widgets}
    }
    
    fn update(
            &mut self,
            message: Self::Input,
            sender: ComponentSender<Self>,
        ) {
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
        }
    }
}