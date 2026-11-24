//! Persistent file logging with startup rotation and panic/crash reporting.
//!
//! Adapted from Telegram Desktop's `log.txt` + `Updater.exe` `DebugLogs` /
//! unhandled-exception pattern so release builds (which have no console on
//! Windows) still retain diagnostic and crash logs in `data_dir()`.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const LOG_FILE_NAME: &str = "pinto-chat.log";
const PREV_LOG_FILE_NAME: &str = "pinto-chat.prev.log";

static FILE_LOGGER: OnceLock<FileLogger> = OnceLock::new();

struct FileLogger {
    inner: env_logger::Logger,
    file: Option<Mutex<File>>,
}

impl FileLogger {
    fn write_line(&self, record: &Record) {
        let Some(lock) = &self.file else {
            return;
        };
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        if let Ok(mut f) = lock.lock() {
            let _ = writeln!(
                f,
                "[{ts}] [{}] [{}] {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.inner.log(record);
            self.write_line(record);
        }
    }

    fn flush(&self) {
        self.inner.flush();
        if let Some(lock) = &self.file
            && let Ok(mut f) = lock.lock()
        {
            let _ = f.flush();
        }
    }
}

/// Rotate `pinto-chat.log` to `pinto-chat.prev.log` when it exceeds `max_bytes`,
/// then open it for appending.
fn open_log_file(dir: &Path, max_bytes: u64) -> std::io::Result<File> {
    fs::create_dir_all(dir)?;
    let log_path = dir.join(LOG_FILE_NAME);
    if let Ok(meta) = fs::metadata(&log_path)
        && meta.len() >= max_bytes
    {
        let prev_path = dir.join(PREV_LOG_FILE_NAME);
        let _ = fs::remove_file(&prev_path);
        let _ = fs::rename(&log_path, &prev_path);
    }
    OpenOptions::new().create(true).append(true).open(log_path)
}

/// Write a standalone crash report into `dir/DebugLogs/YYYYMMDD_HHMMSS_crash.txt`.
fn write_crash_report(dir: &Path, message: &str) -> Option<PathBuf> {
    let debug_dir = dir.join("DebugLogs");
    fs::create_dir_all(&debug_dir).ok()?;
    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let path = debug_dir.join(format!("{stamp}_crash.txt"));
    let mut f = File::create(&path).ok()?;
    let _ = writeln!(f, "Pinto Chat Desktop Crash Report ({stamp})");
    let _ = writeln!(f, "{message}");
    let _ = f.flush();
    Some(path)
}

fn install_panic_hook(log_dir: PathBuf) {
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("{info}");
        log::error!("PANIC: {msg}");
        log::logger().flush();
        let _ = write_crash_report(&log_dir, &msg);
        prev_hook(info);
    }));
}

/// Initialize stderr + persistent file logging (skipped in preview mode).
pub fn init(data_dir: &Path, enable_file_log: bool) {
    let inner = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,naga=warn,wgpu=warn"),
    )
    .build();
    let max_level = inner.filter();

    let file = if enable_file_log {
        open_log_file(data_dir, MAX_LOG_BYTES).ok().map(Mutex::new)
    } else {
        None
    };

    let logger = FILE_LOGGER.get_or_init(|| FileLogger { inner, file });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(max_level);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    if enable_file_log {
        install_panic_hook(data_dir.to_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_log_when_exceeding_limit() {
        let dir = std::env::temp_dir().join(format!("pinto_log_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();

        let log_path = dir.join(LOG_FILE_NAME);
        fs::write(&log_path, b"0123456789").unwrap();

        let mut f = open_log_file(&dir, 5).unwrap();
        writeln!(f, "new-entry").unwrap();
        drop(f);

        assert_eq!(
            fs::read_to_string(dir.join(PREV_LOG_FILE_NAME)).unwrap(),
            "0123456789"
        );
        assert!(fs::read_to_string(&log_path).unwrap().contains("new-entry"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_crash_report_in_debug_logs() {
        let dir = std::env::temp_dir().join(format!("pinto_crash_test_{}", uuid::Uuid::new_v4()));
        let report = write_crash_report(&dir, "boom at main.rs:42").unwrap();
        let contents = fs::read_to_string(&report).unwrap();
        assert!(contents.contains("Pinto Chat Desktop Crash Report"));
        assert!(contents.contains("boom at main.rs:42"));
        let _ = fs::remove_dir_all(&dir);
    }
}
