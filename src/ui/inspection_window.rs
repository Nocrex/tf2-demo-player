use std::{collections::HashMap, sync::Arc};

use crate::analyser::{MatchEventType, MatchState, StableUserId};
use crate::demo_manager::Demo;
use adw::prelude::*;
use anyhow::Result;
use async_std::path::Path;
use gtk::glib::markup_escape_text;
use itertools::Itertools;
use relm4::{prelude::*, RelmContainerExt};
use tf_demo_parser::demo::{message::usermessage::ChatMessageKind, parser::analyser::Team};

use super::util;

pub struct InspectionModel {
    demo: Demo,

    player_factories: HashMap<Team, FactoryVecDeque<PlayerRowModel>>,
    event_view: Controller<EventListModel>,
}

#[derive(Debug)]
pub enum InspectionMsg {
    Inspect(Demo),
    SearchChanged(String),
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
    type Input = InspectionMsg;
    type Output = InspectionOut;
    type CommandOutput = Result<Arc<MatchState>>;

    view! {
        adw::Window{
            set_hide_on_close: true,
            set_title: Some("Demo Inspector"),
            set_height_request: 500,
            set_default_size: (800, 900),
            #[wrap(Some)]
            set_content = &adw::ToolbarView{
                add_top_bar = &adw::HeaderBar{
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle{
                        #[watch]
                        set_title: if model.demo.inspection.is_some() {&model.demo.filename} else { "" },
                    },

                    pack_start = &gtk::Spinner{
                        #[watch]
                        set_spinning: model.demo.inspection.is_none(),
                    }
                },

                add_top_bar = &adw::Clamp{
                    set_maximum_size: 500,
                    #[wrap(Some)]
                    set_child = &adw::ViewSwitcher{
                        set_policy: adw::ViewSwitcherPolicy::Wide,
                        set_stack: Some(&stack),
                    },
                },

                #[wrap(Some)]
                set_content: stack = &adw::ViewStack{
                    add = &gtk::ScrolledWindow {
                        #[wrap(Some)]
                        set_child = &gtk::Box{
                            set_orientation: gtk::Orientation::Vertical,

                            adw::Clamp{
                                set_maximum_size: 650,
                                #[wrap(Some)]
                                set_child = &gtk::Box {
                                    add_css_class: "card",
                                    set_orientation: gtk::Orientation::Vertical,

                                    gtk::Label {
                                        #[watch]
                                        set_label: &model.demo.inspection.as_ref().map(|i|i.server_info.name.clone()).unwrap_or_default(),
                                        set_selectable: true,
                                        set_focusable: false,
                                        set_margin_top: 20,
                                        set_margin_bottom: 2,
                                        add_css_class: "title-3",
                                    },
                                    gtk::Label{
                                        #[watch]
                                        set_label: &model.demo.header.as_ref().map(|h|h.server.clone()).unwrap_or_default(),
                                        set_margin_bottom: 2,
                                        set_selectable: true,
                                        set_focusable: false,
                                        add_css_class: "dim-label",
                                    },
                                    gtk::Label{
                                        #[watch]
                                        set_label: &model.demo.inspection.as_ref().map(|i|match i.server_info.is_stv {
                                            true => "STV Demo",
                                            false => "POV Demo",
                                        }).unwrap_or_default(),
                                        set_margin_bottom: 10,
                                        set_selectable: true,
                                        set_focusable: false,
                                        add_css_class: "dim-label",
                                    },
                                    gtk::Grid{
                                        set_column_spacing: 10,
                                        set_row_homogeneous: true,
                                        set_row_spacing: 10,
                                        set_margin_bottom: 10,
                                        set_margin_start: 10,
                                        set_margin_end: 10,
                                        set_halign: gtk::Align::Center,

                                        attach[0,0,1,1] = &gtk::Label {
                                            set_label: "Map:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[1,0,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.header.as_ref().map(|h|h.map.clone()).unwrap_or_default(),
                                        },

                                        attach[0,1,1,1] = &gtk::Label {
                                            set_label: "Recording User:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[1,1,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.header.as_ref().map(|h|h.nick.clone()).unwrap_or_default(),
                                        },

                                        attach[0,2,1,1] = &gtk::Label {
                                            set_label: "Parsed Duration:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[1,2,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.inspection.as_ref().map(|i|format!("{} ({} ticks)", crate::util::ticks_to_timestamp(i.end_tick.into(), 1.0/i.server_info.interval_per_tick), i.end_tick)).unwrap_or_default(),
                                        },

                                        attach[2,0,1,1] = &gtk::Label {
                                            set_label: "Unique Players:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[3,0,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.inspection.as_ref().map(|i|format!("{} (maxplayers {})", i.users.len(), i.server_info.maxplayers)).unwrap_or_default(),
                                        },

                                        attach[2,1,1,1] = &gtk::Label {
                                            set_label: "Server Tickrate:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[3,1,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.inspection.as_ref().map(|i|format!("{:.2} ({:.0} ms/tick)", 1.0/i.server_info.interval_per_tick, i.server_info.interval_per_tick*1000.0)).unwrap_or_default(),
                                        },

                                        attach[2,2,1,1] = &gtk::Label {
                                            set_label: "Server Platform:",
                                            set_halign: gtk::Align::Start,
                                        },
                                        attach[3,2,1,1] = &gtk::Entry {
                                            #[watch]
                                            set_text: &model.demo.inspection.as_ref().map(|i|match i.server_info.platform.as_str() {
                                                "l" => "Linux".to_string(),
                                                "w" => "Windows".to_string(),
                                                o => format!("Unknown (\"{o}\")")
                                            }).unwrap_or_default(),
                                        },
                                    },
                                }
                            },

                            adw::Clamp{
                                set_maximum_size: 650,
                                #[wrap(Some)]
                                set_child = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    gtk::Label{
                                        set_label: "Players",
                                        set_margin_top: 10,
                                        set_margin_bottom: 10,
                                        add_css_class: "title-3",
                                    },
                                    adw::Clamp {
                                        set_maximum_size: 300,
                                        gtk::SearchEntry{
                                            connect_search_changed[sender] => move |entry|{
                                                sender.input(InspectionMsg::SearchChanged(entry.text().to_string()));
                                            }
                                        }
                                    },
                                    gtk::Grid {
                                        set_hexpand: true,
                                        set_column_homogeneous: true,
                                        set_column_spacing: 10,
                                        set_margin_bottom: 50,
                                        attach[0,0,1,1] = &gtk::Box{
                                            set_orientation: gtk::Orientation::Vertical,
                                            gtk::Label {
                                                set_label: "Red",
                                                set_hexpand: true,
                                                set_margin_top: 10,
                                                add_css_class: "title-3",
                                            },
                                            model.player_factories.get(&Team::Red).unwrap().widget() -> &gtk::ListBox {
                                                set_margin_top: 10,
                                                set_selection_mode: gtk::SelectionMode::None,
                                                add_css_class: "boxed-list",
                                            },
                                        },
                                        attach[1,0,1,1] = &gtk::Box{
                                            set_orientation: gtk::Orientation::Vertical,
                                            gtk::Label {
                                                set_label: "Blue",
                                                set_hexpand: true,
                                                set_margin_top: 10,
                                                add_css_class: "title-3",
                                            },
                                            model.player_factories.get(&Team::Blue).unwrap().widget() -> &gtk::ListBox {
                                                set_margin_top: 10,
                                                set_selection_mode: gtk::SelectionMode::None,
                                                add_css_class: "boxed-list",
                                            },
                                        },
                                        attach[0,1,1,1] = &gtk::Box{
                                            set_orientation: gtk::Orientation::Vertical,
                                            gtk::Label {
                                                set_label: "Spectator",
                                                set_hexpand: true,
                                                set_margin_top: 10,
                                                add_css_class: "title-3",
                                            },
                                            model.player_factories.get(&Team::Spectator).unwrap().widget() -> &gtk::ListBox {
                                                set_margin_top: 10,
                                                set_selection_mode: gtk::SelectionMode::None,
                                                add_css_class: "boxed-list",
                                            },
                                        },
                                        attach[1,1,1,1] = &gtk::Box{
                                            set_orientation: gtk::Orientation::Vertical,
                                            gtk::Label {
                                                set_label: "Other",
                                                set_hexpand: true,
                                                set_margin_top: 10,
                                                add_css_class: "title-3",
                                            },
                                            model.player_factories.get(&Team::Other).unwrap().widget() -> &gtk::ListBox {
                                                set_margin_top: 10,
                                                set_selection_mode: gtk::SelectionMode::None,
                                                add_css_class: "boxed-list",
                                            },
                                        },
                                    }
                                }
                            }
                        }
                    } -> {
                        set_title: Some("Info"),
                        set_name: Some("info"),
                        set_icon_name: Some(relm4_icons::icon_names::INFO_OUTLINE),
                    },
                    add_titled_with_icon: (model.event_view.widget(), None, "Events", relm4_icons::icon_names::LIST_LARGE),
                    add = &gtk::ScrolledWindow{
                        #[watch]
                        set_child: Some(&{
                            let g_box = gtk::FlowBox::builder()
                                .orientation(gtk::Orientation::Horizontal)
                                .homogeneous(true)
                                .build();

                            model.demo.inspection.as_ref().inspect(|ms|{
                                for user in ms.users.iter().sorted_by(|a,b|TEAM_ORDERING[&a.last_team.unwrap_or_default()].cmp(&TEAM_ORDERING[&b.last_team.unwrap_or_default()])){
                                    let row = adw::ActionRow::new();

                                    let sid64 = user.steam_id.as_ref().map(|sid|crate::util::steamid_32_to_64(&sid).unwrap_or_else(||{sid.clone()})).unwrap_or_default();
                                    let color = match &user.last_team.unwrap_or_default() {
                                        Team::Spectator | Team::Other => "848484",
                                        Team::Red => "e04a4a",
                                        Team::Blue => "3449d1",
                                    };
                                    row.set_title(&format!("<span foreground=\"#{color}\">{}</span>", markup_escape_text(user.name.as_ref().unwrap_or(&"".to_owned()))));
                                    row.set_subtitle(&format!("{}, {}", user.last_team.unwrap_or_default(), sid64));
                                    row.set_subtitle_selectable(true);

                                    let open_btn = gtk::Button::builder()
                                        .has_frame(false)
                                        .tooltip_text("Open steam profile")
                                        .icon_name(relm4_icons::icon_names::SYMBOLIC_LINK)
                                        .build();
                                    open_btn.connect_clicked(move |_|{
                                        if let Err(e) = opener::open_browser(format!("https://steamcommunity.com/profiles/{}", sid64)){
                                            log::warn!("Failed to open browser, {e}");
                                        }
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
            demo: Demo::new(Path::new("empty")),
            player_factories: HashMap::from_iter(
                vec![Team::Red, Team::Blue, Team::Spectator, Team::Other]
                    .iter()
                    .map(|t| {
                        (
                            t.clone(),
                            FactoryVecDeque::builder().launch_default().detach(),
                        )
                    }),
            ),
            event_view: EventListModel::builder()
                .launch(None)
                .forward(sender.output_sender(), |t| InspectionOut::GotoTick(t)),
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
        match message {
            InspectionMsg::Inspect(demo) => {
                self.demo = demo;
                for (_, fac) in &mut self.player_factories {
                    fac.guard().clear();
                }
                if self.demo.inspection.is_none() {
                    let mut dem = self.demo.clone();
                    sender.oneshot_command(async move { dem.full_analysis().await });
                } else {
                    self.insert_players();
                }
                root.present();
            }
            InspectionMsg::SearchChanged(txt) => {
                let txt = txt.to_lowercase();
                for (_, fac) in &mut self.player_factories {
                    let txt = txt.clone();
                    fac.broadcast(PlayerRowMsg::SearchChanged(txt));
                }
            }
        }
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
            self.insert_players();
        }
    }
}

impl InspectionModel {
    fn insert_players(&mut self) {
        if let Some(insp) = &self.demo.inspection {
            for user in &insp.users {
                self.player_factories
                    .get_mut(&user.last_team.unwrap_or_default())
                    .unwrap()
                    .guard()
                    .push_back(user.clone());
            }
        }
    }
}

struct PlayerRowModel {
    player: crate::analyser::UserInfo,
    sid: Option<String>,

    matches_search: bool,
}

#[derive(Debug, Clone)]
enum PlayerRowMsg {
    OpenProfile,
    OpenSteamhistory,

    SearchChanged(String),
}

#[derive(Debug)]
enum PlayerRowOut {
    GotoTick(u32),
}

#[relm4::factory]
impl FactoryComponent for PlayerRowModel {
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();
    type Input = PlayerRowMsg;
    type Output = PlayerRowOut;
    type Init = crate::analyser::UserInfo;

    view! {
        #[root]
        adw::ExpanderRow {
            #[watch]
            set_visible: self.matches_search,
            set_title_selectable: true,
            set_title: &self.player.name.clone().unwrap_or_default(),
            set_subtitle: &self.sid.clone().unwrap_or_default(),
            add_row = &gtk::CenterBox {
                #[wrap(Some)]
                set_center_widget = &gtk::Box{
                    gtk::Box {
                        set_sensitive: self.sid.is_some() && !self.sid.as_ref().unwrap().contains("BOT"),
                        set_spacing: 10,
                        gtk::Button {
                            set_label: "Profile",
                            set_has_frame: false,
                            connect_clicked => PlayerRowMsg::OpenProfile,
                        },
                        gtk::Button {
                            set_label: "SteamHistory",
                            set_has_frame: false,
                            connect_clicked => PlayerRowMsg::OpenSteamhistory,
                        },
                    },
                    gtk::Button {
                        set_label: "Show events",
                        set_has_frame: false,
                    },
                }
            },
            add_row = &adw::ActionRow {
                set_title: "SteamIDs",
                add_suffix = &gtk::Label {
                    set_selectable: true,
                    set_focusable: false,
                    set_justify: gtk::Justification::Right,
                    #[watch]
                    set_label: &format!("{}\n{}", self.sid.clone().unwrap_or_default(), self.player.steam_id.clone().unwrap_or_default()),
                }
            },
            add_row = &adw::ActionRow {
                set_title: "Classes",
                #[watch]
                set_subtitle: &format!("{} switches", self.player.class_switches.len()),
                add_suffix = &gtk::Label{
                    set_margin_top: 10,
                    set_margin_bottom: 10,
                    set_selectable: true,
                    set_focusable: false,
                    set_wrap: true,
                    set_justify: gtk::Justification::Right,
                    set_use_markup: true,
                    #[watch]
                    set_label: &self.player.class_switches.iter()
                        .map(|c|format!("<a href=\"{0}\">{0}</a>: {1}", c.0, c.1.to_string()))
                        .join("\n"),
                    connect_activate_link[sender] => move |_, tick|{
                        let _ = sender.output(PlayerRowOut::GotoTick(tick.parse().unwrap()));
                        gtk::glib::Propagation::Stop
                    },
                }
            },
            add_row = &adw::ActionRow{
                set_title: "Connection Events",
                add_suffix = &gtk::Label {
                    set_margin_top: 10,
                    set_margin_bottom: 10,
                    set_selectable: true,
                    set_focusable: false,
                    set_wrap: true,
                    set_justify: gtk::Justification::Right,
                    set_use_markup: true,
                    #[watch]
                    set_label: &self.player.connection_events.iter()
                        .map(|c|format!("<a href=\"{0}\">{0}</a>: {1}", c.0, match &c.1 {
                            crate::analyser::ConnectionEventType::Join => "Connected".to_string(),
                            crate::analyser::ConnectionEventType::Leave(reason) => format!("Disconnected\n<small>({reason})</small>"),
                        }))
                        .join("\n"),
                    connect_activate_link[sender] => move |_, tick|{
                        let _ = sender.output(PlayerRowOut::GotoTick(tick.parse().unwrap()));
                        gtk::glib::Propagation::Stop
                    },
                },
            },
        }
    }

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        let sid = init
            .steam_id
            .clone()
            .map(|s| crate::util::steamid_32_to_64(&s).unwrap_or(s));
        Self {
            player: init,
            sid,
            matches_search: true,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            PlayerRowMsg::OpenProfile => {
                if let Err(e) = opener::open_browser(format!(
                    "https://steamcommunity.com/profiles/{}",
                    self.sid.clone().unwrap_or_default()
                )) {
                    log::warn!("Failed to open browser, {e}");
                }
            }
            PlayerRowMsg::OpenSteamhistory => {
                if let Err(e) = opener::open_browser(format!(
                    "https://steamhistory.net/id/{}",
                    self.sid.clone().unwrap_or_default()
                )) {
                    log::warn!("Failed to open browser, {e}");
                }
            }
            PlayerRowMsg::SearchChanged(search) => {
                let search = search.to_lowercase();
                self.matches_search = self
                    .player
                    .name
                    .as_ref()
                    .map_or(false, |n| n.to_lowercase().contains(&search))
                    || self.sid.as_ref().map_or(false, |s| s.contains(&search));
            }
        }
    }
}

#[derive(Debug)]
enum EventListFilter {
    Chat,
    Deaths,
    Rounds,
    Connections,
    Votes,
    Team,
    Class,
}

#[derive(Debug)]
enum EventListMsg {
    Filter(EventListFilter),
    Show(Option<Arc<MatchState>>),
}
struct EventListModel {
    inspection: Option<Arc<MatchState>>,

    show_chat: bool,
    show_deaths: bool,
    show_rounds: bool,
    show_connections: bool,
    show_votes: bool,
    show_team: bool,
    show_class: bool,
}

#[relm4::component]
impl SimpleComponent for EventListModel {
    type Init = Option<(Arc<MatchState>, StableUserId)>;
    type Input = EventListMsg;
    type Output = u32;

    view! {
        gtk::Box{
            set_orientation: gtk::Orientation::Vertical,
            gtk::CenterBox{
                #[wrap(Some)]
                set_end_widget = &gtk::Box{
                    add_css_class: "linked",
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_chat,
                        set_tooltip_text: Some("Chat"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Chat),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_deaths,
                        set_tooltip_text: Some("Deaths"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Deaths),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_rounds,
                        set_tooltip_text: Some("Rounds"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Rounds),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_connections,
                        set_tooltip_text: Some("Connections"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Connections),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_votes,
                        set_tooltip_text: Some("Votes"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Votes),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_team,
                        set_tooltip_text: Some("Team Switches"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Team),
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.show_class,
                        set_tooltip_text: Some("Class switches"),
                        connect_clicked => EventListMsg::Filter(EventListFilter::Class),
                    },
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            inspection: None,
            show_chat: true,
            show_deaths: true,
            show_rounds: true,
            show_connections: true,
            show_votes: true,
            show_team: true,
            show_class: true,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            EventListMsg::Filter(event_list_filter) => match event_list_filter {
                EventListFilter::Chat => self.show_chat = !self.show_chat,
                EventListFilter::Deaths => self.show_deaths = !self.show_deaths,
                EventListFilter::Rounds => self.show_rounds = !self.show_rounds,
                EventListFilter::Connections => self.show_connections = !self.show_connections,
                EventListFilter::Votes => self.show_votes = !self.show_votes,
                EventListFilter::Team => self.show_team = !self.show_team,
                EventListFilter::Class => self.show_class = !self.show_class,
            },
            EventListMsg::Show(match_state) => todo!(),
        }
    }
}
