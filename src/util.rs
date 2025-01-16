pub fn ticks_to_sec(ticks: u32, tickrate: f32) -> f32 {
    return ticks as f32 / tickrate;
}

pub fn ticks_to_timestamp(ticks: u32, tickrate: f32) -> String {
    sec_to_timestamp(ticks_to_sec(ticks, tickrate))
}

pub fn sec_to_timestamp(sec: f32) -> String {
    let secs = (sec % 60.0) as u32;
    let mins = (sec / 60.0).trunc() as u32 % 60;
    let hrs = (sec / 3600.0).trunc() as u32;
    if hrs > 0 {
        format!("{:0>2}:{:0>2}:{:0>2}", hrs, mins, secs)
    } else {
        format!("{:0>2}:{:0>2}", mins, secs)
    }
}

/// Convert a steamid32 (U:0:1234567) to a steamid64 (76561197960265728)
pub fn steamid_32_to_64(steamid32: &str) -> Option<String> {
    let segments: Vec<&str> = steamid32.trim_end_matches("]").split(':').collect();

    let id32: u64 = if let Ok(id32) = segments.get(2)?.parse() {
        id32
    } else {
        return None;
    };

    Some(format!("{}", id32 + 76561197960265728))
}

pub mod steam {

    pub fn tf_folder() -> Option<std::path::PathBuf> {
        let libraries_vdf = steam_folder()?.join("steamapps").join("libraryfolders.vdf");
        let libraries = std::fs::read_to_string(libraries_vdf).ok()?;

        let path_regex = regex_macro::regex!("\"path\"\\s+\"(.+)\"");

        for folder in path_regex.captures_iter(&libraries) {
            let library_folder =
                std::path::PathBuf::from(folder.get(1).unwrap().as_str().replace("\\\\", "\\"));
            if library_folder
                .join("steamapps")
                .join("appmanifest_440.acf")
                .is_file()
            {
                return Some(
                    library_folder
                        .join("steamapps")
                        .join("common")
                        .join("Team Fortress 2"),
                );
            }
        }

        None
    }

    #[cfg(target_family = "unix")]
    fn steam_folder() -> Option<std::path::PathBuf> {
        std::env::var("HOME")
            .map(|home| {
                std::path::Path::new(&home)
                    .join(".local")
                    .join("share")
                    .join("Steam")
            })
            .ok()
    }

    #[cfg(target_family = "windows")]
    fn steam_folder() -> Option<std::path::PathBuf> {
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        let steam = hklm
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .ok()
            .or_else(|| hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam").ok())?;
        steam
            .get_value("InstallPath")
            .map(|p: String| std::path::PathBuf::from(p))
            .ok()
    }
}
