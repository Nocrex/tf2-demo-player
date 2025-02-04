use adw::prelude::*;
use relm4::prelude::*;

use crate::demo_manager::Demo;

#[derive(Debug)]
pub enum DemoInfoboxOut {
    Dirty(bool),
}

#[derive(Debug)]
pub enum DemoInfoboxMsg {
    Display(Option<Demo>),

    NotesChanged(String),
    OpenFolder,
}

pub struct DemoInfoboxModel {
    demo: Option<Demo>,
    pub notes: Option<String>,
}

#[relm4::component(pub)]
impl Component for DemoInfoboxModel {
    type Init = ();
    type Input = DemoInfoboxMsg;
    type Output = DemoInfoboxOut;
    type CommandOutput = ();

    view! {
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
                    set_secondary_icon_name: Some(relm4_icons::icon_names::SEARCH_FOLDER),
                    set_secondary_icon_tooltip_text: Some("Reveal in files"),
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
                    #[wrap(Some)]
                    set_buffer = &gtk::TextBuffer{
                        connect_changed[sender] => move |buf|{
                            sender.input(DemoInfoboxMsg::NotesChanged(
                                buf.text(&buf.start_iter(), &buf.end_iter(), true)
                                    .to_string(),
                            ));
                        }
                    }
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DemoInfoboxModel {
            demo: None,
            notes: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        //log::debug!("{:?}", message);
        match message {
            DemoInfoboxMsg::Display(demo) => {
                self.demo = demo;
                widgets.notes.buffer().set_text(
                    self.demo
                        .as_ref()
                        .and_then(|d| d.notes.as_ref())
                        .unwrap_or(&"".to_owned()),
                )
            }
            DemoInfoboxMsg::NotesChanged(notes) => {
                let new_notes: Option<String>;

                if notes.is_empty() {
                    new_notes = None;
                } else {
                    new_notes = Some(notes);
                }

                let _ = sender.output(DemoInfoboxOut::Dirty(
                    new_notes.as_ref() != self.demo.as_ref().and_then(|d| d.notes.as_ref()),
                ));

                self.notes = new_notes;
            }
            DemoInfoboxMsg::OpenFolder => {
                let path = self.demo.as_ref().unwrap().path.as_path();
                let _ = opener::reveal(path).inspect_err(|e| log::warn!("{}", e));
            }
        }
        self.update_view(widgets, sender);
    }
}
