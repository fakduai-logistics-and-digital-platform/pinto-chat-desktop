//! Single-instance guard and activation IPC over loopback TCP.
//!
//! Adapted from Telegram Desktop's local-socket instance handoff: the primary
//! instance binds a local listener and writes `instance.lock` (`port\ntoken`).
//! Subsequent launches connect, send `SHOW` (plus optional argument), wait for
//! `OK`, and exit cleanly so two windows never contend over SQLite or WS.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use futures::channel::mpsc;

const LOCK_FILE_NAME: &str = "instance.lock";
const IO_TIMEOUT: Duration = Duration::from_millis(800);

/// Holds the lockfile path and removes it when the primary instance exits.
pub struct InstanceGuard {
    lock_path: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

pub enum AcquireResult {
    /// This process is the primary instance; drain `activations` to focus the window.
    Primary {
        _guard: InstanceGuard,
        activations: mpsc::UnboundedReceiver<Option<String>>,
    },
    /// Another instance is already running and acknowledged the activation request.
    SecondaryNotified,
}

fn read_lock_file(path: &Path) -> Option<(u16, String)> {
    let raw = fs::read_to_string(path).ok()?;
    let mut lines = raw.lines();
    let port: u16 = lines.next()?.trim().parse().ok()?;
    let token = lines.next()?.trim().to_string();
    if token.is_empty() {
        return None;
    }
    Some((port, token))
}

fn try_notify_existing(lock_path: &Path, arg: Option<&str>) -> bool {
    let Some((port, token)) = read_lock_file(lock_path) else {
        return false;
    };
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, IO_TIMEOUT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let payload = match arg {
        Some(a) if !a.is_empty() => format!("{token} SHOW {a}\n"),
        _ => format!("{token} SHOW\n"),
    };
    if stream.write_all(payload.as_bytes()).is_err() {
        return false;
    }

    let mut reader = BufReader::new(stream);
    let mut resp = String::new();
    reader.read_line(&mut resp).is_ok() && resp.trim() == "OK"
}

fn handle_client(
    mut stream: TcpStream,
    expected_token: &str,
    tx: &mpsc::UnboundedSender<Option<String>>,
) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let mut line = String::new();
    let Ok(mut reader) = stream.try_clone().map(BufReader::new) else {
        return;
    };
    if reader.read_line(&mut line).is_err() {
        return;
    }

    let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
    let Some((token, cmd)) = trimmed.split_once(' ') else {
        return;
    };
    if token != expected_token {
        return;
    }

    if let Some(rest) = cmd.strip_prefix("SHOW") {
        let arg = rest.trim();
        let arg = (!arg.is_empty()).then(|| arg.to_string());
        let _ = tx.unbounded_send(arg);
        let _ = stream.write_all(b"OK\n");
    }
}

/// Acquire the single-instance lock in `data_dir`, or notify the running instance.
pub fn acquire(data_dir: &Path, arg: Option<&str>) -> std::io::Result<AcquireResult> {
    fs::create_dir_all(data_dir)?;
    let lock_path = data_dir.join(LOCK_FILE_NAME);

    if try_notify_existing(&lock_path, arg) {
        return Ok(AcquireResult::SecondaryNotified);
    }

    // Stale lockfile (or no lockfile): bind a fresh loopback listener.
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let token = uuid::Uuid::new_v4().to_string();
    fs::write(&lock_path, format!("{port}\n{token}\n"))?;

    let (tx, rx) = mpsc::unbounded();
    thread::Builder::new()
        .name("single-instance".into())
        .spawn(move || {
            for conn in listener.incoming() {
                match conn {
                    Ok(stream) => handle_client(stream, &token, &tx),
                    Err(_) => break,
                }
            }
        })?;

    Ok(AcquireResult::Primary {
        _guard: InstanceGuard { lock_path },
        activations: rx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_receives_secondary_show_and_recovers_stale_lock() {
        let dir = std::env::temp_dir().join(format!("pinto_inst_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        // Stale lockfile pointing to an unused port should be replaced cleanly.
        fs::write(dir.join(LOCK_FILE_NAME), "1\nstale-token\n").unwrap();

        let AcquireResult::Primary {
            _guard,
            mut activations,
        } = acquire(&dir, None).expect("primary acquire")
        else {
            panic!("expected primary");
        };

        let second = acquire(&dir, Some("pinto://open/room-1")).expect("secondary acquire");
        assert!(matches!(second, AcquireResult::SecondaryNotified));

        let received = activations.try_recv().expect("activation message").unwrap();
        assert_eq!(received, "pinto://open/room-1");

        drop(_guard);
        assert!(!dir.join(LOCK_FILE_NAME).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
