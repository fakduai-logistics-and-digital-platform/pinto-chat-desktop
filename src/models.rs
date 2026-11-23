//! Domain types and normalizers, ported from `*.types.ts`,
//! `chats.store.ts::normalizeChat` and `message-normalizer.ts`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Parses the timestamps the API and SQLite hand back. Unparseable values
/// sort as the epoch rather than panicking.
pub fn parse_time(s: &str) -> DateTime<Utc> {
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return t.with_timezone(&Utc);
    }
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(t) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return t.and_utc();
        }
    }
    DateTime::<Utc>::UNIX_EPOCH
}

// ---- JSON helpers -------------------------------------------------------

pub(crate) fn s(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn path(v: &Value, keys: &[&str]) -> Option<String> {
    let mut cur = v;
    for k in &keys[..keys.len() - 1] {
        cur = cur.get(*k)?;
    }
    s(cur, keys[keys.len() - 1])
}

fn b(v: &Value, key: &str) -> bool {
    match v.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn i(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

// ---- Users --------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub profile_image: Option<String>,
}

impl UserProfile {
    pub fn from_value(v: &Value) -> Option<Self> {
        let user_id = s(v, "user_id").or_else(|| s(v, "id"))?;
        let username = s(v, "username").unwrap_or_default();
        Some(Self {
            name: s(v, "name").unwrap_or_else(|| username.clone()),
            username,
            user_id,
            profile_image: s(v, "profile_image"),
        })
    }

    pub fn display_name(&self) -> &str {
        if !self.name.is_empty() {
            &self.name
        } else if !self.username.is_empty() {
            &self.username
        } else {
            "ผู้ใช้"
        }
    }
}

// ---- Chats --------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatType {
    Direct,
    Group,
}

impl ChatType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Group => "group",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "group" {
            Self::Group
        } else {
            Self::Direct
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chat {
    pub chat_id: String,
    pub chat_type: ChatType,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_message_id: Option<String>,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: i64,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub created_at: Option<String>,
    pub updated_at: String,
    pub raw_json: String,
}

impl Chat {
    /// `normalizeChat`: fills name, avatar and last-message fields from the
    /// nested shapes the API may return.
    pub fn from_value(v: &Value) -> Option<Self> {
        let chat_id = s(v, "chat_id").or_else(|| s(v, "id"))?;
        let participant = v
            .get("participants")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(Value::Null);

        let avatar_url = s(v, "avatar_url")
            .or_else(|| path(v, &["use_avatar", "avatar_url"]))
            .or_else(|| s(v, "avatar"))
            .or_else(|| path(&participant, &["use_avatar", "avatar_url"]))
            .or_else(|| s(&participant, "avatar"));
        let name = s(v, "name")
            .or_else(|| s(&participant, "name"))
            .or_else(|| s(&participant, "username"));
        let updated_at = s(v, "updated_at").unwrap_or_else(now_iso);

        Some(Self {
            chat_type: ChatType::parse(&s(v, "chat_type").unwrap_or_default()),
            name,
            avatar_url,
            last_message_id: s(v, "last_message_id")
                .or_else(|| path(v, &["last_message", "message_id"])),
            last_message_preview: s(v, "last_message_preview")
                .or_else(|| path(v, &["last_message", "content"])),
            last_message_at: s(v, "last_message_at")
                .or_else(|| path(v, &["last_message", "created_at"]))
                .or_else(|| Some(updated_at.clone())),
            unread_count: i(v, "unread_count"),
            is_pinned: b(v, "is_pinned"),
            is_muted: b(v, "is_muted"),
            is_archived: b(v, "is_archived"),
            is_deleted: b(v, "is_deleted"),
            created_at: s(v, "created_at"),
            updated_at,
            raw_json: v.to_string(),
            chat_id,
        })
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("ไม่ทราบชื่อ")
    }

    pub fn sort_time(&self) -> DateTime<Utc> {
        parse_time(self.last_message_at.as_deref().unwrap_or(&self.updated_at))
    }
}

// ---- Messages -----------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Image,
    File,
    System,
}

impl MessageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::File => "file",
            Self::System => "system",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "image" => Self::Image,
            "file" => Self::File,
            "system" => Self::System,
            _ => Self::Text,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageStatus {
    Queued,
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Delivered => "delivered",
            Self::Read => "read",
            Self::Failed => "failed",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "queued" => Self::Queued,
            "sending" => Self::Sending,
            "delivered" => Self::Delivered,
            "read" => Self::Read,
            "failed" => Self::Failed,
            _ => Self::Sent,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub message_id: String,
    pub chat_id: String,
    pub sender_id: Option<String>,
    pub content: Option<String>,
    pub message_type: MessageType,
    pub reply_to: Option<String>,
    pub media_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub status: MessageStatus,
    pub is_read: bool,
    /// Our idempotency key; the API calls it `client_message_id`.
    pub client_temp_id: Option<String>,
    /// Server ordering within a chat; gaps are filled with `/messages/repair`.
    pub sequence_id: Option<i64>,
    pub is_edited: bool,
    pub raw_json: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default)]
pub struct Sender {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

impl Message {
    /// `normalizeMessage` from the TS client.
    /// Accepts the `ChatMessage` shape from the API spec (nested `sender` and
    /// `media`, `client_message_id`, `sequence_id`, `read_by`) and the older flat one.
    pub fn from_value(raw: &Value) -> Self {
        let now = now_iso();
        let media = raw.get("media").cloned().unwrap_or(Value::Null);
        let sender_id = s(raw, "sender_id").or_else(|| path(raw, &["sender", "user_id"]));
        // Read once anyone other than the sender appears in `read_by`.
        let read_by_other = raw
            .get("read_by")
            .and_then(Value::as_array)
            .is_some_and(|list| list.iter().any(|r| s(r, "user_id") != sender_id));
        Self {
            message_id: s(raw, "message_id")
                .or_else(|| s(raw, "id"))
                .unwrap_or_else(new_id),
            chat_id: s(raw, "chat_id").unwrap_or_default(),
            sender_id: sender_id.clone(),
            content: s(raw, "content"),
            message_type: MessageType::parse(&s(raw, "message_type").unwrap_or_default()),
            reply_to: s(raw, "reply_to"),
            media_url: s(&media, "url").or_else(|| s(raw, "media_url")),
            file_name: s(&media, "name").or_else(|| s(raw, "file_name")),
            file_size: media
                .get("size")
                .or_else(|| raw.get("file_size"))
                .and_then(Value::as_i64),
            mime_type: s(&media, "mime_type").or_else(|| s(raw, "mime_type")),
            status: if read_by_other {
                MessageStatus::Read
            } else {
                MessageStatus::Sent
            },
            is_read: b(raw, "is_read") || read_by_other,
            client_temp_id: s(raw, "client_message_id").or_else(|| s(raw, "client_temp_id")),
            sequence_id: raw.get("sequence_id").and_then(Value::as_i64),
            is_edited: b(raw, "is_edited"),
            raw_json: Some(raw.to_string()),
            sent_at: s(raw, "sent_at"),
            created_at: s(raw, "created_at").unwrap_or_else(|| now.clone()),
            updated_at: now,
        }
    }

    /// `createTempMessage`: the optimistic row shown while a send is in flight.
    pub fn temp(
        chat_id: &str,
        sender_id: &str,
        content: &str,
        kind: MessageType,
        reply_to: Option<String>,
    ) -> Self {
        let id = new_id();
        let now = now_iso();
        Self {
            message_id: id.clone(),
            chat_id: chat_id.to_string(),
            sender_id: Some(sender_id.to_string()),
            content: Some(content.to_string()),
            message_type: kind,
            reply_to,
            media_url: None,
            file_name: None,
            file_size: None,
            mime_type: None,
            status: MessageStatus::Sending,
            is_read: false,
            client_temp_id: Some(id),
            sequence_id: None,
            is_edited: false,
            raw_json: None,
            sent_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Sender name and avatar, read from whichever field the backend used.
    pub fn sender(&self) -> Sender {
        let Some(raw) = self
            .raw_json
            .as_deref()
            .and_then(|r| serde_json::from_str::<Value>(r).ok())
        else {
            return Sender::default();
        };
        let sender = raw.get("sender").cloned().unwrap_or(Value::Null);
        Sender {
            name: s(&sender, "name")
                .or_else(|| s(&sender, "display_name"))
                .or_else(|| s(&sender, "full_name"))
                .or_else(|| s(&raw, "sender_name")),
            avatar_url: s(&sender, "avatar_url")
                .or_else(|| s(&sender, "avatar"))
                .or_else(|| s(&sender, "profile_image"))
                .or_else(|| s(&raw, "sender_avatar_url"))
                .or_else(|| s(&raw, "sender_avatar"))
                .or_else(|| s(&raw, "sender_profile_image")),
        }
    }

    /// Unsent by its sender (`is_deleted` from the server): content is cleared.
    pub fn unsent(&self) -> bool {
        self.raw_json
            .as_deref()
            .and_then(|r| serde_json::from_str::<Value>(r).ok())
            .is_some_and(|v| b(&v, "is_deleted"))
    }

    pub fn text(&self) -> &str {
        self.content.as_deref().unwrap_or("")
    }
}

#[derive(Clone, Debug)]
pub struct OutboxItem {
    pub id: String,
    pub chat_id: String,
    pub client_temp_id: String,
    pub content: Option<String>,
    pub message_type: MessageType,
    pub reply_to: Option<String>,
    pub retry_count: i64,
    pub created_at: String,
}

/// What the reply bar and the bubble's quote show.
#[derive(Clone, Debug)]
pub struct ReplyTarget {
    pub message_id: String,
    pub sender_name: String,
    pub content: String,
}
