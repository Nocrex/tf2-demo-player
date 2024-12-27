use rcon::{AsyncStdStream, Connection, Error};

use crate::demo_manager::Demo;

#[derive(Debug)]
pub enum Command<'a> {
    PlayDemo(&'a Demo),
    SkipToTick(u32, bool),
    SkipRelative(u32, bool),
    SetEndTick(u32),
    DemoDebug(bool),
    PausePlayback(),
    ResumePlayback(),
    TogglePlayback(),
    StopPlayback(),
    SetPlaybackSpeed(f32),
    /* StartRecording(&'a str, Codec), */
}

impl Command<'_> {
    pub fn get_command(&self) -> String {
        match self {
            Command::PlayDemo(d) => format!("disconnect; playdemo \"{}\"", d.get_path()),
            Command::SkipToTick(t, p) => format!("demo_gototick {} 0 {}", t, *p as u8),
            Command::SkipRelative(t, p) => format!("demo_gototick {} 1 {}", t, *p as u8),
            Command::SetEndTick(t) => format!("demo_setendtick {}", t),
            Command::DemoDebug(b) => format!("demo_debug {}", *b as u8),
            Command::PausePlayback() => "demo_pause".to_owned(),
            Command::ResumePlayback() => "demo_resume".to_owned(),
            Command::SetPlaybackSpeed(s) => format!("demo_timescale {:.2}", s),
            Command::TogglePlayback() => "demo_togglepause".to_owned(),
            Command::StopPlayback() => "stopdemo".to_owned(),
            /* Command::StartRecording(name, codec) => format!("startmovie \"{}\" {}", name, codec.params()), */
        }
    }
}

pub struct RconManager {
    conn: Option<Connection<AsyncStdStream>>,
    password: String,
}

impl RconManager {
    pub fn new(password: String) -> Self {
        RconManager {
            conn: None,
            password,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        let mut err: Result<(), Error> = Ok(());
        match <Connection<AsyncStdStream>>::builder()
            .connect("localhost:27015", &self.password)
            .await
        {
            Ok(c) => self.conn = Some(c),
            Err(e) => err = Err(e),
        };

        if let Err(e) = &err {
            log::error!("RCon connection failed: {:?}", e);
        } else {
            log::info!("Successfully connected to TF2");
        }

        err
    }

    pub async fn send_command(&mut self, command: Command<'_>) -> Result<String, Error> {
        log::debug!("Sending command: {}", command.get_command());
        if !self.is_connected() {
            if let Err(e) = self.connect().await {
                return Err(e);
            }
        }

        let conn = self.conn.as_mut().unwrap();
        let cmd = command.get_command();
        log::debug!("Sending command: {}", cmd);
        let res = conn.cmd(&cmd).await;
        log::debug!("Response: {:?}", res);
        if let Err(e) = &res {
            match e {
                Error::Io(_) => self.conn = None,
                Error::Auth => self.conn = None,
                Error::CommandTooLong => {}
            }
        }
        res
    }

    pub async fn play_demo(&mut self, demo: &Demo) -> Result<String, Error> {
        self.send_command(Command::PlayDemo(demo)).await
    }

    pub async fn skip_to_tick(&mut self, tick: u32, pause: bool) -> Result<String, Error> {
        self.send_command(Command::SkipToTick(tick, pause)).await
    }

    pub async fn stop_playback(&mut self) -> Result<String, Error> {
        self.send_command(Command::StopPlayback()).await
    }
}
