use std::{collections::HashMap, sync::Arc};

use crate::analyser::{MatchEventType, MatchState};
use crate::demo_manager::Demo;
use adw::prelude::*;
use anyhow::Result;
use async_std::path::Path;
use event_list::{EventViewModel, EventViewMsg};
use gtk::glib::markup_escape_text;
use itertools::Itertools;
use relm4::prelude::*;
use tf_demo_parser::demo::{message::usermessage::ChatMessageKind, parser::analyser::Team};

use super::util;

mod event_list;

pub struct InspectionModel {
    demo: Demo,

    player_factories: HashMap<Team, FactoryVecDeque<PlayerRowModel>>,
    event_view: Controller<EventViewModel>,
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
                                        set_wrap: true,
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
                            FactoryVecDeque::builder().launch_default().forward(
                                sender.output_sender(),
                                |m| match m {
                                    PlayerRowOut::GotoTick(t) => InspectionOut::GotoTick(t),
                                },
                            ),
                        )
                    }),
            ),
            event_view: EventViewModel::builder()
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
                    self.update_display();
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
            self.update_display();
        }
    }
}

impl InspectionModel {
    fn update_display(&mut self) {
        if let Some(insp) = &self.demo.inspection {
            for user in &insp.users {
                self.player_factories
                    .get_mut(&user.last_team.unwrap_or_default())
                    .unwrap()
                    .guard()
                    .push_back(user.clone());
            }
        }
        self.event_view.emit(EventViewMsg::Show(
            self.demo.inspection.clone(),
            self.demo.tps(),
        ));
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
                    //gtk::Button { // TODO
                    //    set_label: "Show events",
                    //    set_has_frame: false,
                    //},
                }
            },
            add_row = &adw::ActionRow {
                set_title: "SteamIDs",
                add_suffix = &gtk::Label {
                    set_selectable: true,
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
