use tokio::net::TcpStream;
use rcon::{Connection, Error};

use crate::demo_manager::Demo;

pub enum Command<'a> {
    PlayDemo(&'a Demo),
}

impl Command<'_> {
    pub fn get_command(&self) -> String {
        match self {
            Self::PlayDemo(d) => format!("playdemo \"{}\"", d.get_path())
        }
    }
}

pub struct RconManager {
    conn: Option<Connection<TcpStream>>,
    password: String,
}

impl RconManager {
    pub fn new(password: String) -> Self {
        RconManager{
            conn: None,
            password
        }
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    async fn connect(&mut self) -> Result<(), Error>{
        let mut err: Result<(), Error> = Ok(());
        match <Connection<TcpStream>>::builder().connect("localhost:27015", &self.password).await {
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
        if !self.is_connected() {
            if let Err(e) = self.connect().await{
                return Err(e);
            }
        }

        let conn = self.conn.as_mut().unwrap();
        let cmd = command.get_command();
        log::debug!("Sending command: {}", cmd);
        conn.cmd(&cmd).await
    }
}