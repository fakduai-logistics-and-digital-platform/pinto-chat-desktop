//! Small JSON files in the app data directory, replacing the browser's
//! `localStorage` (`token-storage.ts`, `settings.store.ts`).

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::env::data_dir;

fn read<T: DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read_to_string(data_dir().join(name))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write<T: Serialize>(name: &str, value: &T) {
    let path = data_dir().join(name);
    let tmp = path.with_extension("tmp");
    if let Ok(s) = serde_json::to_string_pretty(value)
        && std::fs::write(&tmp, s).is_ok()
    {
        let _ = std::fs::rename(&tmp, &path);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoredSession {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl StoredSession {
    pub fn load() -> Self {
        read("session.json")
    }
    pub fn save(&self) {
        write("session.json", self);
    }
    pub fn clear() {
        let _ = std::fs::remove_file(data_dir().join("session.json"));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    System,
}

/// Chat wall behind the messages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Wallpaper {
    #[default]
    Pattern,
    Plain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub wallpaper: Wallpaper,
    pub theme: ThemeMode,
    pub compact_mode: bool,
    pub notifications_enabled: bool,
    pub show_preview: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wallpaper: Wallpaper::Pattern,
            theme: ThemeMode::Light,
            compact_mode: false,
            notifications_enabled: true,
            show_preview: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        read("settings.json")
    }
    pub fn save(&self) {
        write("settings.json", self);
    }
}

/// Stable per-install device id, sent as `X-Device-ID` so the server binds
/// refresh tokens to this machine and can list it under logged-in devices.
pub fn device_id() -> String {
    let path = data_dir().join("device_id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = std::fs::write(&path, &id);
    id
}
