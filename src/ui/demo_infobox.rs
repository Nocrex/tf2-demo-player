use adw::prelude::*;
use gtk::prelude::*;
use gtk::glib;
use relm4::prelude::*;

use crate::demo_manager::Demo;

#[derive(Debug)]
pub enum DemoInfoboxOut {
    Dirty(Demo),
}

#[derive(Debug)]
pub enum DemoInfoboxMsg {
    Display(Option<Demo>),

    NotesChanged(String),
    OpenFolder,
}

pub struct DemoInfoboxModel {
    demo: Option<Demo>,
    notes: gtk::TextView,
}

#[relm4::component(pub)]
impl SimpleComponent for DemoInfoboxModel {
    type Init = ();
    type Input = DemoInfoboxMsg;
    type Output = DemoInfoboxOut;

    view!{
        gtk::ScrolledWindow{
            gtk::Grid{
                set_width_request: 400,
                set_column_homogeneous: false,
                set_row_homogeneous: false,
                set_row_spacing: 10,
                set_column_spacing: 20,
                set_margin_start: 10,
                set_margin_end: 10,
                set_margin_top: 10,
                set_margin_bottom: 10,
                
                attach[0,0,1,1] = &gtk::Label{
                    set_label: "Name:",
                    set_halign: gtk::Align::Start,
                },
                
                attach[1,0,1,1] = &gtk::Entry{
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_editable: false,
                    set_secondary_icon_sensitive: true,
                    set_secondary_icon_activatable: true,
                    set_secondary_icon_name: Some("folder-open-symbolic"),
                    set_secondary_icon_tooltip_text: Some("Reveal in files"),
                    //connect_icon_press TODO
                    connect_icon_press[sender] => move |_,_|{
                        sender.input(DemoInfoboxMsg::OpenFolder);
                    },
                    #[watch]
                    set_text: model.demo.as_ref().map_or("", |d|&d.filename),
                },

                attach[0,1,1,1] = &gtk::Label{
                    set_label: "Map:",
                    set_halign: gtk::Align::Start,
                },
                
                attach[1,1,1,1] = &gtk::Entry{
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_editable: false,
                    #[watch]
                    set_text: model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or("", |h|&h.map),
                },

                attach[0,2,1,1] = &gtk::Label{
                    set_label: "Username:",
                    set_halign: gtk::Align::Start,
                },
                
                attach[1,2,1,1] = &gtk::Entry{
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_editable: false,
                    #[watch]
                    set_text: model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or("", |h|&h.nick),
                },

                attach[0,3,1,1] = &gtk::Label{
                    set_label: "Duration:",
                    set_halign: gtk::Align::Start,
                },
                
                attach[1,3,1,1] = &gtk::Entry{
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_editable: false,
                    #[watch]
                    set_text: &model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or("".to_owned(), |header|
                        format!("{} ({} ticks | {:.3} tps)", 
                            crate::util::sec_to_timestamp(header.duration), 
                            header.ticks, 
                            header.ticks as f32/header.duration
                        )),
                },

                attach[0,4,1,1] = &gtk::Label{
                    set_label: "Server:",
                    set_halign: gtk::Align::Start,
                },
                
                attach[1,4,1,1] = &gtk::Entry{
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_editable: false,
                    #[watch]
                    set_text: model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or("", |h|&h.server),
                },

                attach[0,5,1,1] = &gtk::Label{
                    set_label: "Notes:",
                    set_halign: gtk::Align::Start,
                },
                
                #[name="notes"]
                attach[0,6,2,1] = &gtk::TextView{
                    set_vexpand: true,
                },
            }
        }
    }
    
    fn init(
            _: Self::Init,
            root: Self::Root,
            sender: ComponentSender<Self>,
        ) -> ComponentParts<Self> {
        let mut model = DemoInfoboxModel{
            demo: None,
            notes: gtk::TextView::new(),
        };

        let widgets = view_output!();

        model.notes = widgets.notes.clone();


        let notes_sender = sender.clone();
        model.notes.buffer().connect_changed(move |buf|{
            notes_sender.input(DemoInfoboxMsg::NotesChanged(buf.text(&buf.start_iter(), &buf.end_iter(), true).to_string()));
        });

        ComponentParts{model, widgets}
    }

    fn update(
            &mut self,
            message: Self::Input,
            sender: ComponentSender<Self>,
        ) {
        match message {
            DemoInfoboxMsg::Display(demo) => {
                self.demo = demo;
                self.notes.buffer().set_text(self.demo.as_ref().and_then(|d|d.notes.as_ref()).unwrap_or(&"".to_owned()))
            },
            DemoInfoboxMsg::NotesChanged(notes) => {
                let new_notes: Option<String>;

                if notes.is_empty() {
                    new_notes = None;
                }else{
                    new_notes = Some(notes);
                }

                if new_notes.as_ref() != self.demo.as_ref().and_then(|d|d.notes.as_ref()){
                    self.demo.as_mut().unwrap().notes = new_notes;
                    let _ = sender.output(DemoInfoboxOut::Dirty(self.demo.clone().unwrap()));
                }
            },
            DemoInfoboxMsg::OpenFolder => {
                let path = self.demo.as_ref().unwrap().path.as_path();
                let _ = opener::reveal(path).inspect_err(|e|log::warn!("{}", e));
            }
        }
    }
}