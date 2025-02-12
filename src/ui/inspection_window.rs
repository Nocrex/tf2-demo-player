use std::{collections::HashMap, sync::Arc};

use crate::analyser::{MatchEventType, MatchState};
use crate::demo_manager::Demo;
use adw::prelude::*;
use anyhow::Result;
use async_std::path::Path;
use gtk::glib::markup_escape_text;
use itertools::Itertools;
use relm4::prelude::*;
use tf_demo_parser::demo::{message::usermessage::ChatMessageKind, parser::analyser::Team};

use super::util;

pub struct InspectionModel {
    demo: Demo,
}

#[derive(Debug)]
pub enum InspectionOut {
    GotoTick(u32),
    DemoProcessed(Demo),
}

lazy_static::lazy_static! {
    static ref TEAM_ORDERING: HashMap<Team, usize> = HashMap::from_iter(vec![Team::Blue, Team::Red, Team::Spectator, Team::Other].iter().cloned().enumerate().map(|i|(i.1, i.0)));
}

#[relm4::component(pub)]
impl Component for InspectionModel {
    type Init = ();
    type Input = Demo;
    type Output = InspectionOut;
    type CommandOutput = Result<Arc<MatchState>>;

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
                        set_spinning: model.demo.inspection.is_none(),
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

                            model.demo.inspection.as_ref().inspect(|ms|{
                                for user in ms.users.iter().sorted_by(|a,b|TEAM_ORDERING[&a.last_team()].cmp(&TEAM_ORDERING[&b.last_team()])){
                                    let row = adw::ActionRow::new();

                                    let sid64 = user.steam_id.as_ref().map(|sid|crate::util::steamid_32_to_64(&sid).unwrap_or_else(||{sid.clone()})).unwrap_or_default();
                                    let color = match &user.last_team() {
                                        Team::Spectator | Team::Other => "848484",
                                        Team::Red => "e04a4a",
                                        Team::Blue => "3449d1",
                                    };
                                    row.set_title(&format!("<span foreground=\"#{color}\">{}</span>", markup_escape_text(user.name.as_ref().unwrap_or(&"".to_owned()))));
                                    row.set_subtitle(&format!("{}, {}", user.last_team(), sid64));
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


                            model.demo.inspection.as_ref().inspect(|ms|{
                                ms.events.iter().filter(|me|matches!(me.value, MatchEventType::Chat(_))).for_each(|me|{
                                    let chat = match &me.value {
                                        MatchEventType::Chat(c) => c,
                                        _ => panic!(),
                                    };
                                    let row = adw::ActionRow::new();
                                    row.set_activatable(true);

                                    let kind = match chat.kind{
                                        ChatMessageKind::ChatAll => "",
                                        ChatMessageKind::ChatTeam => "(Team) ",
                                        ChatMessageKind::ChatAllDead => "*DEAD* ",
                                        ChatMessageKind::ChatTeamDead => "(Team) *DEAD* ",
                                        ChatMessageKind::ChatAllSpec => "*SPEC* ",
                                        ChatMessageKind::NameChange => "[Name Change] ",
                                        ChatMessageKind::Empty => "",
                                    }.to_string();

                                    let color = match &chat.team {
                                        Some(t) => match t{
                                        Team::Spectator | Team::Other => "848484",
                                        Team::Red => "e04a4a",
                                        Team::Blue => "3449d1",
                                        }
                                        None => "848484",
                                    };

                                    row.set_title(&markup_escape_text(&chat.text));
                                    row.set_subtitle(&format!("{}<span foreground=\"#{color}\">{}</span>", kind, markup_escape_text(&chat.from).as_str()));

                                    row.add_suffix(&gtk::Label::new(Some(&format!("{} ({})", crate::util::ticks_to_timestamp(me.tick.into(), model.demo.tps()), me.tick))));

                                    let copy_btn = gtk::Button::builder().icon_name(relm4_icons::icon_names::COPY).tooltip_text("Copy message").has_frame(false).build();
                                    let copy_txt = format!("{}{}: {}", kind, chat.from, chat.text);
                                    copy_btn.connect_clicked(move |_|{
                                        let disp = gtk::gdk::Display::default().unwrap();
                                        let clip = disp.clipboard();
                                        clip.set_text(&copy_txt);
                                    });

                                    row.add_suffix(&copy_btn);

                                    let row_sender = sender.clone();
                                    let tick: u32 = me.tick.into();
                                    row.connect_activated(move |_|{
                                        let _ = row_sender.output(InspectionOut::GotoTick(tick));
                                    });
                                    g_box.append(&row);
                                });
                            });

                            g_box
                        })
                    } -> {
                        set_title: Some("Chat"),
                        set_name: Some("chat"),
                        set_icon_name: Some(relm4_icons::icon_names::CHAT_BUBBLES_TEXT),
                    },

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
            demo: Demo::new(Path::new("null.dem")),
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
        self.demo = message;
        if self.demo.inspection.is_none() {
            let mut dem = self.demo.clone();
            sender.oneshot_command(async move { dem.full_analysis().await });
        }
        root.present();
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        if let Err(e) = &message {
            util::notice_dialog(
                &root,
                "An error occured while parsing the demo",
                &e.to_string(),
            );
        }
        self.demo.inspection = message.ok();
        if self.demo.inspection.is_some() {
            let _ = sender.output(InspectionOut::DemoProcessed(self.demo.clone()));
        }
    }
}
