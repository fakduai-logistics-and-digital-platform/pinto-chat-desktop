//! WebSocket clients on background threads, ported from `ws-events.ts` and
//! `qr-login.service.ts`. Each socket reports through a channel; the UI side
//! drains it inside a GPUI task, so no UI state is touched off the main thread.

use std::{
    net::TcpStream,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use serde_json::Value;
use tungstenite::{Message as WsMessage, WebSocket, stream::MaybeTlsStream};

use crate::env::{QR_RECONNECT_DELAY_MS, WS_RECONNECT_DELAYS_MS};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// Dropping the handle stops the thread at its next read timeout.
pub struct SocketHandle {
    stop: Arc<AtomicBool>,
}

impl SocketHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for SocketHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn connect(url: &str) -> Result<Socket, String> {
    let (mut socket, _) = tungstenite::connect(url).map_err(|e| e.to_string())?;
    let timeout = Some(Duration::from_millis(500));
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => {
            let _ = s.set_read_timeout(timeout);
        }
        MaybeTlsStream::Rustls(s) => {
            let _ = s.get_mut().set_read_timeout(timeout);
        }
        _ => {}
    }
    Ok(socket)
}

enum Read {
    Text(String),
    Idle,
    Closed,
}

fn read(socket: &mut Socket) -> Read {
    match socket.read() {
        Ok(WsMessage::Text(t)) => Read::Text(t.to_string()),
        Ok(WsMessage::Close(_)) => Read::Closed,
        Ok(_) => Read::Idle,
        Err(tungstenite::Error::Io(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Read::Idle
        }
        Err(_) => Read::Closed,
    }
}

/// Sleeps in small steps so a stop request is honoured quickly.
fn wait(stop: &AtomicBool, ms: u64) -> bool {
    let mut left = ms;
    while left > 0 {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        let step = left.min(100);
        thread::sleep(Duration::from_millis(step));
        left -= step;
    }
    !stop.load(Ordering::SeqCst)
}

// ---- chat socket --------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug)]
pub enum ChatEvent {
    State(ConnectionState),
    /// `{ type, data }` frames from `/v1/ws/chat`.
    Frame {
        kind: String,
        data: Value,
    },
}

/// `url` is called before every (re)connect so a refreshed token is picked up;
/// `None` (signed out) stops the loop.
pub fn spawn_chat_socket(
    url: impl Fn() -> Option<String> + Send + 'static,
) -> (SocketHandle, UnboundedReceiver<ChatEvent>) {
    let (tx, rx) = unbounded();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    thread::Builder::new()
        .name("pinto-chat-ws".into())
        .spawn(move || chat_loop(url, tx, flag))
        .expect("spawn chat socket thread");
    (SocketHandle { stop }, rx)
}

fn chat_loop(
    url: impl Fn() -> Option<String>,
    tx: UnboundedSender<ChatEvent>,
    stop: Arc<AtomicBool>,
) {
    let mut attempt = 0usize;
    while !stop.load(Ordering::SeqCst) {
        let _ = tx.unbounded_send(ChatEvent::State(ConnectionState::Connecting));
        let Some(url) = url() else { return };
        match connect(&url) {
            Ok(mut socket) => {
                attempt = 0;
                let _ = tx.unbounded_send(ChatEvent::State(ConnectionState::Connected));
                loop {
                    if stop.load(Ordering::SeqCst) {
                        let _ = socket.close(None);
                        return;
                    }
                    match read(&mut socket) {
                        Read::Text(t) => {
                            if let Ok(v) = serde_json::from_str::<Value>(&t) {
                                let kind = v
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                                let data = v.get("data").cloned().unwrap_or(Value::Null);
                                if tx.unbounded_send(ChatEvent::Frame { kind, data }).is_err() {
                                    return;
                                }
                            }
                        }
                        Read::Idle => {}
                        Read::Closed => break,
                    }
                }
            }
            Err(e) => log::warn!("chat ws connect failed: {e}"),
        }
        let _ = tx.unbounded_send(ChatEvent::State(ConnectionState::Disconnected));
        let delay = WS_RECONNECT_DELAYS_MS[attempt.min(WS_RECONNECT_DELAYS_MS.len() - 1)];
        attempt += 1;
        if !wait(&stop, delay) {
            return;
        }
    }
}

// ---- QR login socket ----------------------------------------------------

#[derive(Debug, Clone)]
pub enum QrEvent {
    Connecting,
    Open,
    SessionReady {
        session_id: String,
        expires_in: u64,
    },
    LoginSuccess {
        access_token: String,
        refresh_token: Option<String>,
        user: Option<Value>,
    },
    Expired,
    Error(String),
}

/// Holds the QR session open until login succeeds or the handle is dropped.
/// Expiry and drops reconnect with a fresh session after 3 seconds.
pub fn spawn_qr_socket(url: String) -> (SocketHandle, UnboundedReceiver<QrEvent>) {
    let (tx, rx) = unbounded();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    thread::Builder::new()
        .name("pinto-qr-ws".into())
        .spawn(move || qr_loop(url, tx, flag))
        .expect("spawn qr socket thread");
    (SocketHandle { stop }, rx)
}

fn qr_loop(url: String, tx: UnboundedSender<QrEvent>, stop: Arc<AtomicBool>) {
    let send = |e: QrEvent| tx.unbounded_send(e).is_ok();
    while !stop.load(Ordering::SeqCst) {
        send(QrEvent::Connecting);
        match connect(&url) {
            Ok(mut socket) => {
                send(QrEvent::Open);
                loop {
                    if stop.load(Ordering::SeqCst) {
                        let _ = socket.close(None);
                        return;
                    }
                    match read(&mut socket) {
                        Read::Text(t) => {
                            let Ok(v) = serde_json::from_str::<Value>(&t) else {
                                continue;
                            };
                            match parse_qr(&v) {
                                Some(e @ QrEvent::LoginSuccess { .. }) => {
                                    send(e);
                                    let _ = socket.close(None);
                                    return;
                                }
                                Some(QrEvent::Expired) => {
                                    send(QrEvent::Expired);
                                    break;
                                }
                                Some(e) => {
                                    send(e);
                                }
                                None => {}
                            }
                        }
                        Read::Idle => {}
                        Read::Closed => break,
                    }
                }
            }
            Err(e) => {
                send(QrEvent::Error(format!("เชื่อมต่อไม่สำเร็จ ({e})")));
            }
        }
        if !wait(&stop, QR_RECONNECT_DELAY_MS) {
            return;
        }
    }
}

fn parse_qr(v: &Value) -> Option<QrEvent> {
    let data = v.get("data").cloned().unwrap_or(Value::Null);
    let field = |k: &str| v.get(k).or_else(|| data.get(k)).cloned();
    match v.get("type")?.as_str()? {
        "init" => Some(QrEvent::SessionReady {
            session_id: field("session_id")?.as_str()?.to_string(),
            expires_in: field("expires_in").and_then(|e| e.as_u64()).unwrap_or(60),
        }),
        "login_success" => {
            let tokens = crate::api::tokens_from(v).or_else(|| crate::api::tokens_from(&data))?;
            Some(QrEvent::LoginSuccess {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                user: field("user"),
            })
        }
        "expired" => Some(QrEvent::Expired),
        "error" => Some(QrEvent::Error(
            field("message")
                .and_then(|m| m.as_str().map(str::to_string))
                .unwrap_or_else(|| "เกิดข้อผิดพลาด".into()),
        )),
        _ => None,
    }
}

/// QR payload the mobile app scans. Carries no credentials.
pub fn qr_payload(session_id: &str) -> String {
    serde_json::json!({ "type_mode": "scan_login", "session_id": session_id }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_init_in_both_shapes() {
        let a = parse_qr(&json!({"type": "init", "session_id": "s1", "expires_in": 45}));
        let b = parse_qr(&json!({"type": "init", "data": {"session_id": "s2"}}));
        assert!(
            matches!(a, Some(QrEvent::SessionReady { ref session_id, expires_in: 45 }) if session_id == "s1")
        );
        assert!(
            matches!(b, Some(QrEvent::SessionReady { ref session_id, expires_in: 60 }) if session_id == "s2")
        );
    }

    #[test]
    fn parses_login_success_camel_case() {
        let e =
            parse_qr(&json!({"type": "login_success", "accessToken": "a", "refreshToken": "r"}));
        assert!(
            matches!(e, Some(QrEvent::LoginSuccess { ref access_token, .. }) if access_token == "a")
        );
    }
}
