#[derive(Default)]
pub struct Settings {
    pub demo_folder_path: String,
    pub rcon_pw: String,
}

impl Settings {
    pub fn new() -> Self {
        Settings::default()
    }

    pub fn load() -> Self {
        let mut s = Self::new();
        s.demo_folder_path = "/home/nocrex/.steam/steam/steamapps/common/Team Fortress 2/tf/demos".into();
        s.rcon_pw = "tf2bk".into();
        s
    }
}