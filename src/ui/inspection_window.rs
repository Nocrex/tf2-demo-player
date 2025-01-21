use std::{collections::HashMap, sync::Arc};

use crate::demo_manager::Demo;
use adw::prelude::*;
use gtk::glib::markup_escape_text;
use itertools::Itertools;
use relm4::prelude::*;
use tf_demo_parser::{
    demo::{message::usermessage::ChatMessageKind, parser::analyser::Team},
    MatchState,
};

use super::ui_util;

pub struct InspectionModel {
    insp: Option<Arc<MatchState>>,
    tps: f32,
}

#[derive(Debug)]
pub enum InspectionOut {
    GotoTick(u32),
}

lazy_static::lazy_static! {
    static ref TEAM_ORDERING: HashMap<Team, usize> = HashMap::from_iter(vec![Team::Blue, Team::Red, Team::Spectator, Team::Other].iter().cloned().enumerate().map(|i|(i.1, i.0)));
}

#[relm4::component(pub)]
impl Component for InspectionModel {
    type Init = ();
    type Input = Demo;
    type Output = InspectionOut;
    type CommandOutput = Result<Arc<MatchState>, String>;

    view! {
        adw::Window{
            set_hide_on_close: true,
            set_title: Some("Demo Inspection Window"),
            set_height_request: 500,
            #[wrap(Some)]
            set_content = &adw::ToolbarView{
                add_top_bar = &adw::HeaderBar{
                    #[wrap(Some)]
                    set_title_widget = &adw::ViewSwitcher{
                        set_policy: adw::ViewSwitcherPolicy::Wide,
                        set_stack: Some(&stack),
                    },

                    pack_start = &gtk::Spinner{
                        #[watch]
                        set_spinning: model.insp.is_none(),
                    }
                },

                #[wrap(Some)]
                set_content: stack = &adw::ViewStack{
                    add = &gtk::ScrolledWindow{
                        #[watch]
                        set_child: Some(&{
                            let g_box = gtk::FlowBox::builder()
                                .orientation(gtk::Orientation::Horizontal)
                                .homogeneous(true)
                                .build();

                            model.insp.as_ref().inspect(|ms|{
                                for user in ms.users.values().sorted_by(|a,b|TEAM_ORDERING[&a.team].cmp(&TEAM_ORDERING[&b.team])){
                                    let row = adw::ActionRow::new();

                                    let sid64 = crate::util::steamid_32_to_64(&user.steam_id).unwrap_or_else(||{user.steam_id.clone()});
                                    let color = match &user.team {
                                        Team::Spectator | Team::Other => "848484",
                                        Team::Red => "e04a4a",
                                        Team::Blue => "3449d1",
                                    };
                                    row.set_title(&format!("<span foreground=\"#{color}\">{}</span>", markup_escape_text(&user.name)));
                                    row.set_subtitle(&format!("{}, {}", user.team, sid64));
                                    row.set_subtitle_selectable(true);

                                    let open_btn = gtk::Button::builder()
                                        .has_frame(false)
                                        .tooltip_text("Open steam profile")
                                        .icon_name(relm4_icons::icon_names::SYMBOLIC_LINK)
                                        .build();
                                    open_btn.connect_clicked(move |_|{
                                        let _ = opener::open_browser(format!("https://steamcommunity.com/profiles/{}", sid64));
                                    });
                                    row.add_suffix(&open_btn);
                                    row.set_activatable_widget(Some(&open_btn));
                                    g_box.append(&gtk::Frame::builder().child(&row).build());
                                }
                            });

                            g_box
                        })
                    } -> {
                        set_title: Some("Players"),
                        set_name: Some("players"),
                        set_icon_name: Some(relm4_icons::icon_names::PEOPLE),
                    },

                    add = &gtk::ScrolledWindow{
                        #[watch]
                        set_child: Some(&{
                            let g_box = gtk::ListBox::builder()
                                .show_separators(true)
                                .build();


                            model.insp.as_ref().inspect(|ms|{
                                for chat in &ms.chat{
                                    let row = adw::ActionRow::new();
                                    row.set_activatable(true);

                                    let kind =  match chat.kind{
                                        ChatMessageKind::ChatAll => "",
                                        ChatMessageKind::ChatTeam => "(Team) ",
                                        ChatMessageKind::ChatAllDead => "*DEAD* ",
                                        ChatMessageKind::ChatTeamDead => "(Team) *DEAD* ",
                                        ChatMessageKind::ChatAllSpec => "*SPEC* ",
                                        ChatMessageKind::NameChange => "[Name Change] ",
                                        ChatMessageKind::Empty => "",
                                    };

                                    row.set_title(&markup_escape_text(&chat.text));
                                    row.set_subtitle(&format!("{}{}", kind, markup_escape_text(&chat.from).as_str()));

                                    row.add_suffix(&gtk::Label::new(Some(&format!("{} ({})", crate::util::ticks_to_timestamp(chat.tick.into(), model.tps), chat.tick))));

                                    let copy_btn = gtk::Button::builder().icon_name(relm4_icons::icon_names::COPY).tooltip_text("Copy message").has_frame(false).build();
                                    let copy_txt = format!("{}{}: {}", kind, chat.from, chat.text);
                                    copy_btn.connect_clicked(move |_|{
                                        let disp = gtk::gdk::Display::default().unwrap();
                                        let clip = disp.clipboard();
                                        clip.set_text(&copy_txt);
                                    });

                                    row.add_suffix(&copy_btn);

                                    let row_sender = sender.clone();
                                    let tick: u32 = chat.tick.into();
                                    row.connect_activated(move |_|{
                                        let _ = row_sender.output(InspectionOut::GotoTick(tick));
                                    });
                                    g_box.append(&row);
                                }
                            });

                            g_box
                        })
                    } -> {
                        set_title: Some("Chat"),
                        set_name: Some("chat"),
                        set_icon_name: Some(relm4_icons::icon_names::CHAT_BUBBLES_TEXT),
                    },

                    add = &gtk::ScrolledWindow{
                        #[wrap(Some)]
                        set_child = &gtk::TextView{
                            set_editable: false,
                            #[wrap(Some)]
                            set_buffer = &gtk::TextBuffer{
                                #[watch]
                                set_text: &model.insp.as_ref().map_or("".to_owned(),|i|format!("{:#?}", i))
                            }
                        }
                    } -> {
                        set_title: Some("Dump")
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InspectionModel {
            insp: None,
            tps: Demo::TICKRATE,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) -> () {
        let mut message = message;
        self.tps = message.tps();
        self.insp = None;
        sender.oneshot_command(async move {
            message
                .full_analysis()
                .await
                .map_err(|e| format!("{:?}", e))
        });
        root.present();
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        if let Err(e) = &message {
            ui_util::notice_dialog(&root, "An error occured while parsing the demo", e);
        }
        self.insp = message.ok();
    }
}
