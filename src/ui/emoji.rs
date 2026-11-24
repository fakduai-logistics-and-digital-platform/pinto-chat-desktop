//! Telegram-style Emoji Picker panel (`TabbedPanel` + `EmojiListWidget`).

use std::fs;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::{env::data_dir, theme::Palette, ui::widgets::icon};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmojiCategory {
    #[default]
    Recent,
    Smileys,
    Gestures,
    Hearts,
    Nature,
    Food,
    Activities,
    Objects,
}

impl EmojiCategory {
    pub const ALL: [Self; 8] = [
        Self::Recent,
        Self::Smileys,
        Self::Gestures,
        Self::Hearts,
        Self::Nature,
        Self::Food,
        Self::Activities,
        Self::Objects,
    ];

    pub fn icon_label(self) -> &'static str {
        match self {
            Self::Recent => "🕒",
            Self::Smileys => "😀",
            Self::Gestures => "👍",
            Self::Hearts => "❤️",
            Self::Nature => "🌿",
            Self::Food => "🍔",
            Self::Activities => "🎉",
            Self::Objects => "💡",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Recent => "ใช้ล่าสุด",
            Self::Smileys => "ใบหน้าและอารมณ์",
            Self::Gestures => "มือและท่าทาง",
            Self::Hearts => "หัวใจและสัญลักษณ์",
            Self::Nature => "สัตว์และธรรมชาติ",
            Self::Food => "อาหารและเครื่องดื่ม",
            Self::Activities => "กิจกรรมและเฉลิมฉลอง",
            Self::Objects => "สิ่งของและไอคอน",
        }
    }

    pub fn items(self) -> &'static [&'static str] {
        match self {
            Self::Recent => &[],
            Self::Smileys => &[
                "😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "🙃", "😉", "😊", "😇", "🥰",
                "😍", "🤩", "😘", "😗", "😚", "😋", "😛", "😜", "🤪", "😝", "🤑", "🤗", "🤭", "🤫",
                "🤔", "🤐", "🤨", "😐", "😑", "😶", "😏", "😒", "🙄", "😬", "😌", "😔", "😪", "😴",
                "😷", "🤒", "🤕", "🤢", "🤮", "🥵", "🥶", "😵", "🤯", "🤠", "🥳", "😎", "🤓", "🧐",
                "😕", "😟", "🙁", "😮", "😯", "😲", "😳", "🥺", "😦", "😭", "😱", "😖", "😣", "😞",
                "😓", "😩", "😫", "🥱", "😤", "😡", "😠", "🤬",
            ],
            Self::Gestures => &[
                "👍", "👎", "👌", "🤌", "🤏", "✌️", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆", "👇",
                "☝️", "👋", "🤚", "🖐️", "✋", "🖖", "👏", "🙌", "👐", "🤲", "🤝", "🙏", "✍️", "💪",
                "🦾", "👀", "👁️", "🧠",
            ],
            Self::Hearts => &[
                "❤️",
                "🧡",
                "💛",
                "💚",
                "💙",
                "💜",
                "🖤",
                "🤍",
                "🤎",
                "💔",
                "❤️‍🔥",
                "💕",
                "💞",
                "💓",
                "💗",
                "💖",
                "💘",
                "💝",
                "✨",
                "⭐",
                "🌟",
                "💫",
                "🔥",
                "💥",
                "💯",
                "✅",
                "❌",
                "⚠️",
                "❓",
                "❗",
                "💬",
                "💤",
            ],
            Self::Nature => &[
                "🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸",
                "🐵", "🐔", "🐧", "🐦", "🦆", "🦉", "🦋", "🐝", "🌸", "🌹", "🌻", "🌺", "🌿", "🍀",
                "🍁", "🌈", "☀️", "🌙",
            ],
            Self::Food => &[
                "🍎", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🍒", "🍑", "🥭", "🍍", "🥥", "🥑", "🍅",
                "🍔", "🍟", "🍕", "🌭", "🥪", "🌮", "🍜", "🍣", "🍤", "🍦", "🍩", "🍪", "🎂", "🍰",
                "☕", "🍵", "🧋", "🍺",
            ],
            Self::Activities => &[
                "🎉", "🎊", "🎈", "🎁", "🎂", "🏆", "🥇", "⚽", "🏀", "🏈", "⚾", "🎾", "🏐", "🎮",
                "🕹️", "🎲", "🧩", "🎨", "🎬", "🎤", "🎧", "🎵", "🎶", "🎸", "🚀", "✈️", "🚗", "🏖️",
                "🏔️", "🏕️", "🎡", "🎆",
            ],
            Self::Objects => &[
                "💡", "🔦", "📱", "💻", "⌨️", "🖥️", "🖨️", "📷", "🎥", "📺", "📻", "⏰", "⏳", "🔋",
                "🔌", "💰", "💳", "💎", "🔑", "🔒", "🔓", "📌", "📍", "📎", "📝", "📁", "📅", "📊",
                "📈", "🔔", "📣", "🛠️",
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinTone {
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl SkinTone {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::Light,
        Self::MediumLight,
        Self::Medium,
        Self::MediumDark,
        Self::Dark,
    ];

    pub fn modifier(self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
        }
    }

    pub fn swatch(self) -> u32 {
        match self {
            Self::Default => 0xFFCC4D,
            Self::Light => 0xF7DECE,
            Self::MediumLight => 0xF3D2A2,
            Self::Medium => 0xD5AB88,
            Self::MediumDark => 0xAF7E57,
            Self::Dark => 0x7C533E,
        }
    }
}

const TONE_BASES: &[&str] = &[
    "👍", "👎", "👌", "🤌", "🤏", "✌️", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆", "👇", "☝️", "👋",
    "🤚", "🖐️", "✋", "🖖", "👏", "🙌", "👐", "🤲", "🙏", "✍️", "💪",
];

pub fn apply_skin_tone(emoji: &str, tone: SkinTone) -> String {
    if tone == SkinTone::Default || !TONE_BASES.contains(&emoji) {
        return emoji.to_string();
    }
    let clean = emoji.trim_end_matches('\u{FE0F}');
    format!("{clean}{}", tone.modifier())
}

const MAX_RECENT: usize = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EmojiPrefs {
    pub recent: Vec<String>,
    pub skin_tone: SkinTone,
}

impl Default for EmojiPrefs {
    fn default() -> Self {
        Self {
            recent: default_recent(),
            skin_tone: SkinTone::Default,
        }
    }
}

impl EmojiPrefs {
    pub fn load() -> Self {
        std::fs::read_to_string(data_dir().join("emoji.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = data_dir().join("emoji.json");
        let tmp = path.with_extension("tmp");
        if let Ok(s) = serde_json::to_string_pretty(self)
            && fs::write(&tmp, s).is_ok()
        {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

pub fn default_recent() -> Vec<String> {
    [
        "😀", "😂", "🥰", "👍", "🙏", "❤️", "🔥", "🎉", "😭", "🤔", "✨", "💯",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn record_recent(recent: &mut Vec<String>, emoji: &str) {
    recent.retain(|e| e != emoji);
    recent.insert(0, emoji.to_string());
    recent.truncate(MAX_RECENT);
}

const SEARCH_INDEX: &[(&str, &str)] = &[
    ("😀", "smile happy grin ยิ้ม ดีใจ หัวเราะ"),
    ("😂", "laugh cry joy lol ขำ ฮา น้ำตาไหล"),
    ("🤣", "rofl laugh ขำกลิ้ง ฮา"),
    ("🥰", "love adore hearts รัก เอ็นดู"),
    ("😍", "love heart eyes รัก ชอบ"),
    ("😘", "kiss love จุ๊บ รัก"),
    ("😭", "cry sad tears ร้องไห้ เสียใจ เศร้า"),
    ("😢", "sad cry เศร้า เสียใจ"),
    ("😡", "angry mad โกรธ โมโห"),
    ("🤔", "think hmm คิด สงสัย"),
    ("😴", "sleep tired ง่วง นอน"),
    ("🥳", "party celebrate ฉลอง ปาร์ตี้"),
    ("😎", "cool sunglasses เท่"),
    ("🙏", "please thanks pray ขอบคุณ ไหว้ สาธุ"),
    ("👍", "ok yes good like ดี เยี่ยม โอเค ถูกใจ"),
    ("👎", "no bad dislike ไม่ดี"),
    ("👌", "ok fine โอเค ได้"),
    ("✌️", "peace victory สู้ๆ"),
    ("👏", "clap applause ปรบมือ เก่ง"),
    ("❤️", "heart love red หัวใจ รัก"),
    ("💔", "broken heart อกหัก"),
    ("🔥", "fire hot lit ไฟ ร้อน แรง"),
    ("✨", "sparkle star วิ้ง ดาว"),
    ("💯", "100 perfect เต็มร้อย"),
    ("✅", "check done ok ถูก เสร็จ"),
    ("❌", "cross no wrong ผิด ยกเลิก"),
    ("🎉", "party tada ฉลอง ยินดี"),
    ("🎂", "cake birthday เค้ก วันเกิด"),
    ("☕", "coffee กาแฟ"),
    ("🧋", "boba tea ชานม"),
    ("🐱", "cat แมว"),
    ("🐶", "dog หมา สุนัข"),
    ("🚀", "rocket launch จรวด ลุย"),
    ("💡", "idea light ไอเดีย ไฟ"),
    ("📌", "pin ปักหมุด"),
    ("📎", "attach clip แนบไฟล์"),
];

pub fn filter_emojis(query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    SEARCH_INDEX
        .iter()
        .filter(|(emoji, tags)| emoji.contains(&q) || tags.to_lowercase().contains(&q))
        .map(|(emoji, _)| (*emoji).to_string())
        .collect()
}

pub struct EmojiPanelCallbacks<T, P, C, S, X> {
    pub on_tab: T,
    pub on_pick: P,
    pub on_clear_recent: C,
    pub on_skin_tone: S,
    pub on_clear_search: X,
}

/// Render the floating Telegram-style `TabbedPanel` Emoji Picker.
pub fn render_emoji_panel<T, P, C, S, X>(
    active_tab: EmojiCategory,
    prefs: &EmojiPrefs,
    search_input: &InputState,
    search_entity: &Entity<InputState>,
    p: &Palette,
    cb: EmojiPanelCallbacks<T, P, C, S, X>,
) -> impl IntoElement
where
    T: Fn(EmojiCategory, &mut Window, &mut App) + Clone + 'static,
    P: Fn(String, &mut Window, &mut App) + Clone + 'static,
    C: Fn(&mut Window, &mut App) + Clone + 'static,
    S: Fn(SkinTone, &mut Window, &mut App) + Clone + 'static,
    X: Fn(&mut Window, &mut App) + Clone + 'static,
{
    let query = search_input.value().to_string();
    let searching = !query.trim().is_empty();
    let emojis: Vec<String> = if searching {
        filter_emojis(&query)
            .into_iter()
            .map(|e| apply_skin_tone(&e, prefs.skin_tone))
            .collect()
    } else if active_tab == EmojiCategory::Recent {
        prefs.recent.clone()
    } else {
        active_tab
            .items()
            .iter()
            .map(|s| apply_skin_tone(s, prefs.skin_tone))
            .collect()
    };

    let tab_hover = p.surface_muted;
    let cell_hover = p.surface_muted;

    div()
        .w(px(352.))
        .h(px(344.))
        .rounded(px(14.))
        .bg(p.surface)
        .border_1()
        .border_color(p.divider)
        .shadow(vec![BoxShadow {
            color: p.shadow,
            offset: point(px(0.), px(8.)),
            blur_radius: px(22.),
            spread_radius: px(0.),
            inset: false,
        }])
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div().flex_none().px(px(10.)).pt(px(10.)).pb(px(6.)).child(
                div()
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(16.))
                    .bg(p.surface_muted)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(icon(IconName::Search, 14., p.ink_subtitle))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(search_entity).appearance(false)),
                    )
                    .when(searching, |d| {
                        let on_clear = cb.on_clear_search.clone();
                        d.child(
                            div()
                                .id("emoji-search-clear")
                                .size(px(20.))
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(icon(IconName::X, 12., p.ink_subtitle))
                                .on_click(move |_, window, cx| on_clear(window, cx)),
                        )
                    }),
            ),
        )
        .when(!searching, |d| {
            d.child(
                div()
                    .flex_none()
                    .px(px(8.))
                    .border_b_1()
                    .border_color(p.hairline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .children(EmojiCategory::ALL.into_iter().map(|cat| {
                        let active = cat == active_tab;
                        let on_tab = cb.on_tab.clone();
                        div()
                            .id(SharedString::from(format!("emoji-tab-{}", cat.title())))
                            .w(px(38.))
                            .h(px(34.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .when(!active, |d| {
                                d.rounded_t(px(6.)).hover(move |s| s.bg(tab_hover))
                            })
                            .child(div().h(px(2.)))
                            .child(div().text_size(px(16.)).child(cat.icon_label()))
                            .child(
                                div()
                                    .w(px(24.))
                                    .h(px(2.))
                                    .rounded_full()
                                    .when(active, |d| d.bg(p.primary)),
                            )
                            .on_click(move |_, window, cx| on_tab(cat, window, cx))
                    })),
            )
        })
        .child(
            div()
                .flex_none()
                .px(px(12.))
                .pt(px(8.))
                .pb(px(4.))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink_subtitle)
                        .child(if searching {
                            "ผลการค้นหา"
                        } else {
                            active_tab.title()
                        }),
                )
                .when(
                    !searching && active_tab == EmojiCategory::Recent && !prefs.recent.is_empty(),
                    |d| {
                        let on_clear = cb.on_clear_recent.clone();
                        d.child(
                            div()
                                .id("emoji-clear-recent")
                                .text_size(px(11.))
                                .text_color(p.primary_ink)
                                .cursor_pointer()
                                .child("ล้าง")
                                .on_click(move |_, window, cx| on_clear(window, cx)),
                        )
                    },
                ),
        )
        .child(
            div()
                .id("emoji-grid-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(8.))
                .pb(px(8.))
                .child(div().flex().flex_wrap().gap(px(2.)).children(
                    emojis.into_iter().enumerate().map(|(idx, emoji)| {
                        let on_pick = cb.on_pick.clone();
                        let picked = emoji.clone();
                        div()
                            .id(SharedString::from(format!("emoji-cell-{idx}")))
                            .size(px(40.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(22.))
                            .hover(move |s| s.bg(cell_hover))
                            .child(emoji)
                            .on_click(move |_, window, cx| {
                                on_pick(picked.clone(), window, cx);
                            })
                    }),
                )),
        )
        .child(
            div()
                .flex_none()
                .h(px(34.))
                .px(px(12.))
                .border_t_1()
                .border_color(p.hairline)
                .bg(p.surface)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(p.ink_subtitle)
                        .child("โทนสีผิว"),
                )
                .child(div().flex().items_center().gap(px(6.)).children(
                    SkinTone::ALL.into_iter().enumerate().map(|(i, tone)| {
                        let active = tone == prefs.skin_tone;
                        let on_tone = cb.on_skin_tone.clone();
                        div()
                            .id(SharedString::from(format!("skin-tone-{i}")))
                            .size(px(18.))
                            .rounded_full()
                            .cursor_pointer()
                            .bg(rgb(tone.swatch()))
                            .when(active, |d| d.border_2().border_color(p.primary))
                            .when(!active, |d| d.border_1().border_color(p.hairline))
                            .on_click(move |_, window, cx| on_tone(tone, window, cx))
                    }),
                )),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        EmojiCategory, EmojiPrefs, MAX_RECENT, SkinTone, apply_skin_tone, default_recent,
        filter_emojis, record_recent,
    };

    #[test]
    fn filters_emojis_by_thai_and_english_keywords() {
        assert!(filter_emojis("รัก").contains(&"❤️".to_string()));
        assert!(filter_emojis("rocket").contains(&"🚀".to_string()));
        assert!(filter_emojis("").is_empty());
    }

    #[test]
    fn applies_skin_tone_only_to_supported_gestures() {
        assert_eq!(apply_skin_tone("👍", SkinTone::Default), "👍");
        assert_eq!(apply_skin_tone("👍", SkinTone::Light), "👍\u{1F3FB}");
        assert_eq!(apply_skin_tone("❤️", SkinTone::Dark), "❤️");
    }

    #[test]
    fn prefs_roundtrip_json() {
        let prefs = EmojiPrefs {
            recent: vec!["🚀".into(), "🔥".into()],
            skin_tone: SkinTone::Medium,
        };
        let raw = serde_json::to_string(&prefs).unwrap();
        let back: EmojiPrefs = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.skin_tone, SkinTone::Medium);
        assert_eq!(back.recent, vec!["🚀", "🔥"]);
    }

    #[test]
    fn recent_emojis_deduplicates_and_caps() {
        let mut recent = default_recent();
        record_recent(&mut recent, "🚀");
        assert_eq!(recent[0], "🚀");

        record_recent(&mut recent, "🔥");
        assert_eq!(recent[0], "🔥");
        assert_eq!(recent.iter().filter(|e| *e == "🔥").count(), 1);

        for i in 0..40 {
            record_recent(&mut recent, &format!("e{i}"));
        }
        assert_eq!(recent.len(), MAX_RECENT);
    }

    #[test]
    fn non_recent_categories_have_items() {
        for cat in EmojiCategory::ALL {
            if cat != EmojiCategory::Recent {
                assert!(!cat.items().is_empty());
            }
        }
    }
}
