//! Date, size and text formatting, ported from `shared/utils/*.ts`.
//! Labels are Thai to match the rest of the interface.

use chrono::{DateTime, Datelike, Local, NaiveDate, Utc};

use crate::models::parse_time;

const WEEKDAYS: [&str; 7] = ["จ.", "อ.", "พ.", "พฤ.", "ศ.", "ส.", "อา."];
const MONTHS: [&str; 12] = [
    "มกราคม",
    "กุมภาพันธ์",
    "มีนาคม",
    "เมษายน",
    "พฤษภาคม",
    "มิถุนายน",
    "กรกฎาคม",
    "สิงหาคม",
    "กันยายน",
    "ตุลาคม",
    "พฤศจิกายน",
    "ธันวาคม",
];

fn local(s: &str) -> DateTime<Local> {
    parse_time(s).with_timezone(&Local)
}

fn days_between(a: NaiveDate, b: NaiveDate) -> i64 {
    (b - a).num_days()
}

/// `14:05`
pub fn time(s: &str) -> String {
    local(s).format("%H:%M").to_string()
}

/// Chat list: time today, then `เมื่อวาน`, weekday within a week, then `d/m`.
pub fn chat_time(s: Option<&str>) -> String {
    let Some(s) = s else { return String::new() };
    chat_time_at(local(s), Local::now())
}

fn chat_time_at(t: DateTime<Local>, now: DateTime<Local>) -> String {
    match days_between(t.date_naive(), now.date_naive()) {
        0 => t.format("%H:%M").to_string(),
        1 => "เมื่อวาน".into(),
        2..=6 => WEEKDAYS[t.weekday().num_days_from_monday() as usize].into(),
        _ => format!("{}/{}", t.day(), t.month()),
    }
}

/// Divider between days in a room: `วันนี้`, `เมื่อวาน`, `20 พฤศจิกายน`.
pub fn date_divider(s: &str) -> String {
    date_divider_at(local(s), Local::now())
}

fn date_divider_at(t: DateTime<Local>, now: DateTime<Local>) -> String {
    match days_between(t.date_naive(), now.date_naive()) {
        0 => "วันนี้".into(),
        1 => "เมื่อวาน".into(),
        _ => {
            let base = format!("{} {}", t.day(), MONTHS[t.month0() as usize]);
            if t.year() == now.year() {
                base
            } else {
                format!("{base} {}", t.year() + 543)
            }
        }
    }
}

pub fn same_day(a: &str, b: &str) -> bool {
    local(a).date_naive() == local(b).date_naive()
}

pub fn file_size(bytes: i64) -> String {
    if bytes <= 0 {
        return "0 B".into();
    }
    let units = ["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

/// Character-safe truncation with an ellipsis, like `truncate()` in text.ts.
pub fn truncate(text: &str, max: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let mut out: String = flat.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// `http(s)://` links in a message, with trailing punctuation trimmed.
pub fn links(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|w| {
            let start = w.find("https://").or_else(|| w.find("http://"))?;
            let url = w[start..].trim_end_matches(|c: char| ".,;:!?)]}>\"'".contains(c));
            (url.len() > "https://".len()).then(|| url.to_string())
        })
        .collect()
}

pub fn countdown(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[allow(dead_code)]
pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn chat_time_buckets() {
        let now = Local.with_ymd_and_hms(2026, 11, 23, 12, 0, 0).unwrap();
        assert_eq!(
            chat_time_at(Local.with_ymd_and_hms(2026, 11, 23, 9, 5, 0).unwrap(), now),
            "09:05"
        );
        assert_eq!(
            chat_time_at(Local.with_ymd_and_hms(2026, 11, 22, 23, 0, 0).unwrap(), now),
            "เมื่อวาน"
        );
        assert_eq!(
            chat_time_at(Local.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap(), now),
            "1/11"
        );
    }

    #[test]
    fn divider_uses_buddhist_year_for_other_years() {
        let now = Local.with_ymd_and_hms(2026, 11, 23, 12, 0, 0).unwrap();
        assert_eq!(
            date_divider_at(Local.with_ymd_and_hms(2026, 11, 20, 8, 0, 0).unwrap(), now),
            "20 พฤศจิกายน"
        );
        assert_eq!(
            date_divider_at(Local.with_ymd_and_hms(2025, 1, 2, 8, 0, 0).unwrap(), now),
            "2 มกราคม 2568"
        );
    }

    #[test]
    fn finds_links() {
        assert_eq!(
            links("ดูที่ https://pinto-app.com/a?b=1, และ (http://x.dev)."),
            vec![
                "https://pinto-app.com/a?b=1".to_string(),
                "http://x.dev".to_string()
            ]
        );
        assert!(links("ไม่มีลิงก์ https://").is_empty());
    }

    #[test]
    fn truncates_by_chars() {
        assert_eq!(truncate("สวัสดีครับ", 4), "สวั…");
        assert_eq!(file_size(1536), "1.5 KB");
        assert_eq!(countdown(65), "1:05");
    }
}
