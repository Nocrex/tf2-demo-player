use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;
use tf_demo_parser::demo::gameevent_gen::{PlayerConnectClientEvent, PlayerDisconnectEvent};
use tf_demo_parser::demo::message::usermessage::{SayText2Message, TextMessage};
use tf_demo_parser::demo::message::ServerInfoMessage;
use tf_demo_parser::demo::parser::analyser::Spawn;
use tf_demo_parser::ReadResult;
use tf_demo_parser::{
    demo::{
        data::{DemoTick, ServerTick},
        gameevent_gen::{PlayerDeathEvent, TeamPlayRoundWinEvent},
        gamevent::GameEvent,
        message::{
            packetentities::EntityId,
            usermessage::{ChatMessageKind, HudTextLocation, UserMessage},
            Message,
        },
        packet::stringtable::StringTableEntry,
        parser::{
            analyser::{Class, Team, UserId},
            handler::BorrowMessageHandler,
            MessageHandler,
        },
    },
    MessageType, ParserState, Stream,
};

#[derive(Debug, Clone, Default)]
pub struct UserInfo {
    pub last_class: Option<Class>,
    pub name: Option<String>,
    pub user_id: UserId,
    pub steam_id: Option<String>,
    pub entity_id: Option<EntityId>,
    pub last_team: Option<Team>,

    pub connection_events: Vec<(DemoTick, ConnectionEventType)>,
    pub class_switches: Vec<(DemoTick, Class)>,
}

impl From<&tf_demo_parser::demo::data::UserInfo> for UserInfo {
    fn from(info: &tf_demo_parser::demo::data::UserInfo) -> Self {
        UserInfo {
            name: Some(info.player_info.name.clone()),
            steam_id: Some(info.player_info.steam_id.clone()),
            entity_id: Some(info.entity_id),
            user_id: info.player_info.user_id,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Vote {
    pub start_tick: DemoTick,
    pub end_tick: DemoTick,
    pub team: VoteTeam,
    pub initator: Option<String>,
    pub issue: Option<String>,
    pub options: Vec<String>,
    pub votes: Vec<(DemoTick, String, usize)>,
}

impl Vote {
    pub fn result(&self) -> HashMap<&String, usize> {
        self.votes.iter().map(|c| &self.options[c.2]).counts()
    }
}

#[derive(Debug, Default, Clone)]
pub enum VoteTeam {
    #[default]
    Unknown,
    One(Team),
    Both,
}

impl ToString for VoteTeam {
    fn to_string(&self) -> String {
        match self {
            VoteTeam::Unknown => "Unknown".to_string(),
            VoteTeam::One(t) => t.to_string(),
            VoteTeam::Both => "Both".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Death {
    pub weapon: String,
    pub victim: StableUserId,
    pub assister: Option<StableUserId>,
    pub killer: Option<StableUserId>,
    pub crit_type: CritType,
    pub domination: bool,
    pub revenge: bool,
    pub assist_dom: bool,
    pub assist_revg: bool,
    pub deadringer: bool,
}

#[derive(Debug, Default, Clone)]
pub enum CritType {
    #[default]
    None,
    Mini,
    Full,
    Unknown(u16),
}

impl Death {
    pub fn from_event(event: &PlayerDeathEvent, analyser: &mut Analyser) -> Self {
        const TF_DEATH_DOMINATION: u16 = 0x0001; // killer is dominating victim
        const TF_DEATH_ASSISTER_DOMINATION: u16 = 0x0002; // assister is dominating victim
        const TF_DEATH_REVENGE: u16 = 0x0004; // killer got revenge on victim
        const TF_DEATH_ASSISTER_REVENGE: u16 = 0x0008; // assister got revenge on victim
        const TF_DEATH_FEIGN_DEATH: u16 = 0x0020; // feign death
        let assister = if event.assister < (16 * 1024) {
            Some(analyser.stable_user(
                |u| u.user_id == event.assister,
                || UserInfo {
                    user_id: event.assister.into(),
                    ..Default::default()
                },
            ))
        } else {
            None
        };
        let killer = if event.attacker == 0 {
            None
        } else {
            Some(analyser.stable_user(
                |u| u.user_id == event.attacker,
                || UserInfo {
                    user_id: event.attacker.into(),
                    ..Default::default()
                },
            ))
        };
        Death {
            assister,
            killer: killer,
            weapon: event.weapon.to_string(),
            victim: analyser.stable_user(
                |u| u.user_id == event.user_id,
                || UserInfo {
                    user_id: event.user_id.into(),
                    ..Default::default()
                },
            ),
            crit_type: match event.crit_type {
                0 => CritType::None,
                1 => CritType::Mini,
                2 => CritType::Full,
                a => CritType::Unknown(a),
            },
            domination: (event.death_flags & TF_DEATH_DOMINATION) != 0,
            revenge: (event.death_flags & TF_DEATH_REVENGE) != 0,
            assist_dom: (event.death_flags & TF_DEATH_ASSISTER_DOMINATION) != 0,
            assist_revg: (event.death_flags & TF_DEATH_ASSISTER_REVENGE) != 0,
            deadringer: (event.death_flags & TF_DEATH_FEIGN_DEATH) != 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub winner: Team,
    pub length: f32,
}

impl From<&TeamPlayRoundWinEvent> for Round {
    fn from(event: &TeamPlayRoundWinEvent) -> Self {
        Round {
            winner: Team::new(event.team),
            length: event.round_time,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub maxplayers: u8,
    pub interval_per_tick: f32,
    pub platform: String,
    pub is_stv: bool,
}

impl From<&Box<ServerInfoMessage>> for ServerInfo {
    fn from(value: &std::boxed::Box<ServerInfoMessage>) -> Self {
        Self {
            name: value.server_name.clone(),
            maxplayers: value.max_player_count,
            interval_per_tick: value.interval_per_tick,
            platform: value.platform.clone(),
            is_stv: value.stv,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionEventType {
    Join,
    Leave(String),
}

#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    pub user: StableUserId,
    pub name: String,
    pub steamid: String,
    pub value: ConnectionEventType,
}

impl ConnectionEvent {
    fn from_conn(value: &PlayerConnectClientEvent, user: StableUserId) -> Self {
        Self {
            name: value.name.to_string(),
            steamid: value.network_id.to_string(),
            value: ConnectionEventType::Join,
            user,
        }
    }

    fn from_dc(value: &PlayerDisconnectEvent, user: StableUserId) -> Self {
        Self {
            name: value.name.to_string(),
            steamid: value.network_id.to_string(),
            value: ConnectionEventType::Leave(value.reason.to_string()),
            user,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub kind: ChatMessageKind,
    pub from: String,
    pub text: String,
    pub team: Option<Team>,
}

impl ChatMessage {
    pub fn from_message(message: &SayText2Message, team: Option<Team>) -> Self {
        ChatMessage {
            kind: message.kind,
            from: message
                .from
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            text: message.plain_text(),
            team,
        }
    }

    pub fn from_text(message: &TextMessage) -> Self {
        let mut text = resolve_string(&message.text.to_string()).to_owned();
        for i in 1..4 {
            let pat = format!("%s{i}");
            if text.contains(&pat) {
                text = text.replace(&pat, resolve_string(&message.substitute[i - 1].to_string()));
                continue;
            }
            break;
        }
        ChatMessage {
            kind: ChatMessageKind::Empty,
            from: String::new(),
            text,
            team: None,
        }
    }
}

fn resolve_string(string: &str) -> &str {
    match string {
        "#game_player_was_team_balanced" => "%s1 was moved to the other team for game balance",
        "#game_spawn_as" => "*You will spawn as %s1",
        "#TF_TeamsSwitched" => "Teams have been switched.",
        "#TF_Autobalance_TeamChangeDone_Match" => "You have switched to team %s1 and will receive %s2 experience points at the end of the round for changing teams.",
        "#TF_BlueTeam_Name" => "BLU",
        "#TF_RedTeam_Name" => "RED",
        o => o,
    }
}

#[derive(Debug, Clone)]
pub enum MatchEventType {
    Kill(Death),
    RoundEnd(Round),
    Chat(ChatMessage),
    Connection(ConnectionEvent),
    VoteStarted(Vote),
    TeamSwitch(StableUserId, Team),
    ClassSwitch(StableUserId, Class),
}

#[derive(Debug, Clone)]
pub struct MatchEvent {
    pub tick: DemoTick,
    pub value: MatchEventType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StableUserId(usize);

impl From<usize> for StableUserId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct MatchState {
    pub users: Vec<UserInfo>,
    pub server_info: ServerInfo,
    pub start_tick: ServerTick,
    pub end_tick: DemoTick,

    pub events: Vec<MatchEvent>,
}

#[derive(Default, Debug)]
pub struct Analyser {
    state: MatchState,
    pub votes: BTreeMap<u32, Vote>,
}

impl MessageHandler for Analyser {
    type Output = MatchState;

    fn does_handle(message_type: MessageType) -> bool {
        matches!(
            message_type,
            MessageType::GameEvent
                | MessageType::UserMessage
                | MessageType::ServerInfo
                | MessageType::NetTick
        )
    }

    fn handle_message(&mut self, message: &Message, tick: DemoTick, _parser_state: &ParserState) {
        self.state.end_tick = tick;
        match message {
            Message::NetTick(msg) => {
                if self.state.start_tick == 0 {
                    self.state.start_tick = msg.tick;
                }
            }
            Message::ServerInfo(message) => {
                self.state.server_info = message.into();
            }
            Message::GameEvent(message) => self.handle_event(&message.event, tick),
            Message::UserMessage(message) => self.handle_user_message(message, tick),
            _ => {}
        }
    }

    fn handle_string_entry(
        &mut self,
        table: &str,
        index: usize,
        entry: &StringTableEntry,
        _parser_state: &ParserState,
    ) {
        if table == "userinfo" {
            let _ = self.parse_user_info(
                index,
                entry.text.as_ref().map(|s| s.as_ref()),
                entry.extra_data.as_ref().map(|data| data.data.clone()),
            );
        }
    }

    fn into_output(self, _state: &ParserState) -> Self::Output {
        let mut state = self.state;
        for (_, vote) in self.votes {
            let ind_start = state.events.partition_point(|e| e.tick >= vote.start_tick);
            state.events.insert(
                ind_start,
                MatchEvent {
                    tick: vote.start_tick,
                    value: MatchEventType::VoteStarted(vote),
                },
            );
        }
        state
    }
}

impl BorrowMessageHandler for Analyser {
    fn borrow_output(&self, _state: &ParserState) -> &Self::Output {
        &self.state
    }
}

impl Analyser {
    pub fn new() -> Self {
        Self::default()
    }

    fn stable_user(
        &mut self,
        crit: impl Fn(&UserInfo) -> bool,
        ins: impl FnOnce() -> UserInfo,
    ) -> StableUserId {
        self.state
            .users
            .iter()
            .position(crit)
            .unwrap_or_else(|| {
                let idx = self.state.users.len();
                self.state.users.push(ins());
                idx
            })
            .into()
    }

    fn handle_user_message(&mut self, message: &UserMessage, tick: DemoTick) {
        match message {
            UserMessage::SayText2(text_message) => {
                let team = self
                    .state
                    .users
                    .iter()
                    .find(|u| u.entity_id == Some(text_message.client))
                    .map(|ui| ui.last_team.unwrap_or_default());
                if text_message.kind == ChatMessageKind::NameChange {
                    if let Some(from) = text_message.from.clone() {
                        self.change_name(from.into(), text_message.plain_text());
                    }
                } else {
                    self.state.events.push(MatchEvent {
                        tick: tick,
                        value: MatchEventType::Chat(ChatMessage::from_message(&text_message, team)),
                    });
                }
            }
            UserMessage::Text(text_message) => {
                if text_message.location == HudTextLocation::PrintTalk {
                    self.state.events.push(MatchEvent {
                        tick: tick,
                        value: MatchEventType::Chat(ChatMessage::from_text(&text_message)),
                    });
                }
            }
            _ => {}
        }
    }

    fn change_name(&mut self, from: String, to: String) {
        if let Some(user) = self
            .state
            .users
            .iter_mut()
            .find(|user| user.name.as_ref().map_or(false, |n| *n == from))
        {
            user.name = Some(to);
        }
    }

    fn handle_event(&mut self, event: &GameEvent, tick: DemoTick) {
        const WIN_REASON_TIME_LIMIT: u8 = 6;

        match event {
            GameEvent::PlayerDeath(event) => {
                let value = MatchEventType::Kill(Death::from_event(event, self));
                self.state.events.push(MatchEvent { tick: tick, value });
            }
            GameEvent::TeamPlayRoundWin(event) => {
                if event.win_reason != WIN_REASON_TIME_LIMIT {
                    self.state.events.push(MatchEvent {
                        tick,
                        value: MatchEventType::RoundEnd(event.into()),
                    });
                }
            }
            GameEvent::PlayerSpawn(spawn) => {
                let spawn = Spawn::from_event(spawn, tick);
                let suid = self.stable_user(
                    |u| u.user_id == spawn.user,
                    || UserInfo {
                        user_id: spawn.user,
                        ..Default::default()
                    },
                );
                let player = self.state.users.get_mut(suid.0).unwrap();
                if player.last_class.is_none() {
                    player.last_class = Some(spawn.class);
                    self.state.events.push(MatchEvent {
                        tick,
                        value: MatchEventType::ClassSwitch(suid, spawn.class),
                    });
                    player.class_switches.push((tick, spawn.class));
                }
                if player.last_team.is_none() || player.last_team.unwrap() != spawn.team {
                    player.last_team = Some(spawn.team);
                    self.state.events.push(MatchEvent {
                        tick,
                        value: MatchEventType::TeamSwitch(suid, spawn.team),
                    });
                }
            }
            GameEvent::VoteCast(cast) => {
                self.votes.entry(cast.voteidx).and_modify(|v| {
                    v.end_tick = tick;
                    v.team = match &v.team {
                        VoteTeam::Unknown => VoteTeam::One(Team::new(cast.team)),
                        VoteTeam::One(team) => {
                            if *team == Team::new(cast.team) {
                                VoteTeam::One(*team)
                            } else {
                                VoteTeam::Both
                            }
                        }
                        VoteTeam::Both => VoteTeam::Both,
                    };
                    if let Some(player) = self
                        .state
                        .users
                        .iter()
                        .find(|u| u.entity_id == Some(cast.entity_id.into()))
                        .map(|u| u.name.clone().unwrap_or_default())
                    {
                        v.votes
                            .push((tick, player.clone(), cast.vote_option.into()));
                        if tick == v.start_tick {
                            match v.options[cast.vote_option as usize].as_str() {
                                "Yes" => v.initator = Some(player.clone()),
                                "No" => v.issue = Some(format!("Kick player \"{player}\"?")),
                                _ => (),
                            }
                        }
                    } else {
                        v.votes.push((
                            tick,
                            "-- Unknown Player --".to_owned(),
                            cast.vote_option.into(),
                        ));
                    }
                });
            }
            GameEvent::VoteOptions(options) => {
                let mut opts = vec![
                    options.option_1.to_string(),
                    options.option_2.to_string(),
                    options.option_3.to_string(),
                    options.option_4.to_string(),
                    options.option_5.to_string(),
                ];
                opts.retain(|o| !o.is_empty());
                self.votes.entry(options.voteidx).or_insert_with(|| Vote {
                    start_tick: tick,
                    end_tick: tick,
                    options: opts,
                    ..Default::default()
                });
            }
            GameEvent::PlayerConnectClient(conn) => {
                let mut id = conn.network_id.to_string();
                if id == "BOT" {
                    id.push_str(&format!(" {}", conn.user_id));
                }
                let suid = self.stable_user(
                    |u| u.user_id == Into::<UserId>::into(conn.user_id),
                    || UserInfo {
                        user_id: conn.user_id.into(),
                        steam_id: Some(conn.network_id.to_string()),
                        name: Some(conn.name.to_string()),
                        ..Default::default()
                    },
                );
                let c = ConnectionEvent::from_conn(conn, suid);
                self.state.users[suid.0]
                    .connection_events
                    .push((tick, c.value.clone()));
                self.state.events.push(MatchEvent {
                    tick,
                    value: MatchEventType::Connection(c),
                });
            }
            GameEvent::PlayerDisconnect(discon) => {
                let mut id = discon.network_id.to_string();
                if id == "BOT" {
                    id.push_str(&format!(" {}", discon.user_id));
                }
                let suid = self.stable_user(
                    |u| u.user_id == Into::<UserId>::into(discon.user_id),
                    || UserInfo {
                        user_id: discon.user_id.into(),
                        steam_id: Some(discon.network_id.to_string()),
                        name: Some(discon.name.to_string()),
                        ..Default::default()
                    },
                );
                let c = ConnectionEvent::from_dc(discon, suid);
                self.state.users[suid.0]
                    .connection_events
                    .push((tick, c.value.clone()));
                self.state.events.push(MatchEvent {
                    tick,
                    value: MatchEventType::Connection(c),
                });
            }
            GameEvent::PlayerChangeClass(c) => {
                let class = Class::new(c.class);
                let suid = self.stable_user(
                    |u| u.user_id == c.user_id,
                    || UserInfo {
                        user_id: c.user_id.into(),
                        ..Default::default()
                    },
                );
                let user = self.state.users.get_mut(suid.0).unwrap();
                user.last_class = Some(class);
                user.class_switches.push((tick, class));
            }
            _ => {}
        }
    }

    fn parse_user_info(
        &mut self,
        index: usize,
        text: Option<&str>,
        data: Option<Stream>,
    ) -> ReadResult<()> {
        if let Some(user_info) =
            tf_demo_parser::demo::data::UserInfo::parse_from_string_table(index as u16, text, data)?
        {
            if user_info.player_info.steam_id == "BOT" {
                let suid = self.stable_user(
                    |u| u.user_id == user_info.player_info.user_id,
                    || (&user_info).into(),
                );
                self.state.users.get_mut(suid.0).unwrap().entity_id = Some(user_info.entity_id);
            } else {
                let suid = self.stable_user(
                    |u| {
                        u.steam_id == Some(user_info.player_info.steam_id.clone())
                            || u.name == Some(user_info.player_info.name.clone())
                    },
                    || (&user_info).into(),
                );
                let player = self.state.users.get_mut(suid.0).unwrap();
                player.entity_id = Some(user_info.entity_id);
                player.user_id = user_info.player_info.user_id;
            }
        }

        Ok(())
    }
}
