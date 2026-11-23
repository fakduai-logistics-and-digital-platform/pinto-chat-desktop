//! Blocking HTTP client for the Pinto API (`/docs/swagger.yaml`). Call it from
//! a background task, never the UI thread.
//!
//! Every request carries the device headers the spec asks for, so login and
//! refresh return a refresh token bound to this install. A 401 triggers one
//! token refresh and a single retry.

use std::{
    io::Read,
    path::Path,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use serde_json::{Value, json};
use ureq::Agent;

use crate::{
    models::{Chat, ChatType, UserProfile},
    persist::{StoredSession, device_id},
};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: Option<u16>,
    pub message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

fn network(e: impl std::fmt::Display) -> ApiError {
    ApiError {
        status: None,
        message: format!("เชื่อมต่อเซิร์ฟเวอร์ไม่ได้ ({e})"),
    }
}

fn invalid(message: &str) -> ApiError {
    ApiError {
        status: None,
        message: message.into(),
    }
}

pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

pub struct ChatPage {
    pub chats: Vec<Chat>,
}

enum Body {
    Empty,
    Json(Value),
    Multipart { boundary: String, bytes: Vec<u8> },
}

/// `multipart/form-data` encoder for the message endpoints.
struct Form {
    boundary: String,
    bytes: Vec<u8>,
}

impl Form {
    fn new() -> Self {
        Self {
            boundary: format!("----pinto{}", uuid::Uuid::new_v4().simple()),
            bytes: Vec::new(),
        }
    }

    fn text(mut self, name: &str, value: &str) -> Self {
        self.bytes.extend_from_slice(
            format!(
                "--{}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n",
                self.boundary
            )
            .as_bytes(),
        );
        self
    }

    fn opt(self, name: &str, value: Option<&str>) -> Self {
        match value {
            Some(v) => self.text(name, v),
            None => self,
        }
    }

    fn file(mut self, name: &str, file_name: &str, mime: &str, data: &[u8]) -> Self {
        let safe = file_name.replace('"', "'");
        self.bytes.extend_from_slice(
            format!(
                "--{}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{safe}\"\r\nContent-Type: {mime}\r\n\r\n",
                self.boundary
            )
            .as_bytes(),
        );
        self.bytes.extend_from_slice(data);
        self.bytes.extend_from_slice(b"\r\n");
        self
    }

    fn finish(mut self) -> Body {
        self.bytes
            .extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());
        Body::Multipart {
            boundary: self.boundary,
            bytes: self.bytes,
        }
    }
}

#[derive(Clone)]
pub struct Api {
    base: String,
    agent: Agent,
    access: Arc<RwLock<Option<String>>>,
    refresh: Arc<RwLock<Option<String>>>,
    /// One refresh at a time; concurrent 401s wait and reuse the new token.
    refresh_lock: Arc<Mutex<()>>,
    device_id: String,
}

const USER_AGENT: &str = concat!("PintoChatDesktop/", env!("CARGO_PKG_VERSION"));
const DEVICE_NAME: &str = "Pinto Chat Desktop";
/// The spec allows `ios`, `android` or `web`; the desktop app is the web client.
const DEVICE_PLATFORM: &str = "web";

impl Api {
    pub fn new(base: impl Into<String>) -> Self {
        let agent: Agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(USER_AGENT)
            .build()
            .into();
        Self {
            base: base.into(),
            agent,
            access: Arc::new(RwLock::new(None)),
            refresh: Arc::new(RwLock::new(None)),
            refresh_lock: Arc::new(Mutex::new(())),
            device_id: device_id(),
        }
    }

    pub fn set_tokens(&self, access: Option<String>, refresh: Option<String>) {
        *self.access.write().unwrap() = access;
        *self.refresh.write().unwrap() = refresh;
    }

    /// A copy with its own token cells, unaffected by later `set_tokens`.
    pub fn detached(&self) -> Self {
        let mut copy = self.clone();
        copy.access = Arc::new(RwLock::new(self.access.read().unwrap().clone()));
        copy.refresh = Arc::new(RwLock::new(self.refresh.read().unwrap().clone()));
        copy.refresh_lock = Arc::new(Mutex::new(()));
        copy
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// Sends one request with the auth and device headers.
    fn call(
        &self,
        method: &str,
        url: &str,
        body: &Body,
    ) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
        let bearer = self
            .access
            .read()
            .unwrap()
            .as_ref()
            .map(|t| format!("Bearer {t}"));
        macro_rules! headers {
            ($req:expr) => {{
                let mut req = $req
                    .header("X-Device-ID", &self.device_id)
                    .header("X-Device-Name", DEVICE_NAME)
                    .header("X-Device-Platform", DEVICE_PLATFORM);
                if let Some(b) = &bearer {
                    req = req.header("Authorization", b);
                }
                req
            }};
        }
        macro_rules! with_body {
            ($req:expr) => {{
                let req = headers!($req);
                match body {
                    Body::Empty => req.header("Content-Type", "application/json").send_empty(),
                    Body::Json(v) => req.send_json(v),
                    Body::Multipart { boundary, bytes } => req
                        .header(
                            "Content-Type",
                            &format!("multipart/form-data; boundary={boundary}"),
                        )
                        .send(&bytes[..]),
                }
            }};
        }
        match method {
            "GET" => headers!(self.agent.get(url)).call(),
            "DELETE" => headers!(self.agent.delete(url)).call(),
            "PUT" => with_body!(self.agent.put(url)),
            _ => with_body!(self.agent.post(url)),
        }
    }

    /// Unwraps `{ ok, data, error }` envelopes the same way the TS client did.
    fn finish(resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> ApiResult<Value> {
        let mut resp = resp.map_err(network)?;
        let status = resp.status().as_u16();
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        let payload: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let err_text = || {
            payload
                .get("error")
                .and_then(|e| {
                    e.as_str()
                        .map(str::to_string)
                        .or_else(|| e.get("message")?.as_str().map(str::to_string))
                })
                .or_else(|| {
                    payload
                        .get("message")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
        };
        if !(200..300).contains(&status) {
            return Err(ApiError {
                status: Some(status),
                message: err_text().unwrap_or_else(|| format!("HTTP {status}")),
            });
        }
        if payload.get("ok") == Some(&Value::Bool(false)) {
            return Err(ApiError {
                status: Some(status),
                message: err_text().unwrap_or_else(|| "เกิดข้อผิดพลาดจากเซิร์ฟเวอร์".into()),
            });
        }
        Ok(match payload.get("data") {
            Some(d) if !d.is_null() => d.clone(),
            _ => payload,
        })
    }

    /// Request with one refresh-and-retry on 401.
    fn request(&self, method: &str, path: &str, body: Body) -> ApiResult<Value> {
        let url = self.url(path);
        let sent_with = self.access.read().unwrap().clone();
        let first = Self::finish(self.call(method, &url, &body));
        match first {
            Err(ApiError {
                status: Some(401), ..
            }) if self.refresh_after(sent_with.as_deref()) => {
                Self::finish(self.call(method, &url, &body))
            }
            other => other,
        }
    }

    /// Refreshes unless another thread already did since `stale` was sent.
    /// Returns whether a usable new access token is in place.
    fn refresh_after(&self, stale: Option<&str>) -> bool {
        let _guard = self.refresh_lock.lock().unwrap();
        if self.access.read().unwrap().as_deref() != stale {
            return true;
        }
        let Some(refresh) = self.refresh.read().unwrap().clone() else {
            return false;
        };
        let url = self.url("/api/auth/refresh");
        let saved_access = self.access.write().unwrap().take();
        let result = Self::finish(self.call(
            "POST",
            &url,
            &Body::Json(json!({ "refresh_token": refresh })),
        ))
        .ok()
        .and_then(|v| tokens_from(&v));
        match result {
            Some(t) => {
                let refresh = t.refresh_token.or(Some(refresh));
                StoredSession {
                    access_token: Some(t.access_token.clone()),
                    refresh_token: refresh.clone(),
                }
                .save();
                self.set_tokens(Some(t.access_token), refresh);
                log::info!("access token refreshed");
                true
            }
            None => {
                *self.access.write().unwrap() = saved_access;
                log::warn!("token refresh failed");
                false
            }
        }
    }

    // ---- auth -----------------------------------------------------------

    pub fn login(&self, identifier: &str, password: &str) -> ApiResult<AuthTokens> {
        let v = self.request(
            "POST",
            "/api/auth/login",
            Body::Json(json!({ "email": identifier, "password": password })),
        )?;
        tokens_from(&v).ok_or_else(|| invalid("เซิร์ฟเวอร์ไม่ได้ส่งโทเคนกลับมา"))
    }

    /// Revokes the refresh token too, as the spec allows.
    pub fn logout(&self) -> ApiResult<()> {
        let refresh = self.refresh.read().unwrap().clone();
        let body = match refresh {
            Some(r) => Body::Json(json!({ "refresh_token": r })),
            None => Body::Empty,
        };
        Self::finish(self.call("POST", &self.url("/api/auth/logout"), &body)).map(|_| ())
    }

    pub fn profile(&self) -> ApiResult<UserProfile> {
        let v = self.request("GET", "/v1/users/profile", Body::Empty)?;
        UserProfile::from_value(&v).ok_or_else(|| invalid("อ่านข้อมูลโปรไฟล์ไม่ได้"))
    }

    // ---- chats ----------------------------------------------------------

    pub fn chats(
        &self,
        page: usize,
        limit: usize,
        q: Option<&str>,
        kind: Option<ChatType>,
    ) -> ApiResult<ChatPage> {
        let mut qs = vec![format!("page={page}"), format!("limit={limit}")];
        if let Some(k) = kind {
            qs.push(format!("chat_type={}", k.as_str()));
        }
        if let Some(q) = q.filter(|q| !q.is_empty()) {
            qs.push(format!("q={}", urlencoding::encode(q)));
        }
        let v = self.request("GET", &format!("/v1/chats?{}", qs.join("&")), Body::Empty)?;
        let list = v
            .get("chats")
            .and_then(Value::as_array)
            .or_else(|| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(ChatPage {
            chats: list.iter().filter_map(Chat::from_value).collect(),
        })
    }

    pub fn send_typing(&self, chat_id: &str) -> ApiResult<()> {
        self.request(
            "POST",
            "/v1/chats/typing",
            Body::Json(json!({ "chat_id": chat_id })),
        )
        .map(|_| ())
    }

    #[allow(dead_code)]
    pub fn delete_chat(&self, chat_id: &str) -> ApiResult<()> {
        self.request("DELETE", &format!("/v1/chats/{chat_id}"), Body::Empty)
            .map(|_| ())
    }

    // ---- messages -------------------------------------------------------

    /// Deprecated by the backend: returns only messages from the old
    /// MongoDB store. Used once per chat to import that history.
    pub fn legacy_history(&self, chat_id: &str, limit: usize) -> ApiResult<Vec<Value>> {
        let v = self.request(
            "GET",
            &format!("/v1/chats/{chat_id}/messages?limit={limit}"),
            Body::Empty,
        )?;
        Ok(messages_of(&v))
    }

    /// Messages in a `sequence_id` range (at most 500), to fill gaps.
    pub fn repair(&self, chat_id: &str, from: i64, to: i64) -> ApiResult<Vec<Value>> {
        let v = self.request(
            "GET",
            &format!("/v1/chats/{chat_id}/messages/repair?from_sequence={from}&to_sequence={to}"),
            Body::Empty,
        )?;
        Ok(messages_of(&v))
    }

    /// `multipart/form-data` with `client_message_id`, per the spec.
    pub fn send_text(
        &self,
        chat_id: &str,
        content: &str,
        reply_to: Option<&str>,
        client_message_id: &str,
    ) -> ApiResult<Value> {
        let form = Form::new()
            .text("content", content)
            .text("message_type", "text")
            .opt("reply_to", reply_to)
            .text("client_message_id", client_message_id)
            .finish();
        self.request("POST", &format!("/v1/chats/{chat_id}/messages"), form)
    }

    /// File or image upload in the `media` field.
    pub fn send_file(
        &self,
        chat_id: &str,
        file: &Path,
        reply_to: Option<&str>,
        client_message_id: &str,
    ) -> ApiResult<Value> {
        let bytes = std::fs::read(file).map_err(|e| invalid(&format!("อ่านไฟล์ไม่ได้: {e}")))?;
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let mime = mime_for(&name);
        let kind = if mime.starts_with("image/") {
            "image"
        } else {
            "file"
        };
        let form = Form::new()
            .text("message_type", kind)
            .opt("reply_to", reply_to)
            .text("client_message_id", client_message_id)
            .file("media", &name, mime, &bytes)
            .finish();
        self.request("POST", &format!("/v1/chats/{chat_id}/messages"), form)
    }

    pub fn mark_chat_read(&self, chat_id: &str) -> ApiResult<()> {
        self.request(
            "POST",
            &format!("/v1/chats/{chat_id}/messages/read-all"),
            Body::Empty,
        )
        .map(|_| ())
    }

    /// Downloads raw bytes, used for avatars and image messages.
    pub fn fetch_bytes(&self, url: &str) -> ApiResult<Vec<u8>> {
        let mut resp = self.agent.get(url).call().map_err(network)?;
        if !resp.status().is_success() {
            return Err(ApiError {
                status: Some(resp.status().as_u16()),
                message: "โหลดรูปไม่ได้".into(),
            });
        }
        let mut out = Vec::new();
        resp.body_mut()
            .as_reader()
            .take(15 * 1024 * 1024)
            .read_to_end(&mut out)
            .map_err(network)?;
        Ok(out)
    }
}

/// `{ messages: [...] }` or a bare array.
fn messages_of(v: &Value) -> Vec<Value> {
    v.get("messages")
        .and_then(Value::as_array)
        .or_else(|| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn tokens_from(v: &Value) -> Option<AuthTokens> {
    let pick = |a: &str, b: &str| {
        v.get(a)
            .or_else(|| v.get(b))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    Some(AuthTokens {
        access_token: pick("access_token", "accessToken")?,
        refresh_token: pick("refresh_token", "refreshToken"),
    })
}

fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_body_has_fields_and_terminator() {
        let Body::Multipart { boundary, bytes } = Form::new()
            .text("content", "สวัสดี")
            .opt("reply_to", None)
            .text("client_message_id", "tmp-1")
            .finish()
        else {
            unreachable!()
        };
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("name=\"content\"\r\n\r\nสวัสดี\r\n"));
        assert!(text.contains("name=\"client_message_id\"\r\n\r\ntmp-1\r\n"));
        assert!(!text.contains("reply_to"));
        assert!(text.ends_with(&format!("--{boundary}--\r\n")));
    }

    #[test]
    fn repair_response_shapes() {
        assert_eq!(
            messages_of(&json!({"messages": [{"message_id": "a"}], "pagination": {}})).len(),
            1
        );
        assert_eq!(
            messages_of(&json!([{"message_id": "a"}, {"message_id": "b"}])).len(),
            2
        );
    }
}
