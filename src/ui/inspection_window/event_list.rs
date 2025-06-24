use std::sync::Arc;

use crate::{
    analyser::{
        ConnectionEventType, CritType, MatchEvent, MatchEventType, MatchState, StableUserId, Vote,
        VoteTeam,
    },
    util,
};
use adw::prelude::*;
use itertools::Itertools;
use relm4::{gtk::glib::markup_escape_text, prelude::*};
use tf_demo_parser::demo::{message::usermessage::ChatMessageKind, parser::analyser::Team};

fn get_team_color_string(team: Option<&Team>) -> &str {
    let dark = adw::StyleManager::default().is_dark();
    match team {
        Some(t) => match t {
            Team::Spectator | Team::Other => "#848484",
            Team::Red => "#e04a4a",
            Team::Blue => {
                if dark {
                    "#6aaef7"
                } else {
                    "#3449d1"
                }
            }
        },
        None => "#848484",
    }
}

fn get_message_kind_prefix(kind: &ChatMessageKind) -> &str {
    match kind {
        ChatMessageKind::ChatAll => "",
        ChatMessageKind::ChatTeam => "(Team) ",
        ChatMessageKind::ChatAllDead => "*DEAD* ",
        ChatMessageKind::ChatTeamDead => "(Team) *DEAD* ",
        ChatMessageKind::ChatAllSpec => "*SPEC* ",
        ChatMessageKind::NameChange => "[Name Change] ",
        ChatMessageKind::Empty => "",
    }
}

fn vote_table(vote: &Vote) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_row_spacing(10);
    grid.set_column_spacing(30);

    for (i, option) in vote.options.iter().enumerate() {
        let lab = gtk::Label::new(Some(&format!("<b>{option}</b>")));
        lab.set_use_markup(true);

        grid.attach(&lab, i as i32 * 2, 0, 2, 1);
    }

    for (o, votes) in vote.votes.iter().into_group_map_by(|v| v.2) {
        for (i, vot) in votes.iter().enumerate() {
            let name = gtk::Label::new(Some(&vot.1));
            name.set_halign(gtk::Align::Start);
            let tick = gtk::Label::new(Some(&vot.0.to_string()));
            tick.set_halign(gtk::Align::End);

            grid.attach(&name, o as i32 * 2, i as i32 + 1, 1, 1);
            grid.attach(&tick, o as i32 * 2 + 1, i as i32 + 1, 1, 1);
        }
    }

    grid
}

#[derive(Debug)]
pub enum EventListFilterChange {
    Chat,
    Deaths,
    Rounds,
    Connections,
    Votes,
    Team,
    Class,
}

#[derive(Debug, Clone)]
pub struct EventListFilter {
    show_chat: bool,
    show_deaths: bool,
    show_rounds: bool,
    show_connections: bool,
    show_votes: bool,
    show_team: bool,
    show_class: bool,
}

impl EventListFilter {
    fn reset(&mut self) {
        self.show_chat = true;
        self.show_deaths = false;
        self.show_rounds = false;
        self.show_connections = false;
        self.show_votes = false;
        self.show_team = false;
        self.show_class = false;
    }
}

#[derive(Debug)]
pub enum EventViewMsg {
    Filter(EventListFilterChange),
    Show(Option<Arc<MatchState>>, f32),
    Selected(DynamicIndex),
}
pub struct EventViewModel {
    inspection: Option<Arc<MatchState>>,
    tps: f32,

    list_model: FactoryVecDeque<EventRowModel>,
    event_dialog: Controller<EventDialogModel>,

    filter: EventListFilter,
}

#[relm4::component(pub)]
impl SimpleComponent for EventViewModel {
    type Init = Option<(Arc<MatchState>, StableUserId)>;
    type Input = EventViewMsg;
    type Output = u32;

    view! {
        gtk::Box{
            set_orientation: gtk::Orientation::Vertical,
            gtk::CenterBox{
                set_margin_all: 10,
                //#[wrap(Some)]
                //set_start_widget = &gtk::SearchEntry{
                //},
                #[wrap(Some)]
                set_end_widget = &gtk::Box{
                    add_css_class: "linked",
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_chat,
                        set_tooltip_text: Some("Chat"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Chat),
                        set_icon_name: relm4_icons::icon_names::CHAT_BUBBLES_TEXT,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_deaths,
                        set_tooltip_text: Some("Deaths"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Deaths),
                        set_icon_name: relm4_icons::icon_names::VIOLENCE3,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_rounds,
                        set_tooltip_text: Some("Rounds"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Rounds),
                        set_icon_name: relm4_icons::icon_names::FLAG_FILLED,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_connections,
                        set_tooltip_text: Some("Connections"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Connections),
                        set_icon_name: relm4_icons::icon_names::NETWORK_SERVER,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_votes,
                        set_tooltip_text: Some("Votes"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Votes),
                        set_icon_name: relm4_icons::icon_names::CHECK_ROUND_OUTLINE,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_team,
                        set_tooltip_text: Some("Team Switches"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Team),
                        set_icon_name: relm4_icons::icon_names::HORIZONTAL_ARROWS,
                    },
                    gtk::ToggleButton {
                        #[watch]
                        set_active: model.filter.show_class,
                        set_tooltip_text: Some("Class switches"),
                        connect_clicked => EventViewMsg::Filter(EventListFilterChange::Class),
                        set_icon_name: relm4_icons::icon_names::DISCOVER,
                    },
                }
            },
            gtk::ScrolledWindow{
                #[wrap(Some)]
                set_child = model.list_model.widget() {
                    set_vexpand: true,
                    set_show_separators: true,
                },
            },
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            list_model: FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |ind| EventViewMsg::Selected(ind)),
            event_dialog: EventDialogModel::builder()
                .launch(root.clone())
                .forward(sender.output_sender(), |t| t),
            inspection: None,
            tps: 0.0,
            filter: EventListFilter {
                show_chat: true,
                show_deaths: false,
                show_rounds: false,
                show_connections: false,
                show_votes: false,
                show_team: false,
                show_class: false,
            },
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            EventViewMsg::Filter(event_list_filter) => {
                match event_list_filter {
                    EventListFilterChange::Chat => self.filter.show_chat = !self.filter.show_chat,
                    EventListFilterChange::Deaths => {
                        self.filter.show_deaths = !self.filter.show_deaths
                    }
                    EventListFilterChange::Rounds => {
                        self.filter.show_rounds = !self.filter.show_rounds
                    }
                    EventListFilterChange::Connections => {
                        self.filter.show_connections = !self.filter.show_connections
                    }
                    EventListFilterChange::Votes => {
                        self.filter.show_votes = !self.filter.show_votes
                    }
                    EventListFilterChange::Team => self.filter.show_team = !self.filter.show_team,
                    EventListFilterChange::Class => {
                        self.filter.show_class = !self.filter.show_class
                    }
                };
                self.list_model
                    .broadcast(EventRowMsg::Filter(self.filter.clone()));
            }
            EventViewMsg::Show(match_state, tps) => {
                self.inspection = match_state;
                self.tps = tps;
                let mut g = self.list_model.guard();
                g.clear();
                self.filter.reset();
                if let Some(state) = &self.inspection {
                    for ev in &state.events {
                        g.push_back((ev.clone(), self.tps, state.clone()));
                    }
                }
                g.broadcast(EventRowMsg::Filter(self.filter.clone()));
            }
            EventViewMsg::Selected(ind) => self.event_dialog.emit(EventDialogMsg::Update(Some((
                self.inspection.clone().unwrap(),
                ind.current_index(),
                self.tps,
            )))),
        }
    }
}

#[derive(Debug)]
struct EventDialogModel {
    inspection: Option<Arc<MatchState>>,
    tps: f32,

    selected_event: Option<usize>,
    parent: gtk::Box,
}

#[derive(Debug)]
enum EventDialogMsg {
    Update(Option<(Arc<MatchState>, usize, f32)>),
    CopyMessage,
    Goto,
}

#[relm4::component]
impl Component for EventDialogModel {
    type Init = gtk::Box;
    type Input = EventDialogMsg;
    type Output = u32;
    type CommandOutput = ();

    view! {
        adw::Dialog{
            set_can_close: false,
            set_follows_content_size: true,
            set_presentation_mode: adw::DialogPresentationMode::BottomSheet,
            connect_close_attempt => EventDialogMsg::Update(None),
            #[wrap(Some)]
            set_child =
                match model.inspection.as_ref()
                    .and_then(|i|model.selected_event.as_ref().map(|e|(i,e)))
                    .map(|(s,i)|&s.events[*i])
                {
                    Some(ev) => gtk::Box{
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 10,
                        gtk::CenterBox{
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label{
                                #[watch]
                                set_label: ev.value.get_type_string(),
                                add_css_class: "title-3",
                                set_halign: gtk::Align::Start,
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Button{
                                set_icon_name: "find-location-symbolic",
                                set_tooltip_text: Some("Set playbar to event"),
                                connect_clicked => EventDialogMsg::Goto,
                            }
                        },
                        gtk::Label{
                            set_selectable: true,
                            #[watch]
                            set_label: &format!("Tick {}", ev.tick.to_string()),
                            add_css_class: "dimmed",
                            set_halign: gtk::Align::Start,
                            set_margin_bottom: 10,
                        },
                        container_add = match &ev.value {
                            MatchEventType::Kill(kill) => gtk::Grid{
                                set_row_spacing: 10,
                                set_column_spacing: 10,
                                attach[0,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Killer:",
                                    #[watch]
                                    set_visible: kill.killer.is_some(),
                                },
                                attach[1,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &format!("{}{}",
                                        kill.killer.as_ref()
                                            .and_then(|k|model.inspection.as_ref().unwrap().users[k].name.clone())
                                            .map_or_else(||"unknown".to_string(), |n|n),
                                        if kill.domination {" (domination)"} else if kill.revenge {" (revenge)"} else {""}
                                    ),
                                    #[watch]
                                    set_visible: kill.killer.is_some(),
                                },
                                attach[0,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Assister:",
                                    #[watch]
                                    set_visible: kill.assister.is_some(),
                                },
                                attach[1,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &format!("{}{}",
                                        kill.assister.as_ref()
                                            .and_then(|k|model.inspection.as_ref().unwrap().users[k].name.clone())
                                            .map_or_else(||"unknown".to_string(), |n|n),
                                        if kill.assist_dom {" (domination)"} else if kill.assist_revg {" (revenge)"} else {""}
                                    ),
                                    #[watch]
                                    set_visible: kill.assister.is_some(),
                                },
                                attach[0,2,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Victim:",
                                },
                                attach[1,2,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &format!("{}{}",
                                        model.inspection.as_ref().unwrap()
                                        .users[&kill.victim].name.clone()
                                        .map_or_else(||"unknown".to_string(), |n|n),
                                        if kill.deadringer {" (death feigned)"} else {""}
                                    ),
                                },
                                attach[0,3,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Weapon:",
                                },
                                attach[1,3,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &format!("{}{}",
                                        kill.weapon,
                                        match kill.crit_type {
                                            CritType::None => "",
                                            CritType::Mini => " (mini-crit)",
                                            CritType::Full => " (crit)",
                                            CritType::Unknown(_) => " (unknown crit type)",
                                        }
                                    ),
                                },
                            }
                            MatchEventType::RoundEnd(r) => gtk::Grid{
                                set_row_spacing: 10,
                                set_column_spacing: 10,
                                attach[0,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Winner:",
                                },
                                attach[1,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &r.winner.to_string(),
                                },
                                attach[0,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Round Length:",
                                },
                                attach[1,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &util::sec_to_timestamp(r.length),
                                },
                            }
                            MatchEventType::Chat(chat) => gtk::Box{
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 10,
                                set_margin_top: 10,
                                gtk::Label{
                                    set_selectable: true,
                                    set_use_markup: true,
                                    #[watch]
                                    set_label: &format!("{}<span foreground=\"{}\">{}</span>{}{}",
                                        get_message_kind_prefix(&chat.kind),
                                        get_team_color_string(chat.team.as_ref()),
                                        markup_escape_text(&chat.from),
                                        if chat.from.is_empty() {""} else {": "},
                                        markup_escape_text(&chat.text),
                                    ),
                                },
                                gtk::Button{
                                    set_label: "Copy message",
                                    set_halign: gtk::Align::Center,
                                    connect_clicked => EventDialogMsg::CopyMessage,
                                }
                            }
                            MatchEventType::Connection(conn) => gtk::Label{
                                set_selectable: true,
                                #[watch]
                                set_label: &format!("{} ({}) {}", conn.name, conn.steamid, match &conn.value {
                                    ConnectionEventType::Join => "joined the game".to_owned(),
                                    ConnectionEventType::Leave(reason) => format!("left the game\nReason: {}", reason),
                                })
                            }
                            MatchEventType::TeamSwitch(id, team) => gtk::Label{
                                set_selectable: true,
                                #[watch]
                                set_label: &format!("{} switched to team {}",
                                    model.inspection.as_ref()
                                        .map(|i|&i.users[id])
                                        .and_then(|u|u.name.as_ref())
                                        .map_or("unknown", |n|n),
                                    team
                                )
                            }
                            MatchEventType::ClassSwitch(id, class) => gtk::Label{
                                set_selectable: true,
                                #[watch]
                                set_label: &format!("{} switched to {}",
                                    model.inspection.as_ref()
                                        .map(|i|&i.users[id])
                                        .and_then(|u|u.name.as_ref())
                                        .map_or("unknown", |n|n),
                                    class
                                )
                            }
                            MatchEventType::VoteStarted(vote) => gtk::Grid{
                                set_row_spacing: 10,
                                set_column_spacing: 10,
                                attach[0,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Team:",
                                },
                                attach[1,0,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &vote.team.to_string(),
                                },
                                attach[0,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Time:",
                                },
                                attach[1,1,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &util::ticks_to_timestamp((vote.end_tick - vote.start_tick).into(), model.tps),
                                },
                                attach[0,2,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Initiator:",
                                },
                                attach[1,2,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: vote.initiator.as_ref().map_or("unknown",|v|v),
                                },
                                attach[0,3,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Issue:",
                                },
                                attach[1,3,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: vote.issue.as_ref().map_or("unknown", |v|v),
                                },
                                attach[0,4,1,1] = &gtk::Label{
                                    set_halign: gtk::Align::Start,
                                    set_label: "Votes:",
                                },
                                #[watch]
                                remove_row: 5,
                                #[watch]
                                attach: (&vote_table(vote), 0, 5, 2, 1),
                            }
                        },
                    },
                    None => gtk::Label{
                        set_label: "Nothing",
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
            selected_event: None,
            tps: 0.0,
            parent: init,
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
        match message {
            EventDialogMsg::Update(Some((state, ind, tps))) => {
                self.inspection = Some(state);
                self.selected_event = Some(ind);
                self.tps = tps;
                root.present(Some(&self.parent));
            }
            EventDialogMsg::Update(None) => {
                root.force_close();
            }
            EventDialogMsg::CopyMessage => {
                if let Some(MatchEventType::Chat(chat)) = self
                    .inspection
                    .as_ref()
                    .zip(self.selected_event)
                    .and_then(|(i, s)| i.events.get(s))
                    .map(|ev| &ev.value)
                {
                    let msg = format!(
                        "{}{}{}{}",
                        get_message_kind_prefix(&chat.kind),
                        chat.from,
                        if chat.from.is_empty() { "" } else { ": " },
                        markup_escape_text(&chat.text),
                    );
                    let disp = gtk::gdk::Display::default().unwrap();
                    let clip = disp.clipboard();
                    clip.set_text(&msg);
                }
            }
            EventDialogMsg::Goto => {
                if let Some(ev) = self
                    .inspection
                    .as_ref()
                    .zip(self.selected_event)
                    .and_then(|(i, s)| i.events.get(s))
                {
                    let _ = sender.output(ev.tick.into());
                }
            }
        }
        self.update_view(widgets, sender);
    }
}

#[derive(Debug, Clone)]
struct EventRowModel {
    event: MatchEvent,
    tps: f32,

    icon: String,
    title: String,
    subtitle: String,

    matches_filter: bool,
}

#[derive(Debug, Clone)]
enum EventRowMsg {
    Filter(EventListFilter),
}

#[relm4::factory]
impl FactoryComponent for EventRowModel {
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();
    type Input = EventRowMsg;
    type Output = DynamicIndex;
    type Init = (MatchEvent, f32, Arc<MatchState>);

    view! {
        #[root]
        adw::ActionRow{
            set_activatable: true,
            #[watch]
            set_visible: self.matches_filter,
            add_prefix = &gtk::Image{
                set_icon_name: Some(&self.icon),
            },
            add_suffix = &gtk::Label{
                set_label: &format!("{} ({})",
                    crate::util::ticks_to_timestamp(self.event.tick.into(), self.tps), self.event.tick
                )
            },
            set_title: &self.title,
            set_subtitle: &self.subtitle,
            connect_activated[sender, index] => move |_|{
                sender.output(index.clone()).unwrap();
            }
        }
    }

    fn init_model(
        (ev, tps, state): Self::Init,
        _index: &Self::Index,
        _sender: FactorySender<Self>,
    ) -> Self {
        let icon: &str;
        let title: String;
        let subtitle: String;
        match &ev.value {
            MatchEventType::Kill(death) => {
                icon = relm4_icons::icon_names::VIOLENCE3;
                if let Some(killer) = &death.killer {
                    let killer = &state.users[killer];
                    let victim = &state.users[&death.victim];
                    title = markup_escape_text(&format!(
                        "{} killed {} with {}{}",
                        killer.name.as_ref().map_or("unknown", |v| v),
                        victim.name.as_ref().map_or("unknown", |v| v),
                        death.weapon,
                        match death.crit_type {
                            crate::analyser::CritType::None => "".to_string(),
                            crate::analyser::CritType::Mini => " (mini-crit)".to_string(),
                            crate::analyser::CritType::Full => " (crit)".to_string(),
                            crate::analyser::CritType::Unknown(t) =>
                                format!(" (unknown crit type: {t})"),
                        }
                    ))
                    .into();
                } else {
                    let victim = &state.users[&death.victim];
                    title = markup_escape_text(&format!(
                        "{} was killed with {}{}",
                        victim.name.as_ref().map_or("unknown", |v| v),
                        death.weapon,
                        match death.crit_type {
                            crate::analyser::CritType::None => "".to_string(),
                            crate::analyser::CritType::Mini => " (mini-crit)".to_string(),
                            crate::analyser::CritType::Full => " (crit)".to_string(),
                            crate::analyser::CritType::Unknown(t) =>
                                format!(" (unknown crit type: {t})"),
                        }
                    ))
                    .into();
                }
                subtitle = "".into();
            }
            MatchEventType::RoundEnd(round) => {
                icon = relm4_icons::icon_names::FLAG_FILLED;
                title = format!("Round won by {}", round.winner).into();
                subtitle = "".into();
            }
            MatchEventType::Chat(chat) => {
                let kind = get_message_kind_prefix(&chat.kind);

                let color = get_team_color_string(chat.team.as_ref());

                icon = relm4_icons::icon_names::CHAT_BUBBLES_TEXT;
                title = markup_escape_text(&chat.text).into();
                subtitle = format!(
                    "{}<span foreground=\"{color}\">{}</span>",
                    kind,
                    markup_escape_text(&chat.from).as_str()
                )
                .into();
            }
            MatchEventType::Connection(connection_event) => {
                icon = relm4_icons::icon_names::NETWORK_SERVER;
                title = match &connection_event.value {
                    ConnectionEventType::Join => format!(
                        "{} joined the game",
                        markup_escape_text(&connection_event.name)
                    ),
                    ConnectionEventType::Leave(reason) => format!(
                        "{} left the game ({reason})",
                        markup_escape_text(&connection_event.name)
                    ),
                };
                subtitle = connection_event.steamid.clone();
            }
            MatchEventType::VoteStarted(vote) => {
                icon = relm4_icons::icon_names::CHECK_ROUND_OUTLINE;
                title = format!(
                    "{} started a vote: {}",
                    markup_escape_text(
                        &vote
                            .initiator
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string())
                    ),
                    markup_escape_text(
                        &vote
                            .issue
                            .clone()
                            .unwrap_or_else(|| "Unknown vote issue".to_string())
                    )
                );
                subtitle = format!(
                    "{} | {}",
                    match &vote.team {
                        VoteTeam::Unknown => "Unknown".to_string(),
                        VoteTeam::One(team) => team.to_string(),
                        VoteTeam::Both => "Both Teams".to_string(),
                    },
                    vote.options
                        .iter()
                        .enumerate()
                        .map(|(i, o)| format!(
                            "{}: {}",
                            o,
                            vote.votes.iter().filter(|v| v.2 == i).count()
                        ))
                        .join(", ")
                )
            }
            MatchEventType::TeamSwitch(uid, team) => {
                let user = &state.users[uid];

                icon = relm4_icons::icon_names::HORIZONTAL_ARROWS;
                title =
                    markup_escape_text(&user.name.clone().unwrap_or_else(|| "unknown".to_string()))
                        .into();
                subtitle = format!(
                    "<span foreground=\"{}\">{}</span>",
                    get_team_color_string(Some(&team)),
                    team.to_string()
                );
            }
            MatchEventType::ClassSwitch(uid, class) => {
                let user = &state.users[uid];

                icon = relm4_icons::icon_names::DISCOVER;
                title =
                    markup_escape_text(&user.name.clone().unwrap_or_else(|| "unknown".to_string()))
                        .into();
                subtitle = class.to_string();
            }
        }
        Self {
            icon: icon.to_string(),
            title,
            subtitle,

            event: ev,
            matches_filter: true,
            tps: tps,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            EventRowMsg::Filter(filter) => {
                self.matches_filter = (filter.show_chat
                    && matches!(self.event.value, MatchEventType::Chat(_)))
                    || (filter.show_class
                        && matches!(self.event.value, MatchEventType::ClassSwitch(_, _)))
                    || (filter.show_connections
                        && matches!(self.event.value, MatchEventType::Connection(_)))
                    || (filter.show_deaths && matches!(self.event.value, MatchEventType::Kill(_)))
                    || (filter.show_rounds
                        && matches!(self.event.value, MatchEventType::RoundEnd(_)))
                    || (filter.show_team
                        && matches!(self.event.value, MatchEventType::TeamSwitch(_, _)))
                    || (filter.show_votes
                        && matches!(self.event.value, MatchEventType::VoteStarted(_)));
            }
        }
    }
}
