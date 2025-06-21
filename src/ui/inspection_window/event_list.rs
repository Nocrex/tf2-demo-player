use std::sync::Arc;

use crate::analyser::{
    ConnectionEventType, MatchEvent, MatchEventType, MatchState, StableUserId, VoteTeam,
};
use adw::prelude::*;
use itertools::Itertools;
use relm4::{gtk::glib::markup_escape_text, prelude::*};
use tf_demo_parser::demo::{message::usermessage::ChatMessageKind, parser::analyser::Team};

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
        self.show_deaths = true;
        self.show_rounds = true;
        self.show_connections = true;
        self.show_votes = true;
        self.show_team = true;
        self.show_class = true;
    }
}

#[derive(Debug)]
pub enum EventViewMsg {
    Filter(EventListFilterChange),
    Show(Option<Arc<MatchState>>, f32),
}
pub struct EventViewModel {
    inspection: Option<Arc<MatchState>>,
    tps: f32,

    list_model: FactoryVecDeque<EventRowModel>,

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
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            list_model: FactoryVecDeque::builder().launch_default().detach(),
            inspection: None,
            tps: 0.0,
            filter: EventListFilter {
                show_chat: true,
                show_deaths: true,
                show_rounds: true,
                show_connections: true,
                show_votes: true,
                show_team: true,
                show_class: true,
            },
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
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
            }
        }
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
    type Output = ();
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
                let kind = match chat.kind {
                    ChatMessageKind::ChatAll => "",
                    ChatMessageKind::ChatTeam => "(Team) ",
                    ChatMessageKind::ChatAllDead => "*DEAD* ",
                    ChatMessageKind::ChatTeamDead => "(Team) *DEAD* ",
                    ChatMessageKind::ChatAllSpec => "*SPEC* ",
                    ChatMessageKind::NameChange => "[Name Change] ",
                    ChatMessageKind::Empty => "",
                }
                .to_string();

                let dark = adw::StyleManager::default().is_dark();

                let color = match &chat.team {
                    Some(t) => match t {
                        Team::Spectator | Team::Other => "848484",
                        Team::Red => "e04a4a",
                        Team::Blue => {
                            if dark {
                                "6aaef7"
                            } else {
                                "3449d1"
                            }
                        }
                    },
                    None => "848484",
                };

                icon = relm4_icons::icon_names::CHAT_BUBBLES_TEXT;
                title = markup_escape_text(&chat.text).into();
                subtitle = format!(
                    "{}<span foreground=\"#{color}\">{}</span>",
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
                    vote.initator
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    vote.issue
                        .clone()
                        .unwrap_or_else(|| "Unknown vote issue".to_string())
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
                title = team.to_string();
                subtitle = user.name.clone().unwrap_or_else(|| "unknown".to_string());
            }
            MatchEventType::ClassSwitch(uid, class) => {
                let user = &state.users[uid];

                icon = relm4_icons::icon_names::DISCOVER;
                title = class.to_string();
                subtitle = user.name.clone().unwrap_or_else(|| "unknown".to_string());
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

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
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
