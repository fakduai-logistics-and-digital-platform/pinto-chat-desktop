//! Application state, replacing the Pinia stores (`auth`, `chats`,
//! `messages`, `settings`). SQLite stays the source of truth for chats and
//! messages; the in-memory lists here are a view of it for rendering.
//!
//! Network calls run on GPUI's background executor. Realtime frames arrive on
//! a channel and are applied here on the UI thread, writing SQLite first.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use futures::StreamExt;
use gpui_kit::*;
use serde_json::Value;

use crate::{
    api::{Api, ApiError},
    db::Db,
    env::{CHAT_PAGE_SIZE, Env, MESSAGE_PAGE_SIZE, TYPING_AUTO_CLEAR_MS},
    models::{
        Chat, ChatType, Message, MessageStatus, MessageType, OutboxItem, ReplyTarget, UserProfile,
        new_id,
    },
    persist::{Settings, StoredSession},
    realtime::{self, ChatEvent, ConnectionState, SocketHandle},
};

/// Auto-resend only what was written recently and has not kept failing.
const OUTBOX_MAX_AGE_HOURS: i64 = 24;
const OUTBOX_MAX_AUTO_RETRIES: i64 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Checking a stored token on launch.
    Restoring,
    SignedOut,
    SignedIn,
}

pub enum StoreEvent {
    SignedIn,
    SignedOut,
    Toast(SharedString),
    /// A message arrived for a chat that is not open.
    Notify {
        title: String,
        body: String,
    },
    /// The open chat got new rows; the room may want to stick to the bottom.
    MessagesAppended,
}

#[derive(Clone)]
pub enum RemoteImage {
    Loading,
    Ready(Arc<Image>),
    Failed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatFilter {
    #[default]
    All,
    Direct,
    Group,
    Bot,
}

impl ChatFilter {
    pub fn api_chat_type(self) -> Option<ChatType> {
        match self {
            Self::All | Self::Bot => None,
            Self::Direct => Some(ChatType::Direct),
            Self::Group => Some(ChatType::Group),
        }
    }
}

pub struct Store {
    pub env: Env,
    api: Api,
    db: Option<Db>,
    pub settings: Settings,

    pub phase: Phase,
    pub user: Option<UserProfile>,
    pub auth_busy: bool,
    pub auth_error: Option<String>,

    chats: HashMap<String, Chat>,
    pub selected_chat: Option<String>,
    pub filter: ChatFilter,
    pub show_archived: bool,
    pub search: String,
    chat_page: usize,
    pub chats_loading: bool,
    pub chats_loading_more: bool,
    pub chats_has_more: bool,
    /// People you follow, loaded when the friends list is first opened.
    pub friends: Vec<UserProfile>,
    pub friends_loading: bool,
    pub blocked_users: Vec<UserProfile>,
    pub blocked_ids: HashSet<String>,
    pub blocked_loading: bool,
    /// Results of the find-people search.
    pub people: Vec<UserProfile>,
    pub people_query: String,
    pub people_loading: bool,
    people_epoch: u64,
    friends_loaded: bool,
    opening_direct: bool,
    /// Partner ids already looked up for unnamed chats, so each is fetched once.
    resolved_partners: HashSet<String>,
    typing: HashSet<String>,
    typing_epoch: HashMap<String, u64>,

    messages: HashMap<String, Vec<Message>>,
    chat_has_more: HashMap<String, bool>,
    prefetched_chats: HashSet<String>,
    pub messages_loading: bool,
    pub messages_loading_more: bool,
    pub messages_has_more: bool,

    pub connection: ConnectionState,
    socket: Option<SocketHandle>,
    flushing: bool,
    /// Pinned message per chat (`None` = loaded, nothing pinned).
    pinned: HashMap<String, Option<String>>,
    images: HashMap<String, RemoteImage>,
    search_epoch: u64,
}

impl EventEmitter<StoreEvent> for Store {}

impl Store {
    pub fn new(env: Env, cx: &mut Context<Self>) -> Self {
        let api = Api::new(env.api_base_url.clone());
        let db = match Db::open(&crate::env::data_dir().join("pinto-chat.sqlite")) {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("open database: {e:#}");
                None
            }
        };
        // Preview mode (debug only) never reads the real session or database.
        let preview = cfg!(debug_assertions) && std::env::var("PINTO_PREVIEW").is_ok();
        let db = if preview { None } else { db };
        let session = if preview {
            StoredSession::default()
        } else {
            StoredSession::load()
        };
        let phase = if session.access_token.is_some() {
            Phase::Restoring
        } else {
            Phase::SignedOut
        };
        api.set_tokens(session.access_token.clone(), session.refresh_token.clone());

        let mut this = Self {
            env,
            api,
            db,
            settings: Settings::load(),
            phase,
            user: None,
            auth_busy: false,
            auth_error: None,
            chats: HashMap::new(),
            selected_chat: None,
            filter: ChatFilter::All,
            show_archived: false,
            search: String::new(),
            chat_page: 1,
            chats_loading: false,
            chats_loading_more: false,
            chats_has_more: true,
            friends: Vec::new(),
            friends_loading: false,
            blocked_users: Vec::new(),
            blocked_ids: HashSet::new(),
            blocked_loading: false,
            people: Vec::new(),
            people_query: String::new(),
            people_loading: false,
            people_epoch: 0,
            friends_loaded: false,
            opening_direct: false,
            resolved_partners: HashSet::new(),
            typing: HashSet::new(),
            typing_epoch: HashMap::new(),
            messages: HashMap::new(),
            chat_has_more: HashMap::new(),
            prefetched_chats: HashSet::new(),
            messages_loading: false,
            messages_loading_more: false,
            messages_has_more: true,
            connection: ConnectionState::Disconnected,
            socket: None,
            flushing: false,
            pinned: HashMap::new(),
            images: HashMap::new(),
            search_epoch: 0,
        };
        if this.phase == Phase::Restoring {
            this.restore_session(cx);
        }
        this
    }

    fn db(&self) -> Option<&Db> {
        self.db.as_ref()
    }

    // ---- auth -----------------------------------------------------------

    fn restore_session(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { api.profile() }).await;
            this.update(cx, |this, cx| match result {
                Ok(user) => this.finish_sign_in(user, cx),
                // Same as `fetchProfile`: any failure drops the stored token.
                Err(e) => {
                    if e.status.is_none() {
                        this.auth_error = Some(e.message);
                    }
                    this.clear_session(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn login(&mut self, identifier: String, password: String, cx: &mut Context<Self>) {
        if self.auth_busy {
            return;
        }
        self.auth_busy = true;
        self.auth_error = None;
        cx.notify();
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let tokens = api.login(&identifier, &password)?;
                    api.set_tokens(
                        Some(tokens.access_token.clone()),
                        tokens.refresh_token.clone(),
                    );
                    let user = api.profile()?;
                    Ok::<_, ApiError>((tokens, user))
                })
                .await;
            this.update(cx, |this, cx| {
                this.auth_busy = false;
                match result {
                    Ok((tokens, user)) => {
                        StoredSession {
                            access_token: Some(tokens.access_token),
                            refresh_token: tokens.refresh_token,
                        }
                        .save();
                        this.finish_sign_in(user, cx);
                    }
                    Err(e) => {
                        this.api.set_tokens(None, None);
                        this.auth_error = Some(e.message);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// QR login: tokens come from the WS; fall back to the WS user if the
    /// profile endpoint fails, as `loginWithQrTokens` does.
    pub fn login_with_tokens(
        &mut self,
        access: String,
        refresh: Option<String>,
        ws_user: Option<Value>,
        cx: &mut Context<Self>,
    ) {
        self.auth_busy = true;
        self.auth_error = None;
        cx.notify();
        StoredSession {
            access_token: Some(access.clone()),
            refresh_token: refresh.clone(),
        }
        .save();
        self.api.set_tokens(Some(access), refresh);
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { api.profile() }).await;
            this.update(cx, |this, cx| {
                this.auth_busy = false;
                match result
                    .ok()
                    .or_else(|| ws_user.as_ref().and_then(UserProfile::from_value))
                {
                    Some(user) => this.finish_sign_in(user, cx),
                    None => {
                        this.auth_error = Some("โหลดโปรไฟล์ไม่สำเร็จ".into());
                        this.clear_session(cx);
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    fn finish_sign_in(&mut self, user: UserProfile, cx: &mut Context<Self>) {
        self.user = Some(user);
        self.phase = Phase::SignedIn;
        self.load_cached_chats();
        self.prefetch_top_chats(cx);
        self.load_chats(true, cx);
        self.load_blocked_users(cx);
        self.connect_socket(cx);
        self.flush_outbox(cx);
        cx.emit(StoreEvent::SignedIn);
        cx.notify();
    }

    /// Debug builds only: fills the store with sample rows so the chat screens
    /// can be checked without an account. Nothing touches the network or SQLite.
    #[cfg(debug_assertions)]
    pub fn seed_preview(&mut self, cx: &mut Context<Self>) {
        use chrono::{Duration as D, Utc};
        let at = |mins: i64| (Utc::now() - D::minutes(mins)).to_rfc3339();
        self.db = None;
        self.user = Some(UserProfile {
            user_id: "me".into(),
            username: "nattapong.k".into(),
            name: "ณัฐพงษ์ แก้วมณี".into(),
            profile_image: None,
            is_official: false,
        });
        self.phase = Phase::SignedIn;
        self.connection = ConnectionState::Connected;
        let chats = [
            (
                "c1",
                "direct",
                "พลอยไพลิน ศรีสุข",
                "พรุ่งนี้ประชุม 10 โมงนะ ส่งสไลด์ให้แล้ว",
                4,
                0,
                true,
            ),
            (
                "c2",
                "group",
                "ทีมขนส่ง ภาคเหนือ",
                "รถเข้าคลังลำปางแล้ว 3 คัน",
                12,
                3,
                false,
            ),
            (
                "c3",
                "direct",
                "Somchai P.",
                "โอเคครับ เดี๋ยวโทรกลับ",
                95,
                0,
                false,
            ),
            (
                "c4",
                "group",
                "Pinto Design",
                "อัปเดตไอคอนชุดใหม่ใน Figma แล้ว",
                60 * 20,
                1,
                false,
            ),
            ("c5", "direct", "กมลวรรณ ใจดี", "ขอบคุณค่ะ", 60 * 30, 0, false),
            (
                "c6",
                "group",
                "ฝ่ายบัญชี",
                "ใบแจ้งหนี้เดือนตุลาคมครบแล้ว",
                60 * 24 * 3,
                0,
                false,
            ),
            (
                "c7",
                "direct",
                "อนุชา วงศ์ใหญ่",
                "รับทราบครับ",
                60 * 24 * 9,
                0,
                false,
            ),
        ];
        for (id, kind, name, preview, mins, unread, pinned) in chats {
            let mut c = Chat::from_value(&serde_json::json!({
                "chat_id": id, "chat_type": kind, "name": name, "unread_count": unread,
                "is_pinned": pinned, "updated_at": at(mins),
                "last_message": { "message_id": format!("{id}-last"), "content": preview, "created_at": at(mins) }
            }))
            .unwrap();
            c.is_pinned = pinned;
            self.chats.insert(id.into(), c);
        }
        let thread = [
            ("p", "เช้านี้ส่งไฟล์สรุปยอดให้แล้วนะ ดูทันไหม", 60 * 26, None),
            ("me", "เห็นแล้ว ขอบคุณมาก", 60 * 25, None),
            ("p", "พรุ่งนี้ประชุมกับทีมขนส่งกี่โมงนะ", 42, None),
            (
                "p",
                "จะได้เตรียมตัวเลขไปด้วย ยอดส่งของภาคเหนือเดือนนี้เพิ่มขึ้นจากเดือนที่แล้วพอสมควร โดยเฉพาะเส้นทางเชียงใหม่ลำปางที่รถเต็มทุกเที่ยวตั้งแต่สัปดาห์ที่สอง",
                41,
                None,
            ),
            ("me", "10 โมงที่ห้องประชุมชั้น 4", 30, Some("m3")),
            ("me", "เอาตัวเลขเดือนตุลาไปด้วยก็ดี", 29, None),
            ("p", "ได้เลย", 6, None),
            ("p", "พรุ่งนี้ประชุม 10 โมงนะ ส่งสไลด์ให้แล้ว", 4, None),
            (
                "p",
                "สไลด์อยู่ที่ https://pinto-app.com/decks/north-oct นะ",
                3,
                None,
            ),
            ("me", "ได้รับแล้ว เดี๋ยวเปิดดูคืนนี้", 2, None),
        ];
        let msgs: Vec<Message> = thread
            .iter()
            .enumerate()
            .map(|(i, (who, text, mins, reply))| {
                let mut m = Message::from_value(&serde_json::json!({
                    "message_id": format!("m{}", i + 1), "chat_id": "c1",
                    "sender_id": if *who == "me" { "me" } else { "u-ploy" },
                    "sender": { "name": "พลอยไพลิน ศรีสุข" },
                    "content": text, "created_at": at(*mins), "reply_to": reply,
                }));
                if i == 5 {
                    m.status = MessageStatus::Read;
                }
                if i == 9 {
                    m.status = MessageStatus::Failed;
                }
                m
            })
            .collect();
        let mut msgs = msgs;
        let mut file = Message::from_value(&serde_json::json!({
            "message_id": "m-file", "chat_id": "c1", "sender_id": "u-ploy", "message_type": "file",
            "media": {"url": "https://pinto-app.com/files/north-oct.pdf", "name": "สรุปยอดภาคเหนือ ต.ค.pdf", "size": 1_482_000},
            "created_at": at(60 * 25 + 30),
        }));
        file.status = MessageStatus::Sent;
        msgs.insert(1, file);
        self.messages.insert("c1".into(), msgs);
        self.pinned.insert("c1".into(), Some("m3".into()));
        self.friends = [
            ("u-ploy", "ploy.s", "พลอยไพลิน ศรีสุข"),
            ("u-kamol", "kamonwan.j", "กมลวรรณ ใจดี"),
            ("u-somchai", "somchai.p", "Somchai P."),
            ("u-anucha", "anucha.w", "อนุชา วงศ์ใหญ่"),
            ("u-dao", "dao.r", "ดาวเรือง ทองดี"),
            ("u-mek", "mek.t", "เมฆ ธนากร"),
        ]
        .into_iter()
        .map(|(id, user, name)| UserProfile {
            user_id: id.into(),
            username: user.into(),
            name: name.into(),
            profile_image: None,
            is_official: false,
        })
        .collect();
        self.friends_loaded = true;
        self.people_query = "pinto".into();
        self.people = [
            ("b-assist", "pinto.assistant", "Pinto Assistant", true),
            ("b-ship", "pinto.shipping", "Pinto ขนส่ง", true),
            ("u-ploy", "ploy.s", "พลอยไพลิน ศรีสุข", false),
            ("u-pin", "pinnapa.k", "ปิ่นนภา คงสุข", false),
        ]
        .into_iter()
        .map(|(id, user, name, official)| UserProfile {
            user_id: id.into(),
            username: user.into(),
            name: name.into(),
            profile_image: None,
            is_official: official,
        })
        .collect();
        if let Some(c) = self.chats.get_mut("c3") {
            c.is_muted = true;
        }
        let mode = std::env::var("PINTO_PREVIEW").unwrap_or_default();
        self.selected_chat = mode.contains("room").then(|| "c1".to_string());
        if mode.contains("dark") {
            self.settings.theme = crate::persist::ThemeMode::Dark;
        }
        cx.emit(StoreEvent::SignedIn);
        cx.notify();
    }

    pub fn logout(&mut self, cx: &mut Context<Self>) {
        // Snapshot the tokens: clear_session wipes the shared ones right after.
        let api = self.api.detached();
        cx.background_spawn(async move {
            let _ = api.logout();
        })
        .detach();
        self.clear_session(cx);
    }

    fn clear_session(&mut self, cx: &mut Context<Self>) {
        StoredSession::clear();
        self.api.set_tokens(None, None);
        self.socket = None;
        self.connection = ConnectionState::Disconnected;
        self.user = None;
        self.phase = Phase::SignedOut;
        self.chats.clear();
        self.messages.clear();
        self.selected_chat = None;
        cx.emit(StoreEvent::SignedOut);
        cx.notify();
    }

    // ---- settings -------------------------------------------------------

    pub fn update_settings(&mut self, f: impl FnOnce(&mut Settings), cx: &mut Context<Self>) {
        f(&mut self.settings);
        self.settings.save();
        cx.notify();
    }

    // ---- chats ----------------------------------------------------------

    fn load_cached_chats(&mut self) {
        if let Some(rows) = self.db().and_then(|db| db.all_chats().ok()) {
            for c in rows {
                self.chats.insert(c.chat_id.clone(), c);
            }
        }
    }

    pub fn load_chats(&mut self, reset: bool, cx: &mut Context<Self>) {
        if self.phase != Phase::SignedIn && self.user.is_none() {
            return;
        }
        if reset {
            self.chat_page = 1;
            self.chats_has_more = true;
        } else if self.chats_loading || self.chats_loading_more || !self.chats_has_more {
            return;
        }
        self.chats_loading = reset && self.chats.is_empty();
        self.chats_loading_more = !reset;
        cx.notify();

        let api = self.api.clone();
        let (page, q, kind) = (
            self.chat_page,
            self.search.clone(),
            self.filter.api_chat_type(),
        );
        let epoch = self.search_epoch;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { api.chats(page, CHAT_PAGE_SIZE, Some(&q), kind) })
                .await;
            this.update(cx, |this, cx| {
                if epoch != this.search_epoch {
                    return; // a newer search replaced this one
                }
                this.chats_loading = false;
                this.chats_loading_more = false;
                match result {
                    Ok(page) => {
                        // Keep a name/avatar looked up earlier when the server
                        // still sends this chat without one.
                        let chats: Vec<Chat> = page
                            .chats
                            .into_iter()
                            .map(|mut c| {
                                if let Some(old) = this.chats.get(&c.chat_id) {
                                    if c.name.is_none() {
                                        c.name = old.name.clone();
                                    }
                                    if c.avatar_url.is_none() {
                                        c.avatar_url = old.avatar_url.clone();
                                    }
                                }
                                c
                            })
                            .collect();
                        if let Some(db) = this.db()
                            && let Err(e) = db.upsert_chats(&chats)
                        {
                            log::warn!("cache chats: {e:#}");
                        }
                        if reset && !this.search.is_empty() {
                            this.chats.clear();
                        }
                        this.chats_has_more = chats.len() == CHAT_PAGE_SIZE;
                        for c in chats {
                            this.chats.insert(c.chat_id.clone(), c);
                        }
                        this.chat_page += 1;
                        this.resolve_unnamed_chats(cx);
                        this.prefetch_top_chats(cx);
                    }
                    Err(e) => log::warn!("load chats: {e}"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Pinned first, then newest activity. Filter and search apply locally
    /// too, so cached rows narrow immediately while the API catches up.
    pub fn sorted_chats(&self) -> Vec<Chat> {
        let q = self.search.to_lowercase();
        let mut list: Vec<Chat> = self
            .chats
            .values()
            .filter(|c| !c.is_deleted)
            .filter(|c| {
                if !q.is_empty() {
                    true
                } else if self.show_archived {
                    c.is_archived
                } else {
                    !c.is_archived
                }
            })
            .filter(|c| match self.filter {
                ChatFilter::All => true,
                ChatFilter::Direct => c.chat_type == ChatType::Direct,
                ChatFilter::Group => c.chat_type == ChatType::Group,
                ChatFilter::Bot => self.is_chat_bot(c),
            })
            .filter(|c| {
                q.is_empty()
                    || c.display_name().to_lowercase().contains(&q)
                    || c.last_message_preview
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.sort_time().cmp(&a.sort_time()))
        });
        list
    }

    pub fn chat_type(&self, id: &str) -> Option<ChatType> {
        self.chats.get(id).map(|c| c.chat_type)
    }

    pub fn selected(&self) -> Option<&Chat> {
        self.selected_chat
            .as_deref()
            .and_then(|id| self.chats.get(id))
    }

    pub fn total_unread(&self) -> i64 {
        self.chats.values().map(|c| c.unread_count).sum()
    }

    pub fn is_typing(&self, chat_id: &str) -> bool {
        self.typing.contains(chat_id)
    }

    pub fn set_filter(&mut self, filter: ChatFilter, cx: &mut Context<Self>) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        if self.chats.is_empty() {
            self.search_epoch += 1;
            self.load_chats(true, cx);
        } else {
            cx.notify();
        }
    }

    /// Debounced in the view; this runs the query.
    pub fn set_search(&mut self, q: String, cx: &mut Context<Self>) {
        if self.search == q {
            return;
        }
        self.search = q;
        self.search_epoch += 1;
        if self.search.is_empty() && self.filter == ChatFilter::All {
            self.load_cached_chats();
        }
        self.load_chats(true, cx);
    }

    pub fn select_chat(&mut self, chat_id: Option<String>, cx: &mut Context<Self>) {
        if self.selected_chat == chat_id {
            return;
        }
        self.selected_chat = chat_id.clone();
        if let Some(id) = chat_id {
            self.load_pin(&id);
            self.load_messages(&id, cx);
            self.mark_read(&id, cx);
            if self
                .chats
                .get(&id)
                .is_some_and(|c| c.chat_type == ChatType::Group)
            {
                self.load_chat_detail(&id, cx);
            }
        }
        cx.notify();
    }

    pub fn load_chat_detail(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        if self.phase != Phase::SignedIn {
            return;
        }
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.spawn(async move |this, cx| {
            let fetch_id = id.clone();
            let res = cx
                .background_spawn(async move { api.chat_detail(&fetch_id) })
                .await;
            if let Ok(mut fresh) = res {
                let _ = this.update(cx, |this, cx| {
                    if let Some(old) = this.chats.get(&id) {
                        fresh.is_pinned = old.is_pinned;
                        fresh.is_muted = old.is_muted;
                        fresh.is_archived = old.is_archived;
                    }
                    if let Some(db) = this.db() {
                        let _ = db.upsert_chats(std::slice::from_ref(&fresh));
                    }
                    this.chats.insert(id, fresh);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Some direct chats arrive with an empty participant name. Look the
    /// partner up by id once, then keep the name and avatar in SQLite.
    fn resolve_unnamed_chats(&mut self, cx: &mut Context<Self>) {
        let Some(me) = self.user.as_ref().map(|u| u.user_id.clone()) else {
            return;
        };
        let todo: Vec<(String, String)> = self
            .chats
            .values()
            .filter(|c| c.name.is_none())
            .filter_map(|c| Some((c.chat_id.clone(), c.partner_id(&me)?)))
            .filter(|(_, partner)| !self.resolved_partners.contains(partner))
            .collect();
        if todo.is_empty() {
            return;
        }
        for (_, partner) in &todo {
            self.resolved_partners.insert(partner.clone());
        }
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            for (chat_id, partner) in todo {
                let api = api.clone();
                let found = cx.background_spawn(async move { api.user(&partner) }).await;
                let Ok(Some(user)) = found else { continue };
                let keep = this.update(cx, |this, cx| {
                    if let Some(chat) = this.chats.get_mut(&chat_id) {
                        chat.name = Some(user.display_name().to_string());
                        if chat.avatar_url.is_none() {
                            chat.avatar_url = user.profile_image.clone();
                        }
                        let updated = chat.clone();
                        if let Some(db) = this.db() {
                            let _ = db.upsert_chats(std::slice::from_ref(&updated));
                        }
                        cx.notify();
                    }
                });
                if keep.is_err() {
                    return;
                }
            }
        })
        .detach();
    }

    // ---- blocks ---------------------------------------------------------

    pub fn is_chat_official(&self, chat: &Chat) -> bool {
        let me = self.user.as_ref().map(|u| u.user_id.as_str()).unwrap_or("");
        chat.partner_is_official(me)
    }

    pub fn is_chat_bot(&self, chat: &Chat) -> bool {
        let me = self.user.as_ref().map(|u| u.user_id.as_str()).unwrap_or("");
        chat.partner_is_bot(me)
    }

    pub fn selected_partner_id(&self) -> Option<String> {
        let me = self.user.as_ref()?.user_id.as_str();
        self.selected()?.partner_id(me)
    }

    pub fn is_blocked(&self, user_id: &str) -> bool {
        self.blocked_ids.contains(user_id)
    }

    pub fn load_blocked_users(&mut self, cx: &mut Context<Self>) {
        if self.blocked_loading {
            return;
        }
        self.blocked_loading = true;
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut all = Vec::new();
                    let mut page = 1;
                    loop {
                        let (list, pages) = api.blocked_users(page, 50)?;
                        let done = list.is_empty() || page >= pages || all.len() >= 500;
                        all.extend(list);
                        if done {
                            break;
                        }
                        page += 1;
                    }
                    Ok::<_, ApiError>(all)
                })
                .await;
            this.update(cx, |this, cx| {
                this.blocked_loading = false;
                if let Ok(list) = result {
                    this.blocked_ids = list.iter().map(|u| u.user_id.clone()).collect();
                    this.blocked_users = list;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn block_user(&mut self, user_id: String, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let uid = user_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.block_user(&uid) })
                .await;
            this.update(cx, |this, cx| match res {
                Ok(()) => {
                    this.blocked_ids.insert(user_id.clone());
                    this.load_blocked_users(cx);
                    cx.emit(StoreEvent::Toast("บล็อกผู้ใช้แล้ว".into()));
                    cx.notify();
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("บล็อกไม่สำเร็จ: {e}").into())),
            })
            .ok();
        })
        .detach();
    }

    pub fn unblock_user(&mut self, user_id: String, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let uid = user_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.unblock_user(&uid) })
                .await;
            this.update(cx, |this, cx| match res {
                Ok(()) => {
                    this.blocked_ids.remove(&user_id);
                    this.blocked_users.retain(|u| u.user_id != user_id);
                    cx.emit(StoreEvent::Toast("ปลดบล็อกผู้ใช้แล้ว".into()));
                    cx.notify();
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("ปลดบล็อกไม่สำเร็จ: {e}").into())),
            })
            .ok();
        })
        .detach();
    }

    // ---- friends --------------------------------------------------------

    /// Loads the following list once (all pages, capped at 1,000 people).
    pub fn load_friends(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.friends_loading || (self.friends_loaded && !force) {
            return;
        }
        self.friends_loading = true;
        cx.notify();
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut all = Vec::new();
                    let mut page = 1;
                    loop {
                        let (list, pages) = api.following(page, 100)?;
                        let done = list.is_empty() || page >= pages || all.len() >= 1000;
                        all.extend(list);
                        if done {
                            break;
                        }
                        page += 1;
                    }
                    Ok::<_, ApiError>(all)
                })
                .await;
            this.update(cx, |this, cx| {
                this.friends_loading = false;
                match result {
                    Ok(mut list) => {
                        list.sort_by_key(|u| u.display_name().to_lowercase());
                        this.friends = list;
                        this.friends_loaded = true;
                    }
                    Err(e) => cx.emit(StoreEvent::Toast(
                        format!("โหลดรายชื่อเพื่อนไม่สำเร็จ: {e}").into(),
                    )),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Searches people by name or username (debounced by the view).
    pub fn search_people(&mut self, query: String, cx: &mut Context<Self>) {
        self.people_epoch += 1;
        let epoch = self.people_epoch;
        self.people_query = query.clone();
        if query.is_empty() {
            self.people.clear();
            self.people_loading = false;
            cx.notify();
            return;
        }
        self.people_loading = true;
        cx.notify();
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { api.search_users(&query) })
                .await;
            this.update(cx, |this, cx| {
                if this.people_epoch != epoch {
                    return;
                }
                this.people_loading = false;
                match result {
                    Ok(list) => {
                        let me = this.user.as_ref().map(|u| u.user_id.clone());
                        this.people = list
                            .into_iter()
                            .filter(|u| Some(&u.user_id) != me.as_ref())
                            .collect();
                    }
                    Err(e) => {
                        this.people.clear();
                        cx.emit(StoreEvent::Toast(format!("ค้นหาไม่สำเร็จ: {e}").into()));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Opens the one-to-one chat with a friend, creating it on the server
    /// only when no such chat is known locally.
    pub fn open_direct(&mut self, friend: &UserProfile, cx: &mut Context<Self>) {
        if let Some(id) = self
            .chats
            .values()
            .find(|c| c.is_direct_with(&friend.user_id))
            .map(|c| c.chat_id.clone())
        {
            self.select_chat(Some(id), cx);
            return;
        }
        if self.opening_direct {
            return;
        }
        self.opening_direct = true;
        let api = self.api.clone();
        let user_id = friend.user_id.clone();
        let fallback_name = friend.display_name().to_string();
        let fallback_avatar = friend.profile_image.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { api.create_direct_chat(&user_id) })
                .await;
            this.update(cx, |this, cx| {
                this.opening_direct = false;
                match result {
                    Ok(mut chat) => {
                        chat.name.get_or_insert(fallback_name);
                        if chat.avatar_url.is_none() {
                            chat.avatar_url = fallback_avatar;
                        }
                        if let Some(db) = this.db() {
                            let _ = db.upsert_chats(std::slice::from_ref(&chat));
                        }
                        let id = chat.chat_id.clone();
                        this.chats.insert(id.clone(), chat);
                        this.select_chat(Some(id), cx);
                    }
                    Err(e) => cx.emit(StoreEvent::Toast(format!("เปิดแชทไม่สำเร็จ: {e}").into())),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ---- local-only chat features --------------------------------------
    // Mute, pin and export live in SQLite on this device; nothing is sent
    // to the server, so they do not follow the account to other devices.

    pub fn toggle_mute(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.get_mut(chat_id) else {
            return;
        };
        chat.is_muted = !chat.is_muted;
        let muted = chat.is_muted;
        if let Some(db) = self.db() {
            let _ = db.set_chat_muted(chat_id, muted);
        }
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.background_spawn(async move {
            if let Err(e) = api.set_chat_mute(&id, muted, 0) {
                log::warn!("set chat mute: {e}");
            }
        })
        .detach();
        cx.emit(StoreEvent::Toast(
            if muted {
                "ปิดเสียงแชทนี้แล้ว"
            } else {
                "เปิดเสียงแชทนี้แล้ว"
            }
            .into(),
        ));
        cx.notify();
    }

    /// `(count, comma-joined chat names, total unread)` for the Telegram-style Archived row.
    pub fn archived_summary(&self) -> (usize, String, i64) {
        let mut archived: Vec<&Chat> = self
            .chats
            .values()
            .filter(|c| !c.is_deleted && c.is_archived)
            .collect();
        archived.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
        let count = archived.len();
        let unread: i64 = archived.iter().map(|c| c.unread_count).sum();
        let names = archived
            .iter()
            .map(|c| c.display_name())
            .collect::<Vec<_>>()
            .join(", ");
        (count, names, unread)
    }

    pub fn set_show_archived(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.show_archived != show {
            self.show_archived = show;
            cx.notify();
        }
    }

    pub fn toggle_pin_chat(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.get_mut(chat_id) else {
            return;
        };
        chat.is_pinned = !chat.is_pinned;
        let pinned = chat.is_pinned;
        if let Some(db) = self.db() {
            let _ = db.set_chat_pinned(chat_id, pinned);
        }
        cx.emit(StoreEvent::Toast(
            if pinned {
                "ปักหมุดแชทแล้ว"
            } else {
                "เลิกปักหมุดแชทแล้ว"
            }
            .into(),
        ));
        cx.notify();
    }

    pub fn toggle_archive_chat(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let Some(chat) = self.chats.get_mut(chat_id) else {
            return;
        };
        chat.is_archived = !chat.is_archived;
        let archived = chat.is_archived;
        if let Some(db) = self.db() {
            let _ = db.set_chat_archived(chat_id, archived);
        }
        if archived && self.selected_chat.as_deref() == Some(chat_id) {
            self.selected_chat = None;
        }
        cx.emit(StoreEvent::Toast(
            if archived {
                "เก็บแชทถาวรแล้ว"
            } else {
                "ย้ายออกจากที่เก็บถาวรแล้ว"
            }
            .into(),
        ));
        cx.notify();
    }

    pub fn toggle_unread_chat(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        let unread = self.chats.get(chat_id).map(|c| c.unread_count).unwrap_or(0);
        if unread > 0 {
            self.mark_read(chat_id, cx);
        } else {
            if let Some(chat) = self.chats.get_mut(chat_id) {
                chat.unread_count = 1;
            }
            if let Some(db) = self.db() {
                let _ = db.set_unread(chat_id, 1);
            }
            cx.notify();
        }
    }

    pub fn clear_chat_history(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        self.messages.remove(chat_id);
        self.chat_has_more.insert(chat_id.to_string(), false);
        if self.selected_chat.as_deref() == Some(chat_id) {
            self.messages_has_more = false;
        }
        if let Some(chat) = self.chats.get_mut(chat_id) {
            chat.last_message_preview = None;
            chat.last_message_id = None;
        }
        if let Some(db) = self.db() {
            let _ = db.clear_chat_history(chat_id);
        }
        cx.emit(StoreEvent::Toast("ล้างประวัติแชทแล้ว".into()));
        cx.notify();
    }

    pub fn delete_chat(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        self.chats.remove(chat_id);
        self.messages.remove(chat_id);
        self.prefetched_chats.remove(chat_id);
        if self.selected_chat.as_deref() == Some(chat_id) {
            self.selected_chat = None;
        }
        if let Some(db) = self.db() {
            let _ = db.delete_chat(chat_id);
        }
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.delete_chat(&id) })
                .await;
            let _ = this.update(cx, |_, cx| match res {
                Ok(()) => cx.emit(StoreEvent::Toast("ลบแชทแล้ว".into())),
                Err(e) => cx.emit(StoreEvent::Toast(format!("ลบแชทไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
        cx.notify();
    }

    pub fn my_group_role(&self, chat: &Chat) -> String {
        let Some(me) = self.user.as_ref().map(|u| u.user_id.as_str()) else {
            return "member".into();
        };
        chat.participants()
            .into_iter()
            .find(|p| p.user_id == me)
            .map(|p| p.role)
            .unwrap_or_else(|| "member".into())
    }

    fn apply_updated_chat(&mut self, chat: Chat, cx: &mut Context<Self>) {
        if let Some(db) = self.db() {
            let _ = db.upsert_chats(std::slice::from_ref(&chat));
        }
        self.chats.insert(chat.chat_id.clone(), chat);
        cx.notify();
    }

    pub fn rename_group(&mut self, chat_id: String, name: String, cx: &mut Context<Self>) {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() || trimmed.chars().count() > 100 {
            cx.emit(StoreEvent::Toast("ชื่อกลุ่มต้องมีความยาว 1–100 ตัวอักษร".into()));
            return;
        }
        let api = self.api.clone();
        let id = chat_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.rename_group(&id, &trimmed) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(chat) => {
                    this.apply_updated_chat(chat, cx);
                    cx.emit(StoreEvent::Toast("เปลี่ยนชื่อกลุ่มแล้ว".into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("เปลี่ยนชื่อกลุ่มไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn update_group_avatar(
        &mut self,
        chat_id: String,
        file: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        let api = self.api.clone();
        let id = chat_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.update_group_avatar(&id, &file) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(chat) => {
                    this.apply_updated_chat(chat, cx);
                    cx.emit(StoreEvent::Toast("อัปเดตรูปกลุ่มแล้ว".into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("อัปเดตรูปกลุ่มไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn copy_group_invite(&mut self, chat_id: String, cx: &mut Context<Self>) {
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.get_group_invite(&chat_id) })
                .await;
            let _ = this.update(cx, |_, cx| match res {
                Ok(url) => {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(url));
                    cx.emit(StoreEvent::Toast("คัดลอกลิงก์เชิญกลุ่มแล้ว".into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("สร้างลิงก์เชิญไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn add_group_participants(
        &mut self,
        chat_id: String,
        user_ids: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if user_ids.is_empty() {
            return;
        }
        let api = self.api.clone();
        let id = chat_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.add_group_participants(&id, &user_ids) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(()) => {
                    this.load_chat_detail(&chat_id, cx);
                    cx.emit(StoreEvent::Toast("เพิ่มสมาชิกเข้ากลุ่มแล้ว".into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("เพิ่มสมาชิกไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn remove_group_participant(
        &mut self,
        chat_id: String,
        user_id: String,
        is_self: bool,
        cx: &mut Context<Self>,
    ) {
        let api = self.api.clone();
        let id = chat_id.clone();
        let uid = user_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.remove_group_participant(&id, &uid) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(()) => {
                    if is_self {
                        this.chats.remove(&chat_id);
                        this.messages.remove(&chat_id);
                        if this.selected_chat.as_deref() == Some(chat_id.as_str()) {
                            this.selected_chat = None;
                        }
                        if let Some(db) = this.db() {
                            let _ = db.delete_chat(&chat_id);
                        }
                        cx.emit(StoreEvent::Toast("ออกจากกลุ่มแล้ว".into()));
                        cx.notify();
                    } else {
                        this.load_chat_detail(&chat_id, cx);
                        cx.emit(StoreEvent::Toast("นำสมาชิกออกจากกลุ่มแล้ว".into()));
                    }
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("ดำเนินการไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn update_participant_role(
        &mut self,
        chat_id: String,
        user_id: String,
        role: String,
        cx: &mut Context<Self>,
    ) {
        let api = self.api.clone();
        let id = chat_id.clone();
        let uid = user_id.clone();
        let r = role.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.update_participant_role(&id, &uid, &r) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(()) => {
                    this.load_chat_detail(&chat_id, cx);
                    let msg = if role == "admin" {
                        "ตั้งเป็นแอดมินแล้ว"
                    } else {
                        "ลดสิทธิ์เป็นสมาชิกแล้ว"
                    };
                    cx.emit(StoreEvent::Toast(msg.into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("เปลี่ยนสิทธิ์ไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn transfer_group_ownership(
        &mut self,
        chat_id: String,
        new_owner_id: String,
        cx: &mut Context<Self>,
    ) {
        let api = self.api.clone();
        let id = chat_id.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_spawn(async move { api.transfer_group_ownership(&id, &new_owner_id) })
                .await;
            let _ = this.update(cx, |this, cx| match res {
                Ok(()) => {
                    this.load_chat_detail(&chat_id, cx);
                    cx.emit(StoreEvent::Toast("โอนสิทธิ์เจ้าของกลุ่มแล้ว".into()));
                }
                Err(e) => cx.emit(StoreEvent::Toast(format!("โอนสิทธิ์ไม่สำเร็จ: {e}").into())),
            });
        })
        .detach();
    }

    pub fn pinned_message(&self, chat_id: &str) -> Option<Message> {
        let id = self.pinned.get(chat_id)?.clone()?;
        self.messages(chat_id)
            .iter()
            .find(|m| m.message_id == id)
            .cloned()
            .or_else(|| self.db()?.message(&id).ok().flatten())
    }

    fn load_pin(&mut self, chat_id: &str) {
        if !self.pinned.contains_key(chat_id) {
            let id = self
                .db()
                .and_then(|db| db.meta_get(&format!("pin:{chat_id}")).ok().flatten());
            self.pinned.insert(chat_id.to_string(), id);
        }
    }

    pub fn set_pin(&mut self, chat_id: &str, message_id: Option<String>, cx: &mut Context<Self>) {
        if let Some(db) = self.db() {
            let _ = db.meta_set(&format!("pin:{chat_id}"), message_id.as_deref());
        }
        let pinned = message_id.is_some();
        self.pinned.insert(chat_id.to_string(), message_id);
        cx.emit(StoreEvent::Toast(
            if pinned {
                "ปักหมุดข้อความแล้ว"
            } else {
                "เลิกปักหมุดแล้ว"
            }
            .into(),
        ));
        cx.notify();
    }

    pub fn is_pinned(&self, chat_id: &str, message_id: &str) -> bool {
        self.pinned.get(chat_id).and_then(|p| p.as_deref()) == Some(message_id)
    }

    /// Everything stored for a chat, for the media panel and export. Falls
    /// back to the loaded rows when there is no database (preview).
    pub fn all_messages(&self, chat_id: &str) -> Vec<Message> {
        match self.db().and_then(|db| db.all_messages(chat_id).ok()) {
            Some(rows) if !rows.is_empty() => rows,
            _ => self.messages(chat_id).to_vec(),
        }
    }

    /// Plain-text transcript, one line per message.
    pub fn export_text(&self, chat_id: &str) -> String {
        let title = self
            .chats
            .get(chat_id)
            .map(|c| c.display_name().to_string())
            .unwrap_or_default();
        let mut out = format!(
            "แชท: {title}\nส่งออกจาก {} เมื่อ {}\n\n",
            crate::env::APP_NAME,
            chrono::Local::now().format("%d/%m/%Y %H:%M")
        );
        let mut day = String::new();
        for m in self.all_messages(chat_id) {
            let local = crate::models::parse_time(&m.created_at).with_timezone(&chrono::Local);
            let d = local.format("%d/%m/%Y").to_string();
            if d != day {
                out.push_str(&format!("\n— {d} —\n"));
                day = d;
            }
            let body = match m.message_type {
                MessageType::Image => {
                    format!("[รูปภาพ] {}", m.media_url.clone().unwrap_or_default())
                }
                MessageType::File => format!(
                    "[ไฟล์] {} {}",
                    m.file_name.clone().unwrap_or_default(),
                    m.media_url.clone().unwrap_or_default()
                ),
                _ => m.text().to_string(),
            };
            out.push_str(&format!(
                "{} {}: {}\n",
                local.format("%H:%M"),
                self.sender_name(&m),
                body.trim()
            ));
        }
        out
    }

    fn mark_read(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        if let Some(c) = self.chats.get_mut(chat_id) {
            if c.unread_count == 0 {
                return;
            }
            c.unread_count = 0;
        }
        if let Some(db) = self.db() {
            let _ = db.set_unread(chat_id, 0);
            let _ = db.mark_chat_read(chat_id);
        }
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.background_spawn(async move {
            if let Err(e) = api.mark_chat_read(&id) {
                log::warn!("mark read: {e}");
            }
        })
        .detach();
    }

    fn set_typing(&mut self, chat_id: &str, on: bool, cx: &mut Context<Self>) {
        let epoch = self.typing_epoch.entry(chat_id.to_string()).or_default();
        *epoch += 1;
        let current = *epoch;
        if on {
            self.typing.insert(chat_id.to_string());
            let id = chat_id.to_string();
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(TYPING_AUTO_CLEAR_MS))
                    .await;
                this.update(cx, |this, cx| {
                    if this.typing_epoch.get(&id) == Some(&current) {
                        this.typing.remove(&id);
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        } else {
            self.typing.remove(chat_id);
        }
        cx.notify();
    }

    pub fn send_typing(&self, chat_id: &str, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.background_spawn(async move {
            let _ = api.send_typing(&id);
        })
        .detach();
    }

    fn touch_chat(&mut self, chat_id: &str, message_id: &str, preview: &str, at: &str) {
        let preview: String = preview.chars().take(100).collect();
        if let Some(c) = self.chats.get_mut(chat_id) {
            c.last_message_id = Some(message_id.to_string());
            c.last_message_preview = Some(preview.clone());
            c.last_message_at = Some(at.to_string());
        }
        if let Some(db) = self.db() {
            let _ = db.update_chat_last_message(chat_id, message_id, &preview, at);
        }
    }

    // ---- messages -------------------------------------------------------

    pub fn messages(&self, chat_id: &str) -> &[Message] {
        self.messages.get(chat_id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn reply_target(&self, chat_id: &str, message_id: &str) -> Option<ReplyTarget> {
        let m = self
            .messages(chat_id)
            .iter()
            .find(|m| m.message_id == message_id)
            .cloned()
            .or_else(|| self.db()?.message(message_id).ok().flatten())?;
        Some(ReplyTarget {
            message_id: m.message_id.clone(),
            sender_name: self.sender_name(&m),
            content: m.text().to_string(),
        })
    }

    pub fn is_self(&self, m: &Message) -> bool {
        match (&self.user, &m.sender_id) {
            (Some(u), Some(s)) => &u.user_id == s,
            _ => false,
        }
    }

    pub fn sender_name(&self, m: &Message) -> String {
        if self.is_self(m) {
            return "คุณ".into();
        }
        m.sender()
            .name
            .or_else(|| {
                let chat = self.chats.get(&m.chat_id)?;
                (chat.chat_type == ChatType::Direct).then(|| chat.display_name().to_string())
            })
            .unwrap_or_else(|| "ไม่ทราบชื่อ".into())
    }

    /// Preload the top chats' latest messages in the background (SQLite first,
    /// API fallback for uncached top chats) so switching rooms is instant even
    /// on the very first click, matching Telegram Desktop's preload behavior.
    fn prefetch_top_chats(&mut self, cx: &mut Context<Self>) {
        if self.phase != Phase::SignedIn {
            return;
        }
        let todo_items: Vec<(String, Option<String>)> = self
            .sorted_chats()
            .into_iter()
            .take(8)
            .filter(|c| {
                !self.messages.contains_key(&c.chat_id)
                    && !self.prefetched_chats.contains(&c.chat_id)
            })
            .map(|c| (c.chat_id, c.avatar_url))
            .collect();
        if todo_items.is_empty() {
            return;
        }
        let mut todo = Vec::with_capacity(todo_items.len());
        for (id, avatar) in todo_items {
            self.prefetched_chats.insert(id.clone());
            self.load_pin(&id);
            if let Some(url) = avatar {
                let _ = self.image(&url, cx);
            }
            todo.push(id);
        }

        let api = self.api.clone();
        let db_path = crate::env::data_dir().join("pinto-chat.sqlite");
        cx.spawn(async move |this, cx| {
            let mut api_budget = 5usize;
            for chat_id in todo {
                let already_loaded = this
                    .read_with(cx, |this, _| this.messages.contains_key(&chat_id))
                    .unwrap_or(true);
                if already_loaded {
                    continue;
                }
                let id_for_db = chat_id.clone();
                let path = db_path.clone();
                let local = cx
                    .background_spawn(async move {
                        Db::open(&path)
                            .ok()
                            .and_then(|d| d.messages(&id_for_db, MESSAGE_PAGE_SIZE, None).ok())
                            .unwrap_or_default()
                    })
                    .await;

                if !local.is_empty() {
                    let has_more = local.len() == MESSAGE_PAGE_SIZE;
                    let _ = this.update(cx, |this, _| {
                        if !this.messages.contains_key(&chat_id) {
                            this.chat_has_more.insert(chat_id.clone(), has_more);
                            this.messages
                                .insert(chat_id, local.into_iter().rev().collect());
                        }
                    });
                    continue;
                }

                if api_budget == 0 {
                    continue;
                }
                api_budget -= 1;

                let api = api.clone();
                let fetch_id = chat_id.clone();
                let fetched = cx
                    .background_spawn(
                        async move { api.legacy_history(&fetch_id, MESSAGE_PAGE_SIZE) },
                    )
                    .await;
                if let Ok(rows) = fetched {
                    let mut list: Vec<Message> = rows
                        .iter()
                        .map(|raw| {
                            let mut m = Message::from_value(raw);
                            if m.chat_id.is_empty() {
                                m.chat_id = chat_id.clone();
                            }
                            m
                        })
                        .collect();
                    list.sort_by_key(|m| crate::models::parse_time(&m.created_at));
                    let has_more = list.len() == MESSAGE_PAGE_SIZE;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(db) = this.db() {
                            for m in &list {
                                let _ = db.upsert_message(m);
                            }
                        }
                        if !this.messages.contains_key(&chat_id) {
                            this.chat_has_more.insert(chat_id.clone(), has_more);
                            this.messages.insert(chat_id.clone(), list);
                            if this.selected_chat.as_deref() == Some(chat_id.as_str()) {
                                cx.emit(StoreEvent::MessagesAppended);
                            }
                            cx.notify();
                        }
                    });
                }
            }
        })
        .detach();
    }

    /// Local rows first; only an empty cache goes to the API.
    fn load_messages(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        if self
            .messages
            .get(chat_id)
            .is_some_and(|list| !list.is_empty())
        {
            self.messages_loading = false;
            self.messages_has_more = self.chat_has_more.get(chat_id).copied().unwrap_or(true);
            cx.notify();
            return;
        }
        self.messages_has_more = true;
        let local = self
            .db()
            .and_then(|db| db.messages(chat_id, MESSAGE_PAGE_SIZE, None).ok())
            .unwrap_or_default();
        if !local.is_empty() {
            let has_more = local.len() == MESSAGE_PAGE_SIZE;
            self.chat_has_more.insert(chat_id.to_string(), has_more);
            self.messages_has_more = has_more;
            self.messages
                .insert(chat_id.to_string(), local.into_iter().rev().collect());
            cx.emit(StoreEvent::MessagesAppended);
            cx.notify();
            return;
        }
        self.messages_loading = true;
        cx.notify();
        let api = self.api.clone();
        let id = chat_id.to_string();
        cx.spawn(async move |this, cx| {
            let fetch_id = id.clone();
            let result = cx
                .background_spawn(async move { api.legacy_history(&fetch_id, MESSAGE_PAGE_SIZE) })
                .await;
            this.update(cx, |this, cx| {
                this.messages_loading = false;
                match result {
                    Ok(rows) => {
                        let mut list: Vec<Message> = rows
                            .iter()
                            .map(|raw| {
                                let mut m = Message::from_value(raw);
                                if m.chat_id.is_empty() {
                                    m.chat_id = id.clone();
                                }
                                m
                            })
                            .collect();
                        if let Some(db) = this.db() {
                            for m in &list {
                                let _ = db.upsert_message(m);
                            }
                        }
                        let has_more = list.len() == MESSAGE_PAGE_SIZE;
                        this.chat_has_more.insert(id.clone(), has_more);
                        this.messages_has_more = has_more;
                        list.sort_by_key(|m| crate::models::parse_time(&m.created_at));
                        this.messages.insert(id.clone(), list);
                        cx.emit(StoreEvent::MessagesAppended);
                    }
                    Err(e) => cx.emit(StoreEvent::Toast(format!("โหลดข้อความไม่สำเร็จ: {e}").into())),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn load_older(&mut self, chat_id: &str, cx: &mut Context<Self>) {
        if !self.messages_has_more || self.messages_loading_more {
            return;
        }
        let Some(oldest) = self.messages(chat_id).first().map(|m| m.created_at.clone()) else {
            return;
        };
        let older = self
            .db()
            .and_then(|db| db.messages(chat_id, MESSAGE_PAGE_SIZE, Some(&oldest)).ok())
            .unwrap_or_default();
        let has_more = older.len() == MESSAGE_PAGE_SIZE;
        self.chat_has_more.insert(chat_id.to_string(), has_more);
        self.messages_has_more = has_more;
        if !older.is_empty() {
            let list = self.messages.entry(chat_id.to_string()).or_default();
            let mut merged: Vec<Message> = older.into_iter().rev().collect();
            merged.append(list);
            *list = merged;
        }
        cx.notify();
    }

    fn push_message(&mut self, m: Message) -> bool {
        let list = self.messages.entry(m.chat_id.clone()).or_default();
        if list.iter().any(|x| x.message_id == m.message_id) {
            return false;
        }
        // A WS echo of our own optimistic send: adopt the server row in place.
        if let Some(temp) = m.client_temp_id.as_deref()
            && let Some(x) = list
                .iter_mut()
                .find(|x| x.client_temp_id.as_deref() == Some(temp))
        {
            *x = Message {
                status: MessageStatus::Sent,
                ..m
            };
            return false;
        }
        list.push(m);
        true
    }

    fn replace_message(&mut self, chat_id: &str, temp_id: &str, f: impl FnOnce(&mut Message)) {
        if let Some(list) = self.messages.get_mut(chat_id)
            && let Some(m) = list
                .iter_mut()
                .find(|m| m.message_id == temp_id || m.client_temp_id.as_deref() == Some(temp_id))
        {
            f(m);
        }
    }

    pub fn send_text(
        &mut self,
        chat_id: &str,
        content: String,
        reply_to: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(user) = self.user.clone() else {
            return;
        };
        let temp = Message::temp(
            chat_id,
            &user.user_id,
            &content,
            MessageType::Text,
            reply_to.clone(),
        );
        let outbox = OutboxItem {
            id: new_id(),
            chat_id: chat_id.to_string(),
            client_temp_id: temp.message_id.clone(),
            content: Some(content.clone()),
            message_type: MessageType::Text,
            reply_to: reply_to.clone(),
            retry_count: 0,
            created_at: crate::models::now_iso(),
        };
        if let Some(db) = self.db() {
            let _ = db.outbox_insert(&outbox);
            let _ = db.upsert_message(&temp);
        }
        self.push_message(temp.clone());
        cx.emit(StoreEvent::MessagesAppended);
        cx.notify();
        self.dispatch_text(outbox, temp.created_at.clone(), cx);
    }

    fn send_request(api: &Api, item: &OutboxItem) -> Result<Value, ApiError> {
        api.send_text(
            &item.chat_id,
            item.content.as_deref().unwrap_or(""),
            item.reply_to.as_deref(),
            &item.client_temp_id,
        )
    }

    fn dispatch_text(&mut self, item: OutboxItem, created_at: String, cx: &mut Context<Self>) {
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            let req = item.clone();
            let result = cx
                .background_spawn(async move { Self::send_request(&api, &req) })
                .await;
            this.update(cx, |this, cx| {
                this.apply_send_result(&item, created_at, result, cx)
            })
            .ok();
        })
        .detach();
    }

    fn apply_send_result(
        &mut self,
        item: &OutboxItem,
        created_at: String,
        result: Result<Value, ApiError>,
        cx: &mut Context<Self>,
    ) {
        let temp_id = item.client_temp_id.clone();
        match result {
            Ok(raw) => {
                let server = Message::from_value(&raw);
                if let Some(db) = self.db() {
                    let _ = db.confirm_temp_message(&temp_id, &server.message_id);
                    let _ = db.outbox_remove(&item.id);
                }
                let id = server.message_id.clone();
                // Drop a WS echo that may have landed first, then confirm the temp row.
                if let Some(list) = self.messages.get_mut(&item.chat_id) {
                    list.retain(|m| {
                        !(m.message_id == id && m.client_temp_id.as_deref() != Some(&temp_id))
                    });
                }
                self.replace_message(&item.chat_id, &temp_id, |m| {
                    m.message_id = id.clone();
                    m.status = MessageStatus::Sent;
                    m.sequence_id = server.sequence_id;
                });
                let at = if server.created_at.is_empty() {
                    created_at
                } else {
                    server.created_at.clone()
                };
                self.touch_chat(
                    &item.chat_id,
                    &id,
                    item.content.as_deref().unwrap_or(""),
                    &at,
                );
            }
            Err(e) => {
                log::warn!(
                    "send to {} failed ({:?}): {}",
                    item.chat_id,
                    e.status,
                    e.message
                );
                if let Some(db) = self.db() {
                    let _ = db.update_message_status(&temp_id, MessageStatus::Failed);
                    let _ = db.outbox_failed(&item.id, &e.message);
                }
                self.replace_message(&item.chat_id, &temp_id, |m| {
                    m.status = MessageStatus::Failed
                });
            }
        }
        cx.notify();
    }

    pub fn retry(&mut self, chat_id: &str, message_id: &str, cx: &mut Context<Self>) {
        let Some(m) = self
            .messages(chat_id)
            .iter()
            .find(|m| m.message_id == message_id)
            .cloned()
        else {
            return;
        };
        let Some(content) = m.content.clone() else {
            return;
        };
        let temp_id = m.client_temp_id.clone().unwrap_or(m.message_id.clone());
        self.replace_message(chat_id, &temp_id, |m| m.status = MessageStatus::Sending);
        if let Some(db) = self.db() {
            let _ = db.update_message_status(&m.message_id, MessageStatus::Sending);
            let _ = db.outbox_remove_by_temp(&temp_id);
        }
        let item = OutboxItem {
            id: new_id(),
            chat_id: chat_id.to_string(),
            client_temp_id: temp_id,
            content: Some(content),
            message_type: MessageType::Text,
            reply_to: m.reply_to.clone(),
            retry_count: 0,
            created_at: crate::models::now_iso(),
        };
        if let Some(db) = self.db() {
            let _ = db.outbox_insert(&item);
        }
        cx.notify();
        self.dispatch_text(item, m.created_at, cx);
    }

    /// Retries queued and failed sends after (re)connecting.
    /// Retries queued sends after sign-in or a reconnect. Only recent items with
    /// few attempts go out automatically, one at a time, and a 429 stops the
    /// run. Anything older keeps its failed state and waits for a manual retry,
    /// so stale messages are never sent long after they were written.
    fn flush_outbox(&mut self, cx: &mut Context<Self>) {
        if self.flushing {
            return;
        }
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(OUTBOX_MAX_AGE_HOURS);
        let pending: Vec<_> = self
            .db()
            .and_then(|db| db.outbox_pending().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|i| i.message_type == MessageType::Text)
            .filter(|i| i.retry_count < OUTBOX_MAX_AUTO_RETRIES)
            .filter(|i| crate::models::parse_time(&i.created_at) > cutoff)
            .collect();
        if pending.is_empty() {
            return;
        }
        self.flushing = true;
        for item in &pending {
            self.replace_message(&item.chat_id, &item.client_temp_id, |m| {
                m.status = MessageStatus::Sending
            });
        }
        cx.notify();
        let api = self.api.clone();
        cx.spawn(async move |this, cx| {
            for item in pending {
                let req = item.clone();
                let api = api.clone();
                let result = cx
                    .background_spawn(async move { Self::send_request(&api, &req) })
                    .await;
                let limited = matches!(
                    &result,
                    Err(ApiError {
                        status: Some(429),
                        ..
                    })
                );
                let created = item.created_at.clone();
                if this
                    .update(cx, |this, cx| {
                        this.apply_send_result(&item, created, result, cx)
                    })
                    .is_err()
                {
                    return;
                }
                if limited {
                    log::warn!("outbox flush stopped: rate limited");
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
            }
            this.update(cx, |this, _| this.flushing = false).ok();
        })
        .detach();
    }

    pub fn send_file(
        &mut self,
        chat_id: &str,
        path: PathBuf,
        reply_to: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(user) = self.user.clone() else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let is_image = matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("png" | "jpg" | "jpeg" | "gif" | "webp")
        );
        let kind = if is_image {
            MessageType::Image
        } else {
            MessageType::File
        };
        let mut temp = Message::temp(chat_id, &user.user_id, "", kind, reply_to.clone());
        temp.content = None;
        temp.file_name = Some(name);
        temp.file_size = std::fs::metadata(&path).ok().map(|m| m.len() as i64);
        if let Some(db) = self.db() {
            let _ = db.upsert_message(&temp);
        }
        let temp_id = temp.message_id.clone();
        self.push_message(temp);
        cx.emit(StoreEvent::MessagesAppended);
        cx.notify();

        let api = self.api.clone();
        let chat = chat_id.to_string();
        cx.spawn(async move |this, cx| {
            let (c, r) = (chat.clone(), reply_to.clone());
            let result = cx
                .background_spawn({
                    let id = temp_id.clone();
                    async move { api.send_file(&c, &path, r.as_deref(), &id) }
                })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(raw) => {
                        let server = Message::from_value(&raw);
                        if let Some(db) = this.db() {
                            let _ = db.confirm_temp_message(&temp_id, &server.message_id);
                        }
                        this.replace_message(&chat, &temp_id, |m| {
                            m.message_id = server.message_id.clone();
                            m.status = MessageStatus::Sent;
                            m.media_url = server.media_url.clone();
                            m.sequence_id = server.sequence_id;
                        });
                    }
                    Err(e) => {
                        if let Some(db) = this.db() {
                            let _ = db.update_message_status(&temp_id, MessageStatus::Failed);
                        }
                        this.replace_message(&chat, &temp_id, |m| m.status = MessageStatus::Failed);
                        cx.emit(StoreEvent::Toast(format!("ส่งไฟล์ไม่สำเร็จ: {e}").into()));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Hides the message on this device only, as the Vue build does.
    pub fn delete_message(&mut self, chat_id: &str, message_id: &str, cx: &mut Context<Self>) {
        if let Some(db) = self.db() {
            let _ = db.mark_message_deleted(message_id);
        }
        if let Some(list) = self.messages.get_mut(chat_id) {
            list.retain(|m| m.message_id != message_id);
        }
        cx.emit(StoreEvent::Toast("ลบข้อความแล้ว".into()));
        cx.notify();
    }

    /// Fetches a missing `sequence_id` range (500 per request), stores it and
    /// merges it into the open list in time order.
    fn repair(&mut self, chat_id: &str, from: i64, to: i64, cx: &mut Context<Self>) {
        log::info!("repair {chat_id} sequence {from}..={to}");
        let api = self.api.clone();
        let chat = chat_id.to_string();
        cx.spawn(async move |this, cx| {
            let id = chat.clone();
            let result = cx
                .background_spawn(async move {
                    let mut all = Vec::new();
                    let mut start = from;
                    while start <= to {
                        let end = (start + 499).min(to);
                        all.extend(api.repair(&id, start, end)?);
                        start = end + 1;
                    }
                    Ok::<_, ApiError>(all)
                })
                .await;
            this.update(cx, |this, cx| match result {
                Ok(rows) => {
                    let mut added = false;
                    for raw in &rows {
                        let mut m = Message::from_value(raw);
                        if m.chat_id.is_empty() {
                            m.chat_id = chat.clone();
                        }
                        if let Some(db) = this.db() {
                            let _ = db.upsert_message(&m);
                        }
                        added |= this.push_message(m);
                    }
                    if added {
                        if let Some(list) = this.messages.get_mut(&chat) {
                            list.sort_by_key(|m| {
                                (crate::models::parse_time(&m.created_at), m.sequence_id)
                            });
                        }
                        cx.emit(StoreEvent::MessagesAppended);
                        cx.notify();
                    }
                }
                Err(e) => log::warn!("repair {chat}: {e}"),
            })
            .ok();
        })
        .detach();
    }

    // ---- realtime -------------------------------------------------------

    fn connect_socket(&mut self, cx: &mut Context<Self>) {
        if StoredSession::load().access_token.is_none() {
            return;
        }
        let env = self.env.clone();
        // Read the token on every (re)connect: the API client may have refreshed it.
        let url = move || {
            StoredSession::load()
                .access_token
                .map(|t| env.chat_ws_url(&t))
        };
        let (handle, mut rx) = realtime::spawn_chat_socket(url);
        self.socket = Some(handle);
        cx.spawn(async move |this, cx| {
            while let Some(event) = rx.next().await {
                if this
                    .update(cx, |this, cx| this.on_chat_event(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn on_chat_event(&mut self, event: ChatEvent, cx: &mut Context<Self>) {
        match event {
            ChatEvent::State(state) => {
                let reconnected = state == ConnectionState::Connected
                    && self.connection != ConnectionState::Connected;
                self.connection = state;
                if reconnected {
                    self.flush_outbox(cx);
                }
                cx.notify();
            }
            ChatEvent::Frame { kind, data } => self.on_frame(&kind, &data, cx),
        }
    }

    fn on_frame(&mut self, kind: &str, data: &Value, cx: &mut Context<Self>) {
        let chat_id = data
            .get("chat_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        match kind {
            "new_message" => {
                let msg = Message::from_value(data);
                if msg.chat_id.is_empty() {
                    return;
                }
                // A jump in sequence_id means messages were missed: fetch the gap.
                if let (Some(seq), Some(last)) = (
                    msg.sequence_id,
                    self.db()
                        .and_then(|db| db.max_sequence(&msg.chat_id).ok().flatten()),
                ) && seq > last + 1
                {
                    self.repair(&msg.chat_id, last + 1, seq - 1, cx);
                }
                if let Some(db) = self.db() {
                    let _ = db.upsert_message(&msg);
                }
                let own = self.is_self(&msg);
                let open = self.selected_chat.as_deref() == Some(&msg.chat_id);
                self.touch_chat(&msg.chat_id, &msg.message_id, msg.text(), &msg.created_at);
                self.set_typing(&msg.chat_id, false, cx);
                let added = self.push_message(msg.clone());
                if open {
                    if added {
                        cx.emit(StoreEvent::MessagesAppended);
                    }
                    self.mark_read(&msg.chat_id, cx);
                } else if !own {
                    let unread = self
                        .chats
                        .get(&msg.chat_id)
                        .map(|c| c.unread_count + 1)
                        .unwrap_or(1);
                    if let Some(c) = self.chats.get_mut(&msg.chat_id) {
                        c.unread_count = unread;
                    }
                    if let Some(db) = self.db() {
                        let _ = db.set_unread(&msg.chat_id, unread);
                    }
                    if !self.chats.contains_key(&msg.chat_id) {
                        self.load_chats(true, cx);
                    }
                    let muted = self.chats.get(&msg.chat_id).is_some_and(|c| c.is_muted);
                    if self.settings.notifications_enabled && !muted {
                        let title = self
                            .chats
                            .get(&msg.chat_id)
                            .map(|c| c.display_name().to_string())
                            .unwrap_or_else(|| "ข้อความใหม่".into());
                        let body = if self.settings.show_preview {
                            msg.content.clone().unwrap_or_else(|| "ส่งไฟล์".into())
                        } else {
                            "ข้อความใหม่".into()
                        };
                        cx.emit(StoreEvent::Notify { title, body });
                    }
                }
                cx.notify();
            }
            "typing" => {
                let on = data
                    .get("is_typing")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if !chat_id.is_empty() {
                    self.set_typing(&chat_id, on, cx);
                }
            }
            "stop_typing" => {
                if !chat_id.is_empty() {
                    self.set_typing(&chat_id, false, cx);
                }
            }
            "read_receipt" => {
                if let Some(id) = data.get("message_id").and_then(Value::as_str) {
                    if let Some(db) = self.db() {
                        let _ = db.mark_message_read(id);
                        let _ = db.update_message_status(id, MessageStatus::Read);
                    }
                    for list in self.messages.values_mut() {
                        if let Some(m) = list.iter_mut().find(|m| m.message_id == id) {
                            m.is_read = true;
                            m.status = MessageStatus::Read;
                        }
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    // ---- images ---------------------------------------------------------

    /// Avatars and image messages. Returns `None` until the bytes arrive.
    pub fn image(&mut self, url: &str, cx: &mut Context<Self>) -> Option<Arc<Image>> {
        match self.images.get(url) {
            Some(RemoteImage::Ready(img)) => return Some(img.clone()),
            Some(_) => return None,
            None => {}
        }
        self.images.insert(url.to_string(), RemoteImage::Loading);
        let api = self.api.clone();
        let key = url.to_string();
        let cache_dir = crate::env::data_dir();
        cx.spawn(async move |this, cx| {
            let url = key.clone();
            let result = cx
                .background_spawn(async move {
                    if let Some(cached) = crate::cache::read_cached_media(&cache_dir, &url) {
                        return Ok(cached);
                    }
                    let bytes = api.fetch_bytes(&url)?;
                    crate::cache::write_cached_media(&cache_dir, &url, &bytes);
                    Ok::<_, crate::api::ApiError>(bytes)
                })
                .await;
            this.update(cx, |this, cx| {
                let state = match result.ok().and_then(|bytes| Some((sniff(&bytes)?, bytes))) {
                    Some((format, bytes)) => {
                        RemoteImage::Ready(Arc::new(Image::from_bytes(format, bytes)))
                    }
                    None => RemoteImage::Failed,
                };
                this.images.insert(key, state);
                cx.notify();
            })
            .ok();
        })
        .detach();
        None
    }
}

fn sniff(b: &[u8]) -> Option<ImageFormat> {
    if b.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(ImageFormat::Png)
    } else if b.starts_with(&[0xFF, 0xD8]) {
        Some(ImageFormat::Jpeg)
    } else if b.starts_with(b"GIF8") {
        Some(ImageFormat::Gif)
    } else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some(ImageFormat::Webp)
    } else if b.starts_with(b"<svg") || b.starts_with(b"<?xml") {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageFormat, sniff};

    #[test]
    fn sniffs_supported_image_formats() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), Some(ImageFormat::Png));
        assert_eq!(sniff(b"\xFF\xD8\xFF\xE0"), Some(ImageFormat::Jpeg));
        assert_eq!(sniff(b"GIF89a..."), Some(ImageFormat::Gif));
        assert_eq!(
            sniff(b"RIFF\x00\x00\x00\x00WEBPextra"),
            Some(ImageFormat::Webp)
        );
        assert_eq!(sniff(b"<svg xmlns='x'>"), Some(ImageFormat::Svg));
        assert_eq!(sniff(b"not-an-image"), None);
    }
}
