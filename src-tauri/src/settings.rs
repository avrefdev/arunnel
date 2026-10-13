use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Bilgisayar açılınca servis ve tepsi uygulaması başlar.
    pub start_with_windows: bool,
    /// Discord.exe algılanınca otomatik bağlanır.
    pub auto_connect: bool,
    /// Pencere kapatılınca tepsiye küçültülür.
    pub close_to_tray: bool,
    /// Son çalışan strateji; bir sonraki bağlantıda önce bu denenir.
    pub last_profile: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self { start_with_windows: true, auto_connect: false, close_to_tray: true, last_profile: 0 }
    }
}

fn path() -> std::path::PathBuf {
    crate::util::data_dir().join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(path(), json);
    }
}
