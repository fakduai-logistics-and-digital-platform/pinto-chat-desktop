//! Local storage inspection and safe cleanup for logs and crash reports.
//!
//! Adapted from Telegram Desktop's `Settings::LocalStorage` stats & cleanup.

use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StorageStats {
    pub db_bytes: u64,
    pub media_bytes: u64,
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

/// Measure local SQLite database, media cache, and log/crash-report disk usage.
pub fn inspect(data_dir: &Path) -> StorageStats {
    let db_bytes = file_len(&data_dir.join("pinto-chat.sqlite"))
        + file_len(&data_dir.join("pinto-chat.sqlite-wal"))
        + file_len(&data_dir.join("pinto-chat.sqlite-shm"));
    let media_bytes = dir_len(&data_dir.join("media_cache"));
    let log_bytes = file_len(&data_dir.join("pinto-chat.log"))
        + file_len(&data_dir.join("pinto-chat.prev.log"))
        + dir_len(&data_dir.join("DebugLogs"));
    StorageStats {
        db_bytes,
        media_bytes,
        log_bytes,
    }
}

fn cache_key(url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in url.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Read cached image/media bytes from `data_dir/media_cache/`.
pub fn read_cached_media(data_dir: &Path, url: &str) -> Option<Vec<u8>> {
    fs::read(data_dir.join("media_cache").join(cache_key(url))).ok()
}

/// Persist downloaded image/media bytes to `data_dir/media_cache/`.
pub fn write_cached_media(data_dir: &Path, url: &str, bytes: &[u8]) {
    let dir = data_dir.join("media_cache");
    if fs::create_dir_all(&dir).is_ok() {
        let _ = fs::write(dir.join(cache_key(url)), bytes);
    }
}

/// Remove rotated logs, crash reports, and cached media, returning freed bytes.
pub fn clear_old_logs(data_dir: &Path) -> u64 {
    let mut freed = 0u64;
    let prev = data_dir.join("pinto-chat.prev.log");
    let prev_len = file_len(&prev);
    if fs::remove_file(&prev).is_ok() {
        freed = freed.saturating_add(prev_len);
    }

    for sub in ["DebugLogs", "media_cache"] {
        let dir = data_dir.join(sub);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let len = file_len(&p);
                if p.is_file() && fs::remove_file(&p).is_ok() {
                    freed = freed.saturating_add(len);
                }
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

        write_cached_media(&dir, "https://cdn.example.com/a.png", b"imgbytes");
        assert_eq!(
            read_cached_media(&dir, "https://cdn.example.com/a.png").as_deref(),
            Some(&b"imgbytes"[..])
        );
        assert_eq!(inspect(&dir).media_bytes, 8);

        let freed = clear_old_logs(&dir);
        assert_eq!(freed, 776);
        assert!(dir.join("pinto-chat.sqlite").exists());
        assert_eq!(inspect(&dir).log_bytes, 0);
        assert_eq!(inspect(&dir).media_bytes, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn formats_byte_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
