//! Runtime configuration. Source of truth for API URLs, mirroring `src/app/env.ts`.
//!
//! Values come from the process environment, after loading `.env` from the
//! working directory. The old `VITE_PINTO_*` names from the Vue build are still
//! accepted so an existing `.env` keeps working.

use std::path::PathBuf;

pub const APP_NAME: &str = "Pinto Chat Desktop";
pub const APP_IDENTIFIER: &str = "com.pinto.chat-desktop";
pub const MAX_MESSAGE_LENGTH: usize = 1000;
pub const TYPING_DEBOUNCE_MS: u64 = 800;
pub const TYPING_AUTO_CLEAR_MS: u64 = 3000;
pub const SEARCH_DEBOUNCE_MS: u64 = 300;
pub const CHAT_PAGE_SIZE: usize = 20;
pub const MESSAGE_PAGE_SIZE: usize = 50;
pub const WS_RECONNECT_DELAYS_MS: [u64; 5] = [1000, 2000, 5000, 10000, 30000];
pub const QR_RECONNECT_DELAY_MS: u64 = 3000;
pub const QR_MAX_LIFETIME_SECS: u64 = 60;

/// API used when neither the environment nor `.env` names one. Release builds
/// in CI set `PINTO_BUILD_API_BASE_URL` at compile time to point at production;
/// local builds fall back to the dev API.
const DEFAULT_API: &str = match option_env!("PINTO_BUILD_API_BASE_URL") {
    Some(url) => url,
    None => "https://api-dev.pinto-app.com",
};

#[derive(Clone, Debug)]
pub struct Env {
    pub api_base_url: String,
    pub ws_base_url: String,
    pub qr_auth_ws_url: String,
}

impl Env {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let api_base_url = read("PINTO_API_BASE_URL")
            .unwrap_or_else(|| DEFAULT_API.to_string())
            .trim_end_matches('/')
            .to_string();
        let ws_base_url = read("PINTO_WS_URL")
            .unwrap_or_else(|| http_to_ws(&api_base_url))
            .trim_end_matches('/')
            .to_string();
        let qr_auth_ws_url = read("PINTO_QR_AUTH_WS_URL")
            .unwrap_or_else(|| format!("{}/ws/auth/qr", http_to_ws(&api_base_url)));

        Self {
            api_base_url,
            ws_base_url,
            qr_auth_ws_url,
        }
    }

    pub fn chat_ws_url(&self, token: &str) -> String {
        format!(
            "{}/v1/ws/chat?token={}",
            self.ws_base_url,
            urlencoding::encode(token)
        )
    }
}

fn read(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| std::env::var(format!("VITE_{name}")).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn http_to_ws(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https") {
        format!("wss{rest}")
    } else if let Some(rest) = url.strip_prefix("http") {
        format!("ws{rest}")
    } else {
        url.to_string()
    }
}

/// Per-user data directory, shared with the Tauri build's identifier.
pub fn data_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_IDENTIFIER);
    let _ = std::fs::create_dir_all(&dir);
    dir
}
