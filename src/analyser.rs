use std::collections::BTreeMap;

use tf_demo_parser::demo::gameevent_gen::{PlayerConnectClientEvent, PlayerDisconnectEvent};
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
            analyser::{ChatMessage, Class, Team, UserId},
            handler::BorrowMessageHandler,
            MessageHandler,
        },
    },
    MessageType, ParserState, Stream,
};

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub classes: Vec<(DemoTick, Class)>,
    pub name: String,
    pub user_id: UserId,
    pub steam_id: String,
    pub entity_id: EntityId,
    pub team: Vec<(DemoTick, Team)>,
}

impl UserInfo {
    pub fn last_team(&self) -> Team {
        self.team.last().map_or(Team::Other, |t| t.1)
    }
}

impl From<tf_demo_parser::demo::data::UserInfo> for UserInfo {
    fn from(info: tf_demo_parser::demo::data::UserInfo) -> Self {
        UserInfo {
            name: info.player_info.name,
            user_id: info.player_info.user_id,
            steam_id: info.player_info.steam_id,
            entity_id: info.entity_id,
            classes: Default::default(),
            team: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Vote {
    pub start_tick: DemoTick,
    pub end_tick: DemoTick,
    pub team: VoteTeam,
    pub initator: Option<String>,
    pub issue: Option<String>,
    pub options: Vec<String>,
    pub votes: Vec<(DemoTick, String, usize)>,
}

#[derive(Debug, Default)]
pub enum VoteTeam {
    #[default]
    Unknown,
    One(Team),
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Death {
    pub weapon: String,
    pub victim: UserId,
    pub assister: Option<UserId>,
    pub killer: UserId,
    pub tick: DemoTick,
    pub crit_type: u16,
}

impl Death {
    pub fn from_event(event: &PlayerDeathEvent, tick: DemoTick) -> Self {
        let assister = if event.assister < (16 * 1024) {
            Some(UserId::from(event.assister))
        } else {
            None
        };
        Death {
            assister,
            tick,
            killer: UserId::from(event.attacker),
            weapon: event.weapon.to_string(),
            victim: UserId::from(event.user_id),
            crit_type: event.crit_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub winner: Team,
    pub length: f32,
    pub end_tick: DemoTick,
}

impl Round {
    pub fn from_event(event: &TeamPlayRoundWinEvent, end_tick: DemoTick) -> Self {
        Round {
            winner: Team::new(event.team),
            length: event.round_time,
            end_tick,
        }
    }
}

#[derive(Default, Debug)]
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

#[derive(Debug, Default)]
pub struct PlayerConnectionDetails {
    pub name: String,
    pub joins: Vec<DemoTick>,
    pub leaves: Vec<(DemoTick, String)>,
}

impl From<&PlayerConnectClientEvent> for PlayerConnectionDetails {
    fn from(value: &PlayerConnectClientEvent) -> Self {
        Self {
            name: value.name.to_string(),
            ..Default::default()
        }
    }
}

impl From<&PlayerDisconnectEvent> for PlayerConnectionDetails {
    fn from(value: &PlayerDisconnectEvent) -> Self {
        Self {
            name: value.name.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Default, Debug)]
pub struct Analyser {
    state: MatchState,
}

#[derive(Debug, Default)]
pub struct MatchState {
    pub chat: Vec<ChatMessage>,
    pub users: BTreeMap<UserId, UserInfo>,
    pub votes: BTreeMap<u32, Vote>,
    pub deaths: Vec<Death>,
    pub rounds: Vec<Round>,
    pub start_tick: ServerTick,
    pub server_info: ServerInfo,
    pub connections: BTreeMap<String, PlayerConnectionDetails>,
    pub end_tick: DemoTick,
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
        self.state
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

    fn handle_user_message(&mut self, message: &UserMessage, tick: DemoTick) {
        match message {
            UserMessage::SayText2(text_message) => {
                if text_message.kind == ChatMessageKind::NameChange {
                    if let Some(from) = text_message.from.clone() {
                        self.change_name(from.into(), text_message.plain_text());
                    }
                } else {
                    self.state
                        .chat
                        .push(ChatMessage::from_message(text_message, tick));
                }
            }
            UserMessage::Text(text_message) => {
                if text_message.location == HudTextLocation::PrintTalk {
                    self.state
                        .chat
                        .push(ChatMessage::from_text(text_message, tick));
                }
            }
            _ => {}
        }
    }

    fn change_name(&mut self, from: String, to: String) {
        if let Some(user) = self.state.users.values_mut().find(|user| user.name == from) {
            user.name = to;
        }
    }

    fn handle_event(&mut self, event: &GameEvent, tick: DemoTick) {
        const WIN_REASON_TIME_LIMIT: u8 = 6;

        match event {
            GameEvent::PlayerDeath(event) => self.state.deaths.push(Death::from_event(event, tick)),
            GameEvent::TeamPlayRoundWin(event) => {
                if event.win_reason != WIN_REASON_TIME_LIMIT {
                    self.state.rounds.push(Round::from_event(event, tick))
                }
            }
            GameEvent::PlayerSpawn(spawn) => {
                let spawn = Spawn::from_event(spawn, tick);
                if let Some(player) = self.state.users.get_mut(&spawn.user) {
                    if player.classes.is_empty() || player.classes.last().unwrap().1 != spawn.class
                    {
                        player.classes.push((tick, spawn.class));
                    }
                    if player.team.is_empty() || player.team.last().unwrap().1 != spawn.team {
                        player.team.push((tick, spawn.team));
                    }
                }
            }
            GameEvent::VoteStarted(vote) => {
                dbg!(vote);
            }
            GameEvent::VoteCast(cast) => {
                self.state.votes.entry(cast.voteidx).and_modify(|v| {
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
                        .values()
                        .find(|u| u.entity_id == cast.entity_id)
                        .map(|u| u.name.clone())
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
                    }
                });
            }
            GameEvent::VoteFailed(fail) => {
                dbg!(fail);
            }
            GameEvent::VotePassed(pass) => {
                dbg!(pass);
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
                self.state
                    .votes
                    .entry(options.voteidx)
                    .or_insert_with(|| Vote {
                        start_tick: tick,
                        end_tick: tick,
                        options: opts,
                        ..Default::default()
                    });
            }
            GameEvent::VoteChanged(change) => {
                dbg!(change);
            }
            GameEvent::VoteEnded(end) => {
                dbg!(end);
            }
            GameEvent::PartyChat(chat) => {
                //dbg!(chat);
            }
            GameEvent::PlayerConnectClient(conn) => {
                let mut id = conn.network_id.to_string();
                if id == "BOT" {
                    id.push_str(&format!(" {}", conn.user_id));
                }
                self.state
                    .connections
                    .entry(id)
                    .or_insert_with(|| conn.into())
                    .joins
                    .push(tick);
            }
            GameEvent::PlayerDisconnect(discon) => {
                let mut id = discon.network_id.to_string();
                if id == "BOT" {
                    id.push_str(&format!(" {}", discon.user_id));
                }
                self.state
                    .connections
                    .entry(id)
                    .or_insert_with(|| discon.into())
                    .leaves
                    .push((tick, discon.reason.to_string()));
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
                self.state
                    .users
                    .entry(user_info.player_info.user_id)
                    .and_modify(|u| u.entity_id = user_info.entity_id)
                    .or_insert_with(|| user_info.into());
            } else {
                if let Some(uid) = self
                    .state
                    .users
                    .iter()
                    .find(|e| e.1.steam_id == user_info.player_info.steam_id)
                    .map(|e| e.0.clone())
                {
                    if user_info.player_info.user_id == uid {
                        self.state.users.get_mut(&uid).unwrap().entity_id = user_info.entity_id;
                    } else {
                        let mut uif = self.state.users.remove(&uid).unwrap();
                        uif.entity_id = user_info.entity_id;
                        self.state.users.insert(user_info.player_info.user_id, uif);
                    }
                } else {
                    self.state
                        .users
                        .insert(user_info.player_info.user_id, user_info.into());
                }
            }
        }

        Ok(())
    }
}
