//! Local storage inspection and safe cleanup for logs and crash reports.
//!
//! Adapted from Telegram Desktop's `Settings::LocalStorage` stats & cleanup.

use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StorageStats {
    pub db_bytes: u64,
    pub log_bytes: u64,
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn dir_len(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                dir_len(&p)
            } else {
                file_len(&p)
            }
        })
        .sum()
}

/// Measure local SQLite database and log/crash-report disk usage in `data_dir`.
pub fn inspect(data_dir: &Path) -> StorageStats {
    let db_bytes = file_len(&data_dir.join("pinto-chat.sqlite"))
        + file_len(&data_dir.join("pinto-chat.sqlite-wal"))
        + file_len(&data_dir.join("pinto-chat.sqlite-shm"));
    let log_bytes = file_len(&data_dir.join("pinto-chat.log"))
        + file_len(&data_dir.join("pinto-chat.prev.log"))
        + dir_len(&data_dir.join("DebugLogs"));
    StorageStats {
        db_bytes,
        log_bytes,
    }
}

/// Remove rotated logs and crash reports in `DebugLogs/`, returning freed bytes.
pub fn clear_old_logs(data_dir: &Path) -> u64 {
    let mut freed = 0u64;
    let prev = data_dir.join("pinto-chat.prev.log");
    let prev_len = file_len(&prev);
    if fs::remove_file(&prev).is_ok() {
        freed = freed.saturating_add(prev_len);
    }

    let debug_dir = data_dir.join("DebugLogs");
    if let Ok(entries) = fs::read_dir(&debug_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let len = file_len(&p);
            if p.is_file() && fs::remove_file(&p).is_ok() {
                freed = freed.saturating_add(len);
            }
        }
    }
    freed
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_and_clear_old_logs_preserves_db() {
        let dir = std::env::temp_dir().join(format!("pinto_cache_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join("DebugLogs")).unwrap();
        fs::write(dir.join("pinto-chat.sqlite"), vec![1u8; 1024]).unwrap();
        fs::write(dir.join("pinto-chat.prev.log"), vec![2u8; 512]).unwrap();
        fs::write(dir.join("DebugLogs/crash.txt"), vec![3u8; 256]).unwrap();

        let stats = inspect(&dir);
        assert_eq!(stats.db_bytes, 1024);
        assert_eq!(stats.log_bytes, 768);

        let freed = clear_old_logs(&dir);
        assert_eq!(freed, 768);
        assert!(dir.join("pinto-chat.sqlite").exists());
        assert_eq!(inspect(&dir).log_bytes, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn formats_byte_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
