//! Local SQLite store, ported from `local-db/db.client.ts`,
//! `db.repositories.ts`, `messages.db.ts` and `outbox.service.ts`.
//!
//! The schema is the same as the Tauri build so an existing database file can
//! be opened as is.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::models::{Chat, ChatType, Message, MessageStatus, MessageType, OutboxItem, now_iso};

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

const MIGRATIONS: &str = r#"
CREATE TABLE IF NOT EXISTS app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS users (
  user_id TEXT PRIMARY KEY, username TEXT, name TEXT, profile_image TEXT, bio TEXT,
  is_online INTEGER DEFAULT 0, last_seen_at TEXT, raw_json TEXT, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS chats (
  chat_id TEXT PRIMARY KEY, chat_type TEXT NOT NULL, name TEXT, avatar_url TEXT,
  last_message_id TEXT, last_message_preview TEXT, last_message_at TEXT,
  unread_count INTEGER DEFAULT 0, is_pinned INTEGER DEFAULT 0, is_muted INTEGER DEFAULT 0,
  is_archived INTEGER DEFAULT 0, is_deleted INTEGER DEFAULT 0, raw_json TEXT,
  created_at TEXT, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS chat_participants (
  chat_id TEXT NOT NULL, user_id TEXT NOT NULL, role TEXT, joined_at TEXT,
  is_admin INTEGER DEFAULT 0, raw_json TEXT, PRIMARY KEY (chat_id, user_id));
CREATE TABLE IF NOT EXISTS messages (
  message_id TEXT PRIMARY KEY, chat_id TEXT NOT NULL, sender_id TEXT, content TEXT,
  message_type TEXT NOT NULL, reply_to TEXT, media_url TEXT, local_media_path TEXT,
  file_name TEXT, file_size INTEGER, mime_type TEXT, status TEXT NOT NULL,
  is_read INTEGER DEFAULT 0, is_deleted INTEGER DEFAULT 0, client_temp_id TEXT,
  raw_json TEXT, sent_at TEXT, received_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS message_read_states (
  message_id TEXT NOT NULL, user_id TEXT NOT NULL, read_at TEXT NOT NULL, PRIMARY KEY (message_id, user_id));
CREATE TABLE IF NOT EXISTS pending_outbox (
  id TEXT PRIMARY KEY, chat_id TEXT NOT NULL, client_temp_id TEXT NOT NULL, content TEXT,
  message_type TEXT NOT NULL, reply_to TEXT, local_file_path TEXT, status TEXT NOT NULL,
  retry_count INTEGER DEFAULT 0, last_error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS typing_states (
  chat_id TEXT NOT NULL, user_id TEXT NOT NULL, is_typing INTEGER NOT NULL, updated_at TEXT NOT NULL,
  PRIMARY KEY (chat_id, user_id));
CREATE TABLE IF NOT EXISTS bot_generating_states (
  chat_id TEXT NOT NULL, bot_id TEXT NOT NULL, is_generating INTEGER NOT NULL, generating_type TEXT,
  updated_at TEXT NOT NULL, PRIMARY KEY (chat_id, bot_id));
CREATE INDEX IF NOT EXISTS idx_messages_chat_sent_at ON messages(chat_id, sent_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_chat_created_at ON messages(chat_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_chats_last_message_at ON chats(last_message_at DESC);
CREATE INDEX IF NOT EXISTS idx_pending_outbox_status ON pending_outbox(status);
"#;

const MESSAGE_COLS: &str = "message_id, chat_id, sender_id, content, message_type, reply_to, media_url, \
    file_name, file_size, mime_type, status, is_read, client_temp_id, raw_json, sent_at, created_at, updated_at, \
    sequence_id, is_edited";

const CHAT_COLS: &str = "chat_id, chat_type, name, avatar_url, last_message_id, last_message_preview, \
    last_message_at, unread_count, is_pinned, is_muted, is_archived, is_deleted, created_at, updated_at, raw_json";

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(MIGRATIONS)?;
        add_columns(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(MIGRATIONS)?;
        add_columns(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        let conn = self.conn.lock().unwrap();
        Ok(f(&conn)?)
    }

    // ---- chats ----------------------------------------------------------

    pub fn upsert_chats(&self, chats: &[Chat]) -> Result<()> {
        self.with(|c| {
            let mut stmt = c.prepare_cached(&format!(
                "INSERT OR REPLACE INTO chats ({CHAT_COLS}) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
            ))?;
            for ch in chats {
                stmt.execute(params![
                    ch.chat_id,
                    ch.chat_type.as_str(),
                    ch.name,
                    ch.avatar_url,
                    ch.last_message_id,
                    ch.last_message_preview,
                    ch.last_message_at,
                    ch.unread_count,
                    ch.is_pinned,
                    ch.is_muted,
                    ch.is_archived,
                    ch.is_deleted,
                    ch.created_at,
                    ch.updated_at,
                    ch.raw_json,
                ])?;
            }
            Ok(())
        })
    }

    pub fn all_chats(&self) -> Result<Vec<Chat>> {
        self.with(|c| {
            let mut stmt = c.prepare(&format!(
                "SELECT {CHAT_COLS} FROM chats WHERE is_deleted = 0 ORDER BY is_pinned DESC, last_message_at DESC"
            ))?;
            stmt.query_map([], chat_row)?.collect()
        })
    }

    pub fn update_chat_last_message(
        &self,
        chat_id: &str,
        message_id: &str,
        preview: &str,
        at: &str,
    ) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE chats SET last_message_id = ?, last_message_preview = ?, last_message_at = ?, updated_at = ? WHERE chat_id = ?",
                params![message_id, preview, at, now_iso(), chat_id],
            )
        })?;
        Ok(())
    }

    pub fn set_unread(&self, chat_id: &str, count: i64) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE chats SET unread_count = ?, updated_at = ? WHERE chat_id = ?",
                params![count, now_iso(), chat_id],
            )
        })?;
        Ok(())
    }

    // ---- messages -------------------------------------------------------

    pub fn upsert_message(&self, m: &Message) -> Result<()> {
        self.with(|c| {
            c.execute(
                &format!(
                    "INSERT OR REPLACE INTO messages ({MESSAGE_COLS}, is_deleted, received_at) \
                     VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,?)"
                ),
                params![
                    m.message_id,
                    m.chat_id,
                    m.sender_id,
                    m.content,
                    m.message_type.as_str(),
                    m.reply_to,
                    m.media_url,
                    m.file_name,
                    m.file_size,
                    m.mime_type,
                    m.status.as_str(),
                    m.is_read,
                    m.client_temp_id,
                    m.raw_json,
                    m.sent_at,
                    m.created_at,
                    m.updated_at,
                    m.sequence_id,
                    m.is_edited,
                    now_iso(),
                ],
            )
        })?;
        Ok(())
    }

    /// Newest first, like `messagesRepo.getByChatId`.
    pub fn messages(
        &self,
        chat_id: &str,
        limit: usize,
        before: Option<&str>,
    ) -> Result<Vec<Message>> {
        self.with(|c| match before {
            Some(before) => {
                let mut stmt = c.prepare(&format!(
                    "SELECT {MESSAGE_COLS} FROM messages WHERE chat_id = ? AND created_at < ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT ?"
                ))?;
                stmt.query_map(params![chat_id, before, limit as i64], message_row)?.collect()
            }
            None => {
                let mut stmt = c.prepare(&format!(
                    "SELECT {MESSAGE_COLS} FROM messages WHERE chat_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT ?"
                ))?;
                stmt.query_map(params![chat_id, limit as i64], message_row)?.collect()
            }
        })
    }

    pub fn message(&self, message_id: &str) -> Result<Option<Message>> {
        self.with(|c| {
            c.query_row(
                &format!("SELECT {MESSAGE_COLS} FROM messages WHERE message_id = ?"),
                [message_id],
                message_row,
            )
            .optional()
        })
    }

    /// Highest server sequence stored for a chat, to spot gaps.
    pub fn max_sequence(&self, chat_id: &str) -> Result<Option<i64>> {
        self.with(|c| {
            c.query_row(
                "SELECT MAX(sequence_id) FROM messages WHERE chat_id = ?",
                [chat_id],
                |r| r.get(0),
            )
        })
    }

    pub fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE messages SET status = ?, updated_at = ? WHERE message_id = ?",
                params![status.as_str(), now_iso(), message_id],
            )
        })?;
        Ok(())
    }

    /// Swaps the optimistic id for the server id once a send succeeds.
    pub fn confirm_temp_message(&self, temp_id: &str, real_id: &str) -> Result<()> {
        self.with(|c| {
            // The WS echo may already have stored the server row; drop the duplicate first.
            c.execute("DELETE FROM messages WHERE message_id = ? AND client_temp_id IS NOT ?", params![real_id, temp_id])?;
            c.execute(
                "UPDATE messages SET message_id = ?, status = 'sent', updated_at = ? WHERE client_temp_id = ?",
                params![real_id, now_iso(), temp_id],
            )
        })?;
        Ok(())
    }

    pub fn mark_message_read(&self, message_id: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE messages SET is_read = 1, updated_at = ? WHERE message_id = ?",
                params![now_iso(), message_id],
            )
        })?;
        Ok(())
    }

    pub fn mark_chat_read(&self, chat_id: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE messages SET is_read = 1, updated_at = ? WHERE chat_id = ? AND is_read = 0",
                params![now_iso(), chat_id],
            )
        })?;
        Ok(())
    }

    pub fn mark_message_deleted(&self, message_id: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE messages SET is_deleted = 1, updated_at = ? WHERE message_id = ?",
                params![now_iso(), message_id],
            )
        })?;
        Ok(())
    }

    /// Every visible message in a chat, oldest first (export, media panel).
    pub fn all_messages(&self, chat_id: &str) -> Result<Vec<Message>> {
        self.with(|c| {
            let mut stmt = c.prepare(&format!(
                "SELECT {MESSAGE_COLS} FROM messages WHERE chat_id = ? AND is_deleted = 0 ORDER BY created_at ASC"
            ))?;
            stmt.query_map([chat_id], message_row)?.collect()
        })
    }

    pub fn set_chat_muted(&self, chat_id: &str, muted: bool) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE chats SET is_muted = ?, updated_at = ? WHERE chat_id = ?",
                params![muted, now_iso(), chat_id],
            )
        })?;
        Ok(())
    }

    // ---- app_meta (small local key/value) ------------------------------

    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        self.with(|c| {
            c.query_row("SELECT value FROM app_meta WHERE key = ?", [key], |r| {
                r.get(0)
            })
            .optional()
        })
    }

    pub fn meta_set(&self, key: &str, value: Option<&str>) -> Result<()> {
        self.with(|c| match value {
            Some(v) => c.execute(
                "INSERT OR REPLACE INTO app_meta (key, value, updated_at) VALUES (?, ?, ?)",
                params![key, v, now_iso()],
            ),
            None => c.execute("DELETE FROM app_meta WHERE key = ?", [key]),
        })?;
        Ok(())
    }

    // ---- outbox ---------------------------------------------------------

    pub fn outbox_insert(&self, item: &OutboxItem) -> Result<()> {
        let now = now_iso();
        self.with(|c| {
            c.execute(
                "INSERT OR REPLACE INTO pending_outbox (id, chat_id, client_temp_id, content, message_type, reply_to, local_file_path, status, retry_count, last_error, created_at, updated_at) \
                 VALUES (?,?,?,?,?,?,NULL,'queued',?,NULL,?,?)",
                params![item.id, item.chat_id, item.client_temp_id, item.content, item.message_type.as_str(), item.reply_to, item.retry_count, now, now],
            )
        })?;
        Ok(())
    }

    pub fn outbox_pending(&self) -> Result<Vec<OutboxItem>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, chat_id, client_temp_id, content, message_type, reply_to, retry_count, created_at FROM pending_outbox \
                 WHERE status IN ('queued','failed') ORDER BY created_at ASC",
            )?;
            stmt.query_map([], |r| {
                Ok(OutboxItem {
                    id: r.get(0)?,
                    chat_id: r.get(1)?,
                    client_temp_id: r.get(2)?,
                    content: r.get(3)?,
                    message_type: MessageType::parse(&r.get::<_, String>(4)?),
                    reply_to: r.get(5)?,
                    retry_count: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })?
            .collect()
        })
    }

    pub fn outbox_failed(&self, id: &str, error: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE pending_outbox SET retry_count = retry_count + 1, last_error = ?, status = 'failed', updated_at = ? WHERE id = ?",
                params![error, now_iso(), id],
            )
        })?;
        Ok(())
    }

    pub fn outbox_remove(&self, id: &str) -> Result<()> {
        self.with(|c| c.execute("DELETE FROM pending_outbox WHERE id = ?", [id]))?;
        Ok(())
    }

    pub fn outbox_remove_by_temp(&self, temp_id: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "DELETE FROM pending_outbox WHERE client_temp_id = ?",
                [temp_id],
            )
        })?;
        Ok(())
    }
}

/// Columns added after the Tauri schema. `ALTER TABLE ADD COLUMN` has no
/// `IF NOT EXISTS`, so check `table_info` first; safe on old and new files.
fn add_columns(conn: &Connection) -> rusqlite::Result<()> {
    let existing: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('messages')")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for (name, ddl) in [
        (
            "sequence_id",
            "ALTER TABLE messages ADD COLUMN sequence_id INTEGER",
        ),
        (
            "is_edited",
            "ALTER TABLE messages ADD COLUMN is_edited INTEGER DEFAULT 0",
        ),
    ] {
        if !existing.iter().any(|c| c == name) {
            conn.execute(ddl, [])?;
        }
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_chat_sequence ON messages(chat_id, sequence_id)",
        [],
    )?;
    Ok(())
}

fn chat_row(r: &Row) -> rusqlite::Result<Chat> {
    Ok(Chat {
        chat_id: r.get(0)?,
        chat_type: ChatType::parse(&r.get::<_, String>(1)?),
        name: r.get(2)?,
        avatar_url: r.get(3)?,
        last_message_id: r.get(4)?,
        last_message_preview: r.get(5)?,
        last_message_at: r.get(6)?,
        unread_count: r.get::<_, Option<i64>>(7)?.unwrap_or(0),
        is_pinned: r.get::<_, Option<bool>>(8)?.unwrap_or(false),
        is_muted: r.get::<_, Option<bool>>(9)?.unwrap_or(false),
        is_archived: r.get::<_, Option<bool>>(10)?.unwrap_or(false),
        is_deleted: r.get::<_, Option<bool>>(11)?.unwrap_or(false),
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
        raw_json: r.get::<_, Option<String>>(14)?.unwrap_or_default(),
    })
}

fn message_row(r: &Row) -> rusqlite::Result<Message> {
    Ok(Message {
        message_id: r.get(0)?,
        chat_id: r.get(1)?,
        sender_id: r.get(2)?,
        content: r.get(3)?,
        message_type: MessageType::parse(&r.get::<_, String>(4)?),
        reply_to: r.get(5)?,
        media_url: r.get(6)?,
        file_name: r.get(7)?,
        file_size: r.get(8)?,
        mime_type: r.get(9)?,
        status: MessageStatus::parse(&r.get::<_, String>(10)?),
        is_read: r.get::<_, Option<bool>>(11)?.unwrap_or(false),
        client_temp_id: r.get(12)?,
        raw_json: r.get(13)?,
        sent_at: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        sequence_id: r.get(17)?,
        is_edited: r.get::<_, Option<bool>>(18)?.unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_roundtrip_and_temp_confirm() {
        let db = Db::in_memory().unwrap();
        let temp = Message::temp("c1", "u1", "hello", MessageType::Text, None);
        db.upsert_message(&temp).unwrap();
        db.confirm_temp_message(&temp.message_id, "server-1")
            .unwrap();
        let got = db.messages("c1", 10, None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].message_id, "server-1");
        assert_eq!(got[0].status, MessageStatus::Sent);
    }

    #[test]
    fn confirm_drops_ws_echo_duplicate() {
        let db = Db::in_memory().unwrap();
        let temp = Message::temp("c1", "u1", "hi", MessageType::Text, None);
        db.upsert_message(&temp).unwrap();
        let echo =
            Message::from_value(&json!({"message_id": "s1", "chat_id": "c1", "content": "hi"}));
        db.upsert_message(&echo).unwrap();
        db.confirm_temp_message(&temp.message_id, "s1").unwrap();
        assert_eq!(db.messages("c1", 10, None).unwrap().len(), 1);
    }

    #[test]
    fn chats_normalize_and_persist() {
        let db = Db::in_memory().unwrap();
        let chat = Chat::from_value(&json!({
            "chat_id": "c1", "chat_type": "direct", "unread_count": 2,
            "participants": [{"user_id": "u2", "name": "Ploy", "avatar": "https://x/y.png", "joined_at": null, "is_admin": false}],
            "last_message": {"message_id": "m9", "content": "ok", "created_at": "2026-11-20T10:00:00Z"},
            "updated_at": "2026-11-20T10:00:00Z"
        }))
        .unwrap();
        assert_eq!(chat.name.as_deref(), Some("Ploy"));
        assert_eq!(chat.last_message_preview.as_deref(), Some("ok"));
        db.upsert_chats(&[chat]).unwrap();
        let all = db.all_chats().unwrap();
        assert_eq!(all[0].unread_count, 2);
        assert_eq!(all[0].avatar_url.as_deref(), Some("https://x/y.png"));
    }

    #[test]
    fn spec_message_shape_and_sequence() {
        let db = Db::in_memory().unwrap();
        let m = Message::from_value(&json!({
            "message_id": "m1", "chat_id": "c1", "sequence_id": 41,
            "sender": {"user_id": "u1", "name": "Ploy", "avatar": "https://x/a.png"},
            "client_message_id": "tmp-1", "message_type": "image",
            "media": {"url": "https://x/p.jpg", "name": "p.jpg", "size": 2048, "mime_type": "image/jpeg"},
            "read_by": [{"user_id": "u2", "read_at": "2026-11-23T01:00:00Z"}],
            "created_at": "2026-11-23T00:59:00Z"
        }));
        assert_eq!(m.sender_id.as_deref(), Some("u1"));
        assert_eq!(m.client_temp_id.as_deref(), Some("tmp-1"));
        assert_eq!(m.media_url.as_deref(), Some("https://x/p.jpg"));
        assert_eq!(m.file_size, Some(2048));
        assert_eq!(m.status, MessageStatus::Read);
        db.upsert_message(&m).unwrap();
        assert_eq!(db.max_sequence("c1").unwrap(), Some(41));
        assert_eq!(
            db.messages("c1", 10, None).unwrap()[0].sequence_id,
            Some(41)
        );
    }

    #[test]
    fn adds_columns_to_a_tauri_era_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS).unwrap();
        add_columns(&conn).unwrap();
        add_columns(&conn).unwrap(); // idempotent
    }

    #[test]
    fn finds_direct_chat_by_participant() {
        let chat = Chat::from_value(&json!({
            "chat_id": "c9", "chat_type": "direct",
            "participants": [{"user_id": "me"}, {"user_id": "u-ploy", "name": "Ploy"}]
        }))
        .unwrap();
        assert!(chat.is_direct_with("u-ploy"));
        assert_eq!(chat.partner_id("me").as_deref(), Some("u-ploy"));
        assert!(!chat.is_direct_with("u-other"));
        let group = Chat::from_value(&json!({"chat_id": "g1", "chat_type": "group", "participants": [{"user_id": "u-ploy"}]})).unwrap();
        assert!(!group.is_direct_with("u-ploy"));
    }

    #[test]
    fn meta_and_mute() {
        let db = Db::in_memory().unwrap();
        db.meta_set("pin:c1", Some("m9")).unwrap();
        assert_eq!(db.meta_get("pin:c1").unwrap().as_deref(), Some("m9"));
        db.meta_set("pin:c1", None).unwrap();
        assert_eq!(db.meta_get("pin:c1").unwrap(), None);
        let chat = Chat::from_value(&json!({"chat_id": "c1", "chat_type": "direct"})).unwrap();
        db.upsert_chats(&[chat]).unwrap();
        db.set_chat_muted("c1", true).unwrap();
        assert!(db.all_chats().unwrap()[0].is_muted);
    }

    #[test]
    fn outbox_lifecycle() {
        let db = Db::in_memory().unwrap();
        let item = OutboxItem {
            id: "o1".into(),
            chat_id: "c1".into(),
            client_temp_id: "t1".into(),
            content: Some("x".into()),
            message_type: MessageType::Text,
            reply_to: None,
            retry_count: 0,
            created_at: crate::models::now_iso(),
        };
        db.outbox_insert(&item).unwrap();
        db.outbox_failed("o1", "offline").unwrap();
        let pending = db.outbox_pending().unwrap();
        assert_eq!(pending[0].retry_count, 1);
        db.outbox_remove_by_temp("t1").unwrap();
        assert!(db.outbox_pending().unwrap().is_empty());
    }
}
