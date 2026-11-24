//! Chat room, ported from `ChatRoom.vue`, `MessageBubble.vue`,
//! `MessageComposer.vue`, `DateDivider.vue` and `TypingIndicator.vue`.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants},
    dialog::DialogButtonProps,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem},
    scroll::ScrollableElement,
    skeleton::Skeleton,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::sync::Arc;

use crate::{
    env::{MAX_MESSAGE_LENGTH, TYPING_DEBOUNCE_MS},
    format,
    models::{
        ChatParticipant, ChatType, Message, MessageStatus, MessageType, ReplyTarget, UserProfile,
    },
    persist::Wallpaper,
    realtime::ConnectionState,
    store::{Store, StoreEvent},
    theme::{Palette, PaletteExt},
    ui::emoji::{
        EmojiCategory, EmojiPanelCallbacks, EmojiPrefs, record_recent, render_emoji_panel,
    },
    ui::widgets::{ICON, avatar, drag_region, icon, icon_button, tnum, verified_badge},
};

/// Tabs of the side panel listing what a chat has shared.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaTab {
    Images,
    Files,
    Links,
    Members,
}

impl MediaTab {
    fn label(self) -> &'static str {
        match self {
            Self::Images => "รูปภาพ",
            Self::Files => "ไฟล์",
            Self::Links => "ลิงก์",
            Self::Members => "สมาชิก",
        }
    }
}

/// Dark ink on the brand green keeps message text above 4.5:1.
const ON_BUBBLE: u32 = 0x10281A;

/// Shared member row used in both the right-hand Members panel and the Members dialog.
fn render_member_row(
    i: usize,
    m: ChatParticipant,
    id_prefix: &'static str,
    store: &Entity<Store>,
    room: gpui::WeakEntity<RoomView>,
    cx: &mut App,
    p: &Palette,
) -> AnyElement {
    let img = m
        .avatar_url
        .as_deref()
        .and_then(|u| store.update(cx, |s, cx| s.image(u, cx)));
    let role_label = match m.role.as_str() {
        "owner" => Some("เจ้าของกลุ่ม"),
        "admin" => Some("แอดมิน"),
        _ => None,
    };
    let member_for_click = m.clone();
    div()
        .id(SharedString::from(format!(
            "{id_prefix}-member-{i}-{}",
            m.user_id
        )))
        .flex()
        .items_center()
        .gap(px(10.))
        .p(px(8.))
        .rounded(px(8.))
        .cursor_pointer()
        .hover(|s| s.bg(p.surface_muted))
        .child(avatar(img, 36., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(m.name),
                        )
                        .when(m.is_official, |d| d.child(verified_badge(13., p))),
                )
                .when(!m.username.is_empty(), |d| {
                    d.child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(p.ink_subtitle)
                            .child(format!("@{}", m.username)),
                    )
                }),
        )
        .when_some(role_label, |d, role| {
            d.child(
                div()
                    .flex_none()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(p.primary_tint)
                    .text_size(px(11.))
                    .text_color(p.primary_ink)
                    .child(role),
            )
        })
        .on_click(move |_, window, cx| {
            let member = member_for_click.clone();
            let _ = room.update(cx, |this, cx| {
                this.open_profile_dialog(member, window, cx);
            });
        })
        .into_any_element()
}

/// Detect animated GIF / WebP attachments sent as `File` or `Image` messages.
fn animated_badge(m: &Message) -> Option<&'static str> {
    let candidates = [m.file_name.as_deref(), m.media_url.as_deref()];
    for raw in candidates.into_iter().flatten() {
        let clean = raw.split('?').next().unwrap_or(raw).to_ascii_lowercase();
        if clean.ends_with(".gif") {
            return Some("GIF");
        }
        if clean.ends_with(".webp") {
            return Some("WEBP");
        }
    }
    None
}

/// Detect video attachments sent as `File` messages by extension.
fn is_video_message(m: &Message) -> bool {
    if m.message_type != MessageType::File {
        return false;
    }
    let candidates = [m.file_name.as_deref(), m.media_url.as_deref()];
    for raw in candidates.into_iter().flatten() {
        let clean = raw
            .split(['?', '#'])
            .next()
            .unwrap_or(raw)
            .to_ascii_lowercase();
        if [".mp4", ".mov", ".webm", ".mkv", ".avi", ".m4v", ".3gp"]
            .iter()
            .any(|ext| clean.ends_with(ext))
        {
            return true;
        }
    }
    false
}

pub struct RoomView {
    store: Entity<Store>,
    composer: Entity<InputState>,
    scroll: ScrollHandle,
    reply: Option<ReplyTarget>,
    typing_epoch: u64,
    stick_to_bottom: bool,
    /// Set while older messages load, to keep the reading position.
    anchor: Option<Pixels>,
    last_chat: Option<String>,
    room_epoch: u64,
    panel: Option<MediaTab>,
    panel_scroll: ScrollHandle,
    members_dialog_scroll: ScrollHandle,
    group_info_scroll: ScrollHandle,
    group_name_input: Entity<InputState>,
    drafts: std::collections::HashMap<String, (String, Option<ReplyTarget>)>,
    scroll_states: std::collections::HashMap<String, (bool, Point<Pixels>)>,
    emoji_open: bool,
    emoji_tab: EmojiCategory,
    emoji_search: Entity<InputState>,
    emoji_prefs: EmojiPrefs,
}

impl RoomView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("ข้อความ"));
        let emoji_search = cx.new(|cx| InputState::new(window, cx).placeholder("ค้นหาอีโมจิ..."));
        let group_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("ชื่อกลุ่ม"));
        cx.subscribe(&emoji_search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
        cx.subscribe_in(
            &composer,
            window,
            |this, input, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { shift: false, .. } => this.send(window, cx),
                InputEvent::Change => {
                    let value = input.read(cx).value();
                    if value.chars().count() > MAX_MESSAGE_LENGTH {
                        let cut: String = value.chars().take(MAX_MESSAGE_LENGTH).collect();
                        input.update(cx, |s, cx| s.set_value(cut, window, cx));
                    }
                    this.on_typing(cx);
                    cx.notify();
                }
                _ => {}
            },
        )
        .detach();
        cx.subscribe_in(
            &store,
            window,
            |this, store, ev: &StoreEvent, window, cx| {
                if let StoreEvent::MessagesAppended = ev {
                    let selected = store.read(cx).selected_chat.as_deref();
                    let changed = selected != this.last_chat.as_deref();
                    let saved_scrolled_up = selected
                        .and_then(|id| this.scroll_states.get(id))
                        .is_some_and(|(at_bottom, _)| !*at_bottom);
                    if (changed && !saved_scrolled_up)
                        || (!changed
                            && (this.stick_to_bottom || this.distance_from_bottom() < px(300.)))
                    {
                        this.scroll_to_bottom(window, cx);
                    }
                }
            },
        )
        .detach();
        cx.observe_in(&store, window, |this, store, window, cx| {
            let selected = store.read(cx).selected_chat.clone();
            if selected != this.last_chat {
                if let Some(prev) = this.last_chat.take() {
                    let draft_text = this.composer.read(cx).value().to_string();
                    if draft_text.is_empty() && this.reply.is_none() {
                        this.drafts.remove(&prev);
                    } else {
                        this.drafts
                            .insert(prev.clone(), (draft_text, this.reply.clone()));
                    }
                    let at_bottom = this.stick_to_bottom || this.distance_from_bottom() < px(8.);
                    this.scroll_states
                        .insert(prev, (at_bottom, this.scroll.offset()));
                    // Switching chats closes the panel; the first open keeps it.
                    this.panel = None;
                }
                this.last_chat = selected.clone();
                this.room_epoch += 1;
                if let Some(id) = selected.as_deref() {
                    let (draft_text, reply) = this
                        .drafts
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| (String::new(), None));
                    this.reply = reply;
                    this.composer
                        .update(cx, |s, cx| s.set_value(draft_text, window, cx));
                    if let Some(&(false, offset)) = this.scroll_states.get(id) {
                        this.stick_to_bottom = false;
                        this.scroll.set_offset(offset);
                        let handle = this.scroll.clone();
                        window.on_next_frame(move |_, _| handle.set_offset(offset));
                    } else {
                        this.scroll_to_bottom(window, cx);
                    }
                } else {
                    this.reply = None;
                    this.composer
                        .update(cx, |s, cx| s.set_value("", window, cx));
                    this.scroll_to_bottom(window, cx);
                }
            }
            cx.notify();
        })
        .detach();
        Self {
            store,
            composer,
            scroll: ScrollHandle::new(),
            reply: None,
            typing_epoch: 0,
            stick_to_bottom: true,
            anchor: None,
            last_chat: None,
            room_epoch: 0,
            panel: (cfg!(debug_assertions)
                && std::env::var("PINTO_PREVIEW").is_ok_and(|v| v.contains("panel")))
            .then_some(MediaTab::Files),
            panel_scroll: ScrollHandle::new(),
            members_dialog_scroll: ScrollHandle::new(),
            group_info_scroll: ScrollHandle::new(),
            group_name_input,
            drafts: std::collections::HashMap::new(),
            scroll_states: std::collections::HashMap::new(),
            emoji_open: false,
            emoji_tab: EmojiCategory::Recent,
            emoji_search,
            emoji_prefs: if cfg!(debug_assertions) && std::env::var("PINTO_PREVIEW").is_ok() {
                EmojiPrefs::default()
            } else {
                EmojiPrefs::load()
            },
        }
    }

    fn distance_from_bottom(&self) -> Pixels {
        self.scroll.max_offset().y + self.scroll.offset().y
    }

    fn scroll_to_bottom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stick_to_bottom = true;
        self.scroll.scroll_to_bottom();
        // Layout of new rows lands next frame; scroll again once it has.
        let handle = self.scroll.clone();
        window.on_next_frame(move |_, _| handle.scroll_to_bottom());
        cx.notify();
    }

    fn on_typing(&mut self, cx: &mut Context<Self>) {
        self.typing_epoch += 1;
        let epoch = self.typing_epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(TYPING_DEBOUNCE_MS))
                .await;
            this.update(cx, |this, cx| {
                if this.typing_epoch != epoch {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |s, cx| {
                    if let Some(id) = s.selected_chat.clone() {
                        s.send_typing(&id, cx);
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().trim().to_string();
        let Some(chat_id) = self.store.read(cx).selected_chat.clone() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let reply = self.reply.take().map(|r| r.message_id);
        self.emoji_open = false;
        self.composer
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.typing_epoch += 1;
        self.stick_to_bottom = true;
        self.store
            .update(cx, |s, cx| s.send_text(&chat_id, text, reply, cx));
    }

    fn attach(&mut self, cx: &mut Context<Self>) {
        let Some(chat_id) = self.store.read(cx).selected_chat.clone() else {
            return;
        };
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("ส่งไฟล์".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            this.update(cx, |this, cx| {
                let reply = this.reply.take().map(|r| r.message_id);
                this.stick_to_bottom = true;
                this.store
                    .update(cx, |s, cx| s.send_file(&chat_id, path, reply, cx));
            })
            .ok();
        })
        .detach();
    }

    fn start_reply(&mut self, message_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let Some(chat) = store.selected_chat.clone() else {
            return;
        };
        self.reply = store.reply_target(&chat, message_id);
        self.composer.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn copy(&mut self, message_id: &str, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let Some(chat) = store.selected_chat.clone() else {
            return;
        };
        if let Some(m) = store
            .messages(&chat)
            .iter()
            .find(|m| m.message_id == message_id)
            && !m.text().is_empty()
        {
            cx.write_to_clipboard(ClipboardItem::new_string(m.text().to_string()));
            self.store
                .update(cx, |_, cx| cx.emit(StoreEvent::Toast("คัดลอกแล้ว".into())));
        }
    }

    fn confirm_delete(&mut self, message_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let store = store.clone();
            let id = message_id.clone();
            dialog
                .title("ลบข้อความนี้")
                .w(px(400.))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(p.ink_secondary)
                        .child("ข้อความจะถูกซ่อนจากเครื่องนี้เท่านั้น ยังไม่ได้ลบออกจากเซิร์ฟเวอร์"),
                )
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("ลบ")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("ยกเลิก")
                        .show_cancel(true),
                )
                .footer(div())
                .on_ok(move |_, _, cx| {
                    let id = id.clone();
                    store.update(cx, |s, cx| {
                        if let Some(chat) = s.selected_chat.clone() {
                            s.delete_message(&chat, &id, cx);
                        }
                    });
                    true
                })
        });
    }

    // ---- local-only chat features --------------------------------------

    fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            if let Some(id) = s.selected_chat.clone() {
                s.toggle_mute(&id, cx);
            }
        });
    }

    fn toggle_pin(&mut self, message_id: &str, cx: &mut Context<Self>) {
        let id = message_id.to_string();
        self.store.update(cx, |s, cx| {
            if let Some(chat) = s.selected_chat.clone() {
                let next = (!s.is_pinned(&chat, &id)).then_some(id);
                s.set_pin(&chat, next, cx);
            }
        });
    }

    fn open_panel(&mut self, tab: MediaTab, cx: &mut Context<Self>) {
        self.panel = Some(tab);
        cx.notify();
    }

    fn set_wallpaper(&mut self, wallpaper: Wallpaper, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            s.update_settings(|st| st.wallpaper = wallpaper, cx)
        });
    }

    /// Writes the conversation stored on this device to a `.txt` file.
    fn export(&mut self, cx: &mut Context<Self>) {
        let store = self.store.read(cx);
        let Some(chat) = store.selected().cloned() else {
            return;
        };
        let text = store.export_text(&chat.chat_id);
        let dir = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        let safe: String = chat
            .display_name()
            .chars()
            .map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c })
            .collect();
        let rx = cx.prompt_for_new_path(&dir, Some(&format!("แชท {safe}.txt")));
        let store = self.store.downgrade();
        cx.spawn(async move |_, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let message = match std::fs::write(&path, text) {
                Ok(()) => "บันทึกแชทแล้ว".to_string(),
                Err(e) => format!("บันทึกไม่สำเร็จ: {e}"),
            };
            store
                .update(cx, |_, cx| cx.emit(StoreEvent::Toast(message.into())))
                .ok();
        })
        .detach();
    }

    fn render_pinned(&self, cx: &mut Context<Self>, p: &Palette) -> Option<AnyElement> {
        let store = self.store.read(cx);
        let chat = store.selected_chat.clone()?;
        let m = store.pinned_message(&chat)?;
        let who = store.sender_name(&m);
        let what = match m.message_type {
            MessageType::Image => "รูปภาพ".to_string(),
            MessageType::File => m.file_name.clone().unwrap_or_else(|| "ไฟล์".into()),
            _ => format::truncate(m.text(), 80),
        };
        let id = m.message_id.clone();
        Some(
            div()
                .h(px(48.))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(16.))
                .bg(p.surface)
                .border_b_1()
                .border_color(p.divider)
                .child(icon(IconName::Pin, 16., p.primary_ink))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.ink_subtitle)
                                .child(format!("ปักหมุดโดยคุณ · {who}")),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(p.ink)
                                .child(what),
                        ),
                )
                .child(
                    icon_button("unpin", IconName::X, 28., p.ink_subtitle, p)
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_pin(&id, cx))),
                )
                .into_any_element(),
        )
    }

    fn render_panel(&mut self, tab: MediaTab, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let Some(chat) = self.store.read(cx).selected_chat.clone() else {
            return div().into_any_element();
        };
        let all = self.store.read(cx).all_messages(&chat);
        let hover = p.surface_muted;
        let empty = |text: &'static str| {
            div()
                .py(px(48.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .child(div().text_size(px(14.)).text_color(p.ink).child(text))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .child("จากข้อความที่อยู่ในเครื่องนี้"),
                )
                .into_any_element()
        };

        let body: AnyElement = match tab {
            MediaTab::Images => {
                let items: Vec<(String, Option<Arc<Image>>)> = all
                    .iter()
                    .filter(|m| m.message_type == MessageType::Image)
                    .filter_map(|m| m.media_url.clone())
                    .rev()
                    .map(|url| {
                        let img_data = self.store.update(cx, |s, cx| s.image(&url, cx));
                        (url, img_data)
                    })
                    .collect();
                if items.is_empty() {
                    empty("ยังไม่มีรูปภาพในแชทนี้")
                } else {
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.))
                        .children(items.into_iter().enumerate().map(|(i, (url, data))| {
                            let click_url = url.clone();
                            div()
                                .id(("media-img", i))
                                .size(px(92.))
                                .rounded(px(8.))
                                .overflow_hidden()
                                .bg(p.surface_muted)
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(match data {
                                    Some(d) => img(d)
                                        .size(px(92.))
                                        .rounded(px(8.))
                                        .object_fit(ObjectFit::Cover)
                                        .into_any_element(),
                                    None => icon(IconName::Image, 18., p.ink_disabled)
                                        .into_any_element(),
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_image_dialog(
                                        click_url.clone(),
                                        None,
                                        None,
                                        window,
                                        cx,
                                    );
                                }))
                        }))
                        .into_any_element()
                }
            }
            MediaTab::Files => {
                let files: Vec<&Message> = all
                    .iter()
                    .filter(|m| m.message_type == MessageType::File)
                    .rev()
                    .collect();
                if files.is_empty() {
                    empty("ยังไม่มีไฟล์ในแชทนี้")
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .children(files.into_iter().enumerate().map(|(i, m)| {
                            let url = m.media_url.clone();
                            let is_video = is_video_message(m);
                            let file_name = m.file_name.clone().unwrap_or_else(|| {
                                if is_video {
                                    "วิดีโอ"
                                } else {
                                    "ไฟล์"
                                }
                                .into()
                            });
                            let file_size = m.file_size;
                            let meta = match m.file_size {
                                Some(sz) => format!(
                                    "{} · {}",
                                    format::file_size(sz),
                                    format::chat_time(Some(&m.created_at))
                                ),
                                None => format::chat_time(Some(&m.created_at)),
                            };
                            div()
                                .id(("media-file", i))
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .px(px(8.))
                                .py(px(8.))
                                .rounded(px(8.))
                                .when(url.is_some(), |d| {
                                    d.cursor_pointer().hover(move |s| s.bg(hover))
                                })
                                .child(
                                    div()
                                        .size(px(36.))
                                        .flex_none()
                                        .rounded(px(8.))
                                        .bg(p.surface_muted)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(if is_video {
                                            div()
                                                .text_size(px(13.))
                                                .text_color(p.primary_ink)
                                                .child("▶")
                                                .into_any_element()
                                        } else {
                                            icon(IconName::File, 18., p.ink_secondary)
                                                .into_any_element()
                                        }),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(14.))
                                                .text_color(p.ink)
                                                .child(file_name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.ink_subtitle)
                                                .font_features(tnum())
                                                .child(meta),
                                        ),
                                )
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    if let Some(u) = &url {
                                        if is_video {
                                            this.open_video_dialog(
                                                u.clone(),
                                                file_name.clone(),
                                                file_size,
                                                window,
                                                cx,
                                            );
                                        } else {
                                            cx.open_url(u);
                                        }
                                    }
                                }))
                        }))
                        .into_any_element()
                }
            }
            MediaTab::Links => {
                let links: Vec<(String, String)> = all
                    .iter()
                    .rev()
                    .flat_map(|m| {
                        format::links(m.text())
                            .into_iter()
                            .map(move |l| (l, m.created_at.clone()))
                    })
                    .collect();
                if links.is_empty() {
                    empty("ยังไม่มีลิงก์ในแชทนี้")
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .children(links.into_iter().enumerate().map(|(i, (url, at))| {
                            let host = url
                                .split("://")
                                .nth(1)
                                .unwrap_or(&url)
                                .split('/')
                                .next()
                                .unwrap_or("")
                                .to_string();
                            let open = url.clone();
                            div()
                                .id(("media-link", i))
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .px(px(8.))
                                .py(px(8.))
                                .rounded(px(8.))
                                .cursor_pointer()
                                .hover(move |s| s.bg(hover))
                                .child(
                                    div()
                                        .size(px(36.))
                                        .flex_none()
                                        .rounded(px(8.))
                                        .bg(p.surface_muted)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(icon(IconName::Link, 18., p.ink_secondary)),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(14.))
                                                .text_color(p.primary_ink)
                                                .child(url),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.ink_subtitle)
                                                .font_features(tnum())
                                                .child(format!(
                                                    "{host} · {}",
                                                    format::chat_time(Some(&at))
                                                )),
                                        ),
                                )
                                .on_click(move |_, _, cx| cx.open_url(&open))
                        }))
                        .into_any_element()
                }
            }
            MediaTab::Members => {
                let members = self
                    .store
                    .read(cx)
                    .selected()
                    .map(|c| c.participants())
                    .unwrap_or_default();
                let store = self.store.clone();
                let room = cx.entity().downgrade();
                if members.is_empty() {
                    div()
                        .py(px(48.))
                        .text_center()
                        .text_size(px(14.))
                        .text_color(p.ink_subtitle)
                        .child("ไม่พบข้อมูลสมาชิก")
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .children(members.into_iter().enumerate().map(|(i, m)| {
                            render_member_row(i, m, "panel", &store, room.clone(), cx, p)
                        }))
                        .into_any_element()
                }
            }
        };

        let tab_btn = |t: MediaTab, cx: &mut Context<Self>| {
            let active = t == tab;
            div()
                .id(t.label())
                .h(px(36.))
                .flex()
                .items_center()
                .border_b_2()
                .border_color(if active {
                    p.primary
                } else {
                    transparent_black()
                })
                .text_size(px(14.))
                .font_weight(if active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active {
                    p.primary_ink
                } else {
                    p.ink_secondary
                })
                .cursor_pointer()
                .child(t.label())
                .on_click(cx.listener(move |this, _, _, cx| this.open_panel(t, cx)))
        };

        let is_group = self
            .store
            .read(cx)
            .selected()
            .is_some_and(|c| c.chat_type == ChatType::Group);

        div()
            .w(px(320.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(p.surface)
            .border_l_1()
            .border_color(p.divider)
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .pl(px(16.))
                    .pr(px(8.))
                    .border_b_1()
                    .border_color(p.divider)
                    .when(is_group, |d| d.child(tab_btn(MediaTab::Members, cx)))
                    .child(tab_btn(MediaTab::Images, cx))
                    .child(tab_btn(MediaTab::Files, cx))
                    .child(tab_btn(MediaTab::Links, cx))
                    .child(div().flex_1())
                    .child(div().pb(px(4.)).child(
                        icon_button("close-panel", IconName::X, 28., p.ink_subtitle, p).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.panel = None;
                                cx.notify();
                            }),
                        ),
                    )),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("panel-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.panel_scroll)
                            .p(px(12.))
                            .child(body),
                    )
                    .vertical_scrollbar(&self.panel_scroll),
            )
            .into_any_element()
    }

    fn message_menu(
        this: WeakEntity<Self>,
        message_id: String,
        has_text: bool,
        pinned: bool,
        menu: PopupMenu,
    ) -> PopupMenu {
        let (a, b, c, d) = (this.clone(), this.clone(), this.clone(), this);
        let (ia, ib, ic, id) = (
            message_id.clone(),
            message_id.clone(),
            message_id.clone(),
            message_id,
        );
        menu.item(PopupMenuItem::new("ตอบกลับ").icon(IconName::Reply).on_click(
            move |_, window, cx| {
                a.update(cx, |v, cx| v.start_reply(&ia, window, cx)).ok();
            },
        ))
        .item(
            PopupMenuItem::new("คัดลอก")
                .icon(IconName::Copy)
                .disabled(!has_text)
                .on_click(move |_, _, cx| {
                    b.update(cx, |v, cx| v.copy(&ib, cx)).ok();
                }),
        )
        .item(
            PopupMenuItem::new(if pinned {
                "เลิกปักหมุด"
            } else {
                "ปักหมุด"
            })
            .icon(if pinned {
                IconName::PinOff
            } else {
                IconName::Pin
            })
            .on_click(move |_, _, cx| {
                d.update(cx, |v, cx| v.toggle_pin(&id, cx)).ok();
            }),
        )
        .separator()
        .item(
            PopupMenuItem::new("ลบ")
                .icon(IconName::Trash)
                .on_click(move |_, window, cx| {
                    let id = ic.clone();
                    c.update(cx, |v, cx| v.confirm_delete(id, window, cx)).ok();
                }),
        )
    }

    // ---- render ---------------------------------------------------------

    fn render_empty(&self, p: &Palette) -> impl IntoElement {
        div()
            .size_full()
            .bg(p.background)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .child(
                div()
                    .size(px(88.))
                    .mb(px(20.))
                    .rounded_full()
                    .bg(p.primary_tint)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img("images/logo.png").size(px(48.)).rounded(px(12.))),
            )
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.ink)
                    .child("Pinto Chat"),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(p.ink_subtitle)
                    .child("เลือกแชททางซ้ายเพื่อเริ่มคุย"),
            )
    }

    fn render_header(&self, narrow: bool, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let store = self.store.read(cx);
        let chat = store.selected().cloned();
        let connection = store.connection;
        let typing = chat.as_ref().is_some_and(|c| store.is_typing(&c.chat_id));
        let official = chat.as_ref().is_some_and(|c| store.is_chat_official(c));
        let partner_blocked = store
            .selected_partner_id()
            .as_deref()
            .is_some_and(|id| store.is_blocked(id));
        let image = chat
            .as_ref()
            .and_then(|c| c.avatar_url.clone())
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let muted = chat.as_ref().is_some_and(|c| c.is_muted);
        let name = chat
            .as_ref()
            .map(|c| c.display_name().to_string())
            .unwrap_or_default();
        let is_group = chat
            .as_ref()
            .is_some_and(|c| c.chat_type == ChatType::Group);
        let member_count = chat
            .as_ref()
            .filter(|c| c.chat_type == ChatType::Group)
            .map(|c| c.participants().len())
            .unwrap_or(0);
        let (sub, sub_color) = if connection != ConnectionState::Connected {
            ("ออฟไลน์ · ข้อความจะส่งเมื่อเชื่อมต่อได้".to_string(), p.ink_subtitle)
        } else if typing {
            ("กำลังพิมพ์…".to_string(), p.primary_ink)
        } else if partner_blocked {
            ("บล็อกผู้ใช้นี้อยู่".to_string(), p.error)
        } else if is_group {
            if member_count > 0 {
                (format!("กลุ่ม · {member_count} สมาชิก"), p.ink_subtitle)
            } else {
                ("กลุ่ม".to_string(), p.ink_subtitle)
            }
        } else {
            ("แชทส่วนตัว".to_string(), p.ink_subtitle)
        };
        let room = cx.entity().downgrade();

        div()
            .h(px(64.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(16.))
            .bg(p.surface)
            .border_b_1()
            .border_color(p.divider)
            .when(narrow, |d| {
                d.child(
                    icon_button("room-back", IconName::ChevronLeft, 32., p.ink_secondary, p)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.store.update(cx, |s, cx| s.select_chat(None, cx))
                        })),
                )
            })
            .child(
                div()
                    .id("header-chat-avatar")
                    .when(is_group, |d| {
                        d.cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_group_info_dialog(window, cx);
                            }))
                    })
                    .child(avatar(image, 40., p)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(16.))
                                    .line_height(px(24.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(name),
                            )
                            .when(official, |d| d.child(verified_badge(15., p)))
                            .when(muted, |d| {
                                d.child(icon(IconName::BellOff, 14., p.ink_subtitle))
                            }),
                    )
                    .child(
                        div()
                            .id("header-group-members")
                            .self_start()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(connection != ConnectionState::Connected, |d| {
                                d.child(div().size(px(6.)).rounded_full().bg(p.ink_disabled))
                            })
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(sub_color)
                                    .when(is_group, |d| {
                                        d.cursor_pointer().hover(|s| s.text_color(p.primary_ink))
                                    })
                                    .child(sub),
                            )
                            .when(is_group, |d| {
                                d.on_click(cx.listener(|this, _, window, cx| {
                                    this.open_members_dialog(window, cx);
                                }))
                            }),
                    ),
            )
            .child(drag_region(div()).flex_1().h_full())
            .child(
                Button::new("room-menu")
                    .ghost()
                    .icon(icon(IconName::EllipsisVertical, ICON, p.ink_secondary))
                    .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                        let pal = cx.palette();
                        let Some(view) = room.upgrade() else {
                            return menu;
                        };
                        let (muted, wallpaper, partner_id, blocked, is_group) = {
                            let s = view.read(cx).store.read(cx);
                            let pid = s.selected_partner_id();
                            let blk = pid.as_deref().is_some_and(|id| s.is_blocked(id));
                            let grp = s.selected().is_some_and(|c| c.chat_type == ChatType::Group);
                            (
                                s.selected().is_some_and(|c| c.is_muted),
                                s.settings.wallpaper,
                                pid,
                                blk,
                                grp,
                            )
                        };
                        // Every row drawn the same way, so icons and labels share one column.
                        let row_colored =
                            move |label: &'static str, name: Option<IconName>, danger: bool| {
                                let fg = if danger { pal.error_strong } else { pal.ink };
                                let ic = if danger {
                                    pal.error_strong
                                } else {
                                    pal.ink_secondary
                                };
                                PopupMenuItem::element(move |_, _| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_color(fg)
                                        .child(match name {
                                            Some(n) => icon(n, 16., ic).into_any_element(),
                                            None => div().size(px(16.)).into_any_element(),
                                        })
                                        .child(label)
                                })
                            };
                        let row = move |label: &'static str, name: Option<IconName>| {
                            row_colored(label, name, false)
                        };
                        let act = |f: fn(&mut RoomView, &mut Window, &mut Context<RoomView>)| {
                            let room = room.clone();
                            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                room.update(cx, |v, cx| f(v, window, cx)).ok();
                            }
                        };
                        menu.item(
                            row(
                                if muted {
                                    "เปิดเสียงแชท"
                                } else {
                                    "ปิดเสียงแชท"
                                },
                                Some(if muted {
                                    IconName::Bell
                                } else {
                                    IconName::BellOff
                                }),
                            )
                            .on_click(act(|v, _, cx| v.toggle_mute(cx))),
                        )
                        .when_some(partner_id, |m, pid| {
                            let room = room.clone();
                            m.item(
                                row_colored(
                                    if blocked {
                                        "ปลดบล็อกผู้ใช้"
                                    } else {
                                        "บล็อกผู้ใช้"
                                    },
                                    Some(if blocked {
                                        IconName::ShieldCheck
                                    } else {
                                        IconName::Ban
                                    }),
                                    !blocked,
                                )
                                .on_click(move |_, _, cx| {
                                    let pid = pid.clone();
                                    let _ = room.update(cx, |v, cx| {
                                        v.store.update(cx, |s, cx| {
                                            if blocked {
                                                s.unblock_user(pid, cx);
                                            } else {
                                                s.block_user(pid, cx);
                                            }
                                        });
                                    });
                                }),
                            )
                        })
                        .separator()
                        .when(is_group, |m| {
                            m.item(
                                row("สมาชิกในกลุ่ม", Some(IconName::Users))
                                    .on_click(act(|v, _, cx| v.open_panel(MediaTab::Members, cx))),
                            )
                        })
                        .item(
                            row("รูปภาพ", Some(IconName::Image))
                                .on_click(act(|v, _, cx| v.open_panel(MediaTab::Images, cx))),
                        )
                        .item(
                            row("ไฟล์", Some(IconName::File))
                                .on_click(act(|v, _, cx| v.open_panel(MediaTab::Files, cx))),
                        )
                        .item(
                            row("ลิงก์", Some(IconName::Link))
                                .on_click(act(|v, _, cx| v.open_panel(MediaTab::Links, cx))),
                        )
                        .separator()
                        .item(
                            row("บันทึกแชทเป็นไฟล์", Some(IconName::Download))
                                .on_click(act(|v, _, cx| v.export(cx))),
                        )
                        .separator()
                        .label("พื้นหลัง")
                        .item(
                            row(
                                "ลาย Pinto",
                                (wallpaper == Wallpaper::Pattern).then_some(IconName::Check),
                            )
                            .on_click(act(|v, _, cx| v.set_wallpaper(Wallpaper::Pattern, cx))),
                        )
                        .item(
                            row(
                                "เรียบ",
                                (wallpaper == Wallpaper::Plain).then_some(IconName::Check),
                            )
                            .on_click(act(|v, _, cx| v.set_wallpaper(Wallpaper::Plain, cx))),
                        )
                        .min_w(px(220.))
                    }),
            )
    }

    fn render_divider(label: String, p: &Palette) -> AnyElement {
        div()
            .w_full()
            .flex()
            .justify_center()
            .my(px(16.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(2.))
                    .rounded_full()
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.hairline)
                    .text_size(px(12.))
                    .text_color(p.ink_subtitle)
                    .child(label),
            )
            .into_any_element()
    }

    fn status_mark(&self, m: &Message, ink: Hsla) -> AnyElement {
        match m.status {
            MessageStatus::Queued | MessageStatus::Sending => {
                icon(IconName::Clock, 12., ink).into_any_element()
            }
            MessageStatus::Sent => icon(IconName::Check, 12., ink).into_any_element(),
            MessageStatus::Delivered => icon(IconName::CheckCheck, 12., ink).into_any_element(),
            MessageStatus::Read => icon(IconName::CheckCheck, 12., ink).into_any_element(),
            MessageStatus::Failed => icon(IconName::CircleAlert, 12., ink).into_any_element(),
        }
    }

    /// Shown under a bubble that failed to send, outside the green.
    fn retry_line(&self, m: &Message, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let id = m.message_id.clone();
        div()
            .id(SharedString::from(format!("retry-{id}")))
            .flex()
            .items_center()
            .gap(px(4.))
            .mt(px(4.))
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(p.error_strong)
            .hover(|s| s.underline())
            .child(icon(IconName::CircleAlert, 12., p.error_strong))
            .child("ส่งไม่สำเร็จ · แตะเพื่อลองอีกครั้ง")
            .on_click(cx.listener(move |this, _, _, cx| {
                let id = id.clone();
                this.store.update(cx, |s, cx| {
                    if let Some(chat) = s.selected_chat.clone() {
                        s.retry(&chat, &id, cx);
                    }
                });
            }))
            .into_any_element()
    }

    fn render_bubble(
        &self,
        m: &Message,
        compact: bool,
        show_meta: bool,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> AnyElement {
        let store = self.store.read(cx);
        let own = store.is_self(m);
        let pinned = store.is_pinned(&m.chat_id, &m.message_id);
        let sender_name = store.sender_name(m);
        let reply = m
            .reply_to
            .as_deref()
            .and_then(|r| store.reply_target(&m.chat_id, r));
        let reply_pending = m.reply_to.is_some() && reply.is_none();
        let group = store.chat_type(&m.chat_id) == Some(ChatType::Group);
        let sender_avatar = m.sender().avatar_url;
        let avatar_img = (!own && !compact)
            .then_some(sender_avatar)
            .flatten()
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let anim_badge = animated_badge(m);
        let is_visual_media = m.message_type == MessageType::Image
            || (m.message_type == MessageType::File && anim_badge.is_some());
        let media = is_visual_media
            .then(|| m.media_url.clone())
            .flatten()
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));

        let (bg, fg, meta) = if own {
            let ink: Hsla = rgb(ON_BUBBLE).into();
            (p.primary, ink, ink.opacity(0.7))
        } else {
            (p.bubble_other, p.ink, p.ink_subtitle)
        };
        let r = px(16.);
        let tail = px(6.);
        let unsent = m.unsent();
        let has_text = !unsent && !m.text().is_empty();
        let failed = m.status == MessageStatus::Failed;

        let mut bubble = div()
            .max_w_full()
            .px(px(14.))
            .py(px(10.))
            .bg(bg)
            .text_color(fg)
            .text_size(px(14.))
            .line_height(px(21.))
            .rounded(r)
            .when(!own && !p.dark, |d| d.border_1().border_color(p.hairline))
            .flex()
            .flex_col()
            .gap(px(4.));
        bubble = if own {
            bubble
                .rounded_br(tail)
                .when(compact, |d| d.rounded_tr(tail))
        } else {
            bubble
                .rounded_bl(tail)
                .when(compact, |d| d.rounded_tl(tail))
        };

        if !own && !compact && group {
            bubble = bubble.child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.primary_ink)
                    .child(sender_name),
            );
        }
        if reply.is_some() || reply_pending {
            let (who, what) = reply
                .map(|r| (r.sender_name, format::truncate(&r.content, 60)))
                .unwrap_or_else(|| ("ข้อความที่ตอบกลับ".into(), "ไม่พบข้อความต้นฉบับในเครื่องนี้".into()));
            bubble = bubble.child(
                div()
                    .pl(px(8.))
                    .border_l_2()
                    .border_color(if own { fg.opacity(0.4) } else { p.primary })
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(who))
                    .child(div().truncate().text_color(meta).child(what)),
            );
        }
        if is_visual_media {
            let click_url = m.media_url.clone();
            let click_name = m.file_name.clone();
            let media_id = SharedString::from(format!("msg-img-{}", m.message_id));
            bubble = bubble.child(
                div()
                    .id(media_id)
                    .when(click_url.is_some(), |d| d.cursor_pointer())
                    .child(match media {
                        Some(img_data) => div()
                            .relative()
                            .child(
                                img(img_data)
                                    .max_w(px(280.))
                                    .max_h(px(320.))
                                    .rounded(px(8.)),
                            )
                            .when_some(anim_badge, |d, label| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top(px(6.))
                                        .left(px(6.))
                                        .px(px(6.))
                                        .py(px(2.))
                                        .rounded(px(6.))
                                        .bg(black().opacity(0.55))
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(white())
                                        .child(label),
                                )
                            })
                            .into_any_element(),
                        None => div()
                            .w(px(220.))
                            .h(px(140.))
                            .rounded(px(8.))
                            .bg(if own {
                                fg.opacity(0.1)
                            } else {
                                p.surface_muted
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(IconName::Image, 24., meta))
                            .into_any_element(),
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(u) = &click_url {
                            this.open_image_dialog(
                                u.clone(),
                                click_name.clone(),
                                anim_badge,
                                window,
                                cx,
                            );
                        }
                    })),
            );
        } else if is_video_message(m) {
            let click_url = m.media_url.clone();
            let vname = m.file_name.clone().unwrap_or_else(|| "วิดีโอ".into());
            let vsize = m.file_size;
            let video_id = SharedString::from(format!("msg-vid-{}", m.message_id));
            bubble = bubble.child(
                div()
                    .id(video_id)
                    .w(px(240.))
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(if own {
                        fg.opacity(0.1)
                    } else {
                        p.surface_muted
                    })
                    .when(click_url.is_some(), |d| d.cursor_pointer())
                    .child(
                        div()
                            .h(px(120.))
                            .relative()
                            .bg(black().opacity(0.35))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .size(px(44.))
                                    .rounded_full()
                                    .bg(black().opacity(0.6))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(16.))
                                    .text_color(white())
                                    .child("▶"),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top(px(8.))
                                    .left(px(8.))
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(6.))
                                    .bg(black().opacity(0.55))
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(white())
                                    .child("วิดีโอ"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .child(vname.clone()),
                            )
                            .when_some(vsize, |d, s| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(11.))
                                        .text_color(meta)
                                        .child(format::file_size(s)),
                                )
                            }),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(u) = &click_url {
                            this.open_video_dialog(u.clone(), vname.clone(), vsize, window, cx);
                        }
                    })),
            );
        } else if m.message_type == MessageType::File {
            let click_url = m.media_url.clone();
            let file_id = SharedString::from(format!("msg-file-{}", m.message_id));
            bubble = bubble.child(
                div()
                    .id(file_id)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .p(px(8.))
                    .rounded(px(8.))
                    .bg(if own {
                        fg.opacity(0.08)
                    } else {
                        p.surface_muted
                    })
                    .when(click_url.is_some(), |d| d.cursor_pointer())
                    .child(icon(IconName::File, 16., fg))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(m.file_name.clone().unwrap_or_else(|| "ไฟล์".into())),
                    )
                    .when_some(m.file_size, |d, s| {
                        d.child(
                            div()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(meta)
                                .child(format::file_size(s)),
                        )
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(u) = &click_url {
                            cx.open_url(u);
                        }
                    }),
            );
        }
        if unsent {
            bubble = bubble.child(div().text_color(meta).child("ยกเลิกข้อความแล้ว"));
        } else if has_text {
            bubble = bubble.child(div().child(m.text().to_string()));
        }
        // Time and delivery status once, at the end of a run; failures always show.
        if show_meta || m.status == MessageStatus::Failed {
            bubble = bubble.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(4.))
                    .text_size(px(11.))
                    .line_height(px(14.))
                    .text_color(meta)
                    .font_features(tnum())
                    .when(m.is_edited && !unsent, |d| d.child("แก้ไขแล้ว ·"))
                    .child(format::time(&m.created_at))
                    .when(own, |d| d.child(self.status_mark(m, meta))),
            );
        }

        let room = cx.entity().downgrade();
        let id = m.message_id.clone();
        let menu_room = room.clone();
        let menu_id = id.clone();

        div()
            .id(SharedString::from(format!("msg-{}", m.message_id)))
            .group(SharedString::from(format!("g-{}", m.message_id)))
            .flex()
            .items_start()
            .gap(px(8.))
            .mt(px(if compact { 2. } else { 12. }))
            .when(own, |d| d.flex_row_reverse())
            .when(!own, |d| {
                d.child(if compact {
                    div().w(px(32.)).flex_none().into_any_element()
                } else {
                    avatar(avatar_img, 32., p)
                })
            })
            // The "…" button sits in the bubble's own row, a fixed 8px from it;
            // the retry line goes underneath and never pushes the button away.
            .child(
                div()
                    .max_w(relative(0.66))
                    .flex()
                    .flex_col()
                    .when(own, |d| d.items_end())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .when(own, |d| d.flex_row_reverse())
                            .child(bubble)
                            .child(
                                div()
                                    .flex_none()
                                    .invisible()
                                    .group_hover(
                                        SharedString::from(format!("g-{}", m.message_id)),
                                        |s| s.visible(),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "more-{}",
                                            m.message_id
                                        )))
                                        .outline()
                                        .xsmall()
                                        .rounded(px(12.))
                                        .icon(icon(IconName::Ellipsis, 14., p.ink_secondary))
                                        .dropdown_menu(
                                            move |menu, _, _| {
                                                Self::message_menu(
                                                    menu_room.clone(),
                                                    menu_id.clone(),
                                                    has_text,
                                                    pinned,
                                                    menu,
                                                )
                                            },
                                        ),
                                    ),
                            ),
                    )
                    .when(failed, |d| d.child(self.retry_line(m, cx, p))),
            )
            .context_menu(move |menu, _, _| {
                Self::message_menu(room.clone(), id.clone(), has_text, pinned, menu)
            })
            .into_any_element()
    }

    fn render_messages(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> AnyElement {
        let store = self.store.read(cx);
        let Some(chat_id) = store.selected_chat.clone() else {
            return div().into_any_element();
        };
        let messages: Vec<Message> = store.messages(&chat_id).to_vec();
        let (loading, loading_more, has_more) = (
            store.messages_loading,
            store.messages_loading_more,
            store.messages_has_more,
        );

        if loading && messages.is_empty() {
            return div()
                .p(px(24.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .children((0..6).map(|i| {
                    let own = i % 3 == 2;
                    div().flex().when(own, |d| d.justify_end()).child(
                        Skeleton::new()
                            .h(px(40.))
                            .w(px(if own { 160. } else { 220. }))
                            .rounded(px(16.)),
                    )
                }))
                .into_any_element();
        }
        if messages.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .px(px(20.))
                        .py(px(16.))
                        .rounded(px(16.))
                        .bg(p.surface)
                        .border_1()
                        .border_color(p.hairline)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.ink)
                                .child("ยังไม่มีข้อความ"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.ink_secondary)
                                .child("พิมพ์ข้อความแรกด้านล่าง"),
                        ),
                )
                .into_any_element();
        }

        // Keep the reading position after older rows were prepended.
        if let Some(prev) = self.anchor {
            let now = self.scroll.max_offset().y;
            if now != prev {
                let off = self.scroll.offset();
                self.scroll.set_offset(point(off.x, off.y - (now - prev)));
                self.anchor = None;
            }
        }
        // Near the top: pull the previous page from SQLite.
        if has_more
            && !loading_more
            && self.anchor.is_none()
            && self.scroll.max_offset().y > px(0.)
            && self.scroll.offset().y > px(-80.)
        {
            self.anchor = Some(self.scroll.max_offset().y);
            let store = self.store.clone();
            let id = chat_id.clone();
            cx.defer(move |cx| store.update(cx, |s, cx| s.load_older(&id, cx)));
            let handle = cx.entity().downgrade();
            window.on_next_frame(move |_, cx| {
                handle.update(cx, |_, cx| cx.notify()).ok();
            });
        }
        self.stick_to_bottom = self.distance_from_bottom() < px(24.);

        let mut rows: Vec<AnyElement> = Vec::with_capacity(messages.len() + 8);
        // A run is consecutive messages from one sender, same day, under 5 minutes apart.
        let same_run = |a: &Message, b: &Message| {
            a.sender_id == b.sender_id
                && format::same_day(&a.created_at, &b.created_at)
                && (crate::models::parse_time(&b.created_at)
                    - crate::models::parse_time(&a.created_at))
                .num_minutes()
                .abs()
                    < 5
        };
        let mut prev: Option<&Message> = None;
        for (i, m) in messages.iter().enumerate() {
            let new_day = prev.is_none_or(|x| !format::same_day(&x.created_at, &m.created_at));
            if new_day {
                rows.push(Self::render_divider(format::date_divider(&m.created_at), p));
            }
            let compact = !new_day && prev.is_some_and(|x| same_run(x, m));
            let last_in_run = messages.get(i + 1).is_none_or(|next| !same_run(m, next));
            rows.push(self.render_bubble(m, compact, last_in_run, cx, p));
            prev = Some(m);
        }

        div()
            .id(SharedString::from(format!("messages-{chat_id}")))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .px(px(24.))
            .pb(px(16.))
            .children(rows)
            .into_any_element()
    }

    fn insert_emoji(&mut self, emoji: &str, window: &mut Window, cx: &mut Context<Self>) {
        record_recent(&mut self.emoji_prefs.recent, emoji);
        if !(cfg!(debug_assertions) && std::env::var("PINTO_PREVIEW").is_ok()) {
            self.emoji_prefs.save();
        }
        let cur = self.composer.read(cx).value().to_string();
        let next = format!("{cur}{emoji}");
        if next.chars().count() <= MAX_MESSAGE_LENGTH {
            self.composer
                .update(cx, |s, cx| s.set_value(next, window, cx));
        }
        self.composer.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn open_group_info_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(chat) = self.store.read(cx).selected().cloned() else {
            return;
        };
        if chat.chat_type != ChatType::Group {
            return;
        }
        let chat_id = chat.chat_id.clone();
        self.group_info_scroll.set_offset(point(px(0.), px(0.)));
        self.group_name_input.update(cx, |s, cx| {
            s.set_value(chat.display_name().to_string(), window, cx);
        });
        self.store.update(cx, |s, cx| {
            s.load_friends(false, cx);
            s.load_chat_detail(&chat_id, cx);
        });

        let store = self.store.clone();
        let room = cx.entity().downgrade();
        let name_input = self.group_name_input.clone();
        let scroll = self.group_info_scroll.clone();

        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let (chat, my_role, me_id, friends) = {
                let s = store.read(cx);
                let c = s
                    .selected()
                    .filter(|c| c.chat_id == chat_id)
                    .cloned()
                    .unwrap_or_else(|| chat.clone());
                let role = s.my_group_role(&c);
                let me = s
                    .user
                    .as_ref()
                    .map(|u| u.user_id.clone())
                    .unwrap_or_default();
                let friends = s.friends.clone();
                (c, role, me, friends)
            };
            let is_owner = my_role == "owner";
            let is_admin = my_role == "admin";
            let can_manage_members = is_owner || is_admin;
            let role_label = match my_role.as_str() {
                "owner" => "เจ้าของกลุ่ม",
                "admin" => "แอดมิน",
                _ => "สมาชิก",
            };
            let avatar_img = chat
                .avatar_url
                .clone()
                .and_then(|u| store.update(cx, |s, cx| s.image(&u, cx)));
            let members = chat.participants();
            let member_ids: std::collections::HashSet<String> =
                members.iter().map(|m| m.user_id.clone()).collect();
            let addable_friends: Vec<UserProfile> = friends
                .into_iter()
                .filter(|f| !member_ids.contains(&f.user_id))
                .take(20)
                .collect();

            let store_avatar = store.clone();
            let cid_avatar = chat_id.clone();
            let store_rename = store.clone();
            let cid_rename = chat_id.clone();
            let input_rename = name_input.clone();
            let store_invite = store.clone();
            let cid_invite = chat_id.clone();
            let store_leave = store.clone();
            let cid_leave = chat_id.clone();
            let me_leave = me_id.clone();

            let chip_btn = |id: SharedString, label: &'static str, danger: bool| {
                div()
                    .id(id)
                    .flex_none()
                    .px(px(8.))
                    .py(px(3.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if danger {
                        p.error.opacity(0.4)
                    } else {
                        p.hairline
                    })
                    .bg(p.surface_muted)
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(if danger { p.error_strong } else { p.ink })
                    .hover(|s| s.bg(p.divider))
                    .child(label)
            };

            dialog.title("ข้อมูลกลุ่ม").w(px(460.)).footer(div()).child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(14.))
                            .child(avatar(avatar_img, 64., &p))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(div().flex_1().child(Input::new(&name_input)))
                                            .child(
                                                div()
                                                    .id("group-save-name")
                                                    .px(px(12.))
                                                    .h(px(34.))
                                                    .rounded(px(8.))
                                                    .bg(p.primary)
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(rgb(ON_BUBBLE))
                                                    .hover(|s| s.bg(p.primary_hover))
                                                    .child("บันทึกชื่อ")
                                                    .on_click(move |_, _, cx| {
                                                        let val = input_rename
                                                            .read(cx)
                                                            .value()
                                                            .to_string();
                                                        store_rename.update(cx, |s, cx| {
                                                            s.rename_group(
                                                                cid_rename.clone(),
                                                                val,
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .id("group-change-avatar")
                                                    .px(px(10.))
                                                    .py(px(4.))
                                                    .rounded(px(6.))
                                                    .border_1()
                                                    .border_color(p.hairline)
                                                    .bg(p.surface_muted)
                                                    .cursor_pointer()
                                                    .text_size(px(12.))
                                                    .text_color(p.ink)
                                                    .hover(|s| s.bg(p.divider))
                                                    .child("เปลี่ยนรูปกลุ่ม")
                                                    .on_click(move |_, _, cx| {
                                                        let paths = cx.prompt_for_paths(
                                                            PathPromptOptions {
                                                                files: true,
                                                                directories: false,
                                                                multiple: false,
                                                                prompt: Some(
                                                                    "เลือกรูปโปรไฟล์กลุ่ม".into(),
                                                                ),
                                                            },
                                                        );
                                                        let st = store_avatar.clone();
                                                        let cid = cid_avatar.clone();
                                                        cx.spawn(async move |cx| {
                                                            let Ok(Ok(Some(paths))) = paths.await
                                                            else {
                                                                return;
                                                            };
                                                            let Some(path) =
                                                                paths.into_iter().next()
                                                            else {
                                                                return;
                                                            };
                                                            st.update(cx, |s, cx| {
                                                                s.update_group_avatar(
                                                                    cid, path, cx,
                                                                );
                                                            });
                                                        })
                                                        .detach();
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .id("group-copy-invite")
                                                    .px(px(10.))
                                                    .py(px(4.))
                                                    .rounded(px(6.))
                                                    .border_1()
                                                    .border_color(p.hairline)
                                                    .bg(p.surface_muted)
                                                    .cursor_pointer()
                                                    .text_size(px(12.))
                                                    .text_color(p.ink)
                                                    .hover(|s| s.bg(p.divider))
                                                    .child("คัดลอกลิงก์เชิญ")
                                                    .on_click(move |_, _, cx| {
                                                        store_invite.update(cx, |s, cx| {
                                                            s.copy_group_invite(
                                                                cid_invite.clone(),
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(10.))
                            .py(px(6.))
                            .rounded(px(8.))
                            .bg(p.surface_muted)
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.ink_secondary)
                                    .child(format!("สมาชิก {} / 100 คน", members.len())),
                            )
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(6.))
                                    .bg(p.primary_tint)
                                    .text_size(px(11.))
                                    .text_color(p.primary_ink)
                                    .child(format!("สิทธิ์ของคุณ: {role_label}")),
                            ),
                    )
                    .when(can_manage_members && !addable_friends.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink_secondary)
                                        .child("เพิ่มสมาชิก (คลิกเพื่อเชิญเข้ากลุ่ม)"),
                                )
                                .child(div().flex().flex_wrap().gap(px(6.)).children(
                                    addable_friends.into_iter().enumerate().map(|(i, f)| {
                                        let st = store.clone();
                                        let cid = chat_id.clone();
                                        let uid = f.user_id.clone();
                                        div()
                                            .id(SharedString::from(format!(
                                                "add-friend-{i}-{}",
                                                f.user_id
                                            )))
                                            .px(px(8.))
                                            .py(px(4.))
                                            .rounded(px(8.))
                                            .border_1()
                                            .border_color(p.hairline)
                                            .bg(p.surface_muted)
                                            .cursor_pointer()
                                            .text_size(px(12.))
                                            .text_color(p.ink)
                                            .hover(|s| s.bg(p.primary_tint))
                                            .child(format!("+ {}", f.display_name()))
                                            .on_click(move |_, _, cx| {
                                                st.update(cx, |s, cx| {
                                                    s.add_group_participants(
                                                        cid.clone(),
                                                        vec![uid.clone()],
                                                        cx,
                                                    );
                                                });
                                            })
                                    }),
                                )),
                        )
                    })
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(240.))
                            .child(
                                div()
                                    .id("group-info-members-scroll")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&scroll)
                                    .pr(px(12.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .children(members.into_iter().enumerate().map(|(i, m)| {
                                        let m_img = m
                                            .avatar_url
                                            .as_deref()
                                            .and_then(|u| store.update(cx, |s, cx| s.image(u, cx)));
                                        let m_role = match m.role.as_str() {
                                            "owner" => Some("เจ้าของกลุ่ม"),
                                            "admin" => Some("แอดมิน"),
                                            _ => None,
                                        };
                                        let is_me = m.user_id == me_id;
                                        let can_kick = !is_me
                                            && (is_owner || (is_admin && m.role == "member"));
                                        let can_role = is_owner && !is_me;
                                        let click_member = m.clone();
                                        let room_click = room.clone();

                                        let uid_role = m.user_id.clone();
                                        let next_role =
                                            if m.role == "admin" { "member" } else { "admin" };
                                        let role_btn_text = if m.role == "admin" {
                                            "ลดสิทธิ์"
                                        } else {
                                            "ตั้งแอดมิน"
                                        };
                                        let st_role = store.clone();
                                        let cid_role = chat_id.clone();

                                        let uid_owner = m.user_id.clone();
                                        let st_owner = store.clone();
                                        let cid_owner = chat_id.clone();

                                        let uid_kick = m.user_id.clone();
                                        let st_kick = store.clone();
                                        let cid_kick = chat_id.clone();

                                        div()
                                            .id(SharedString::from(format!(
                                                "ginfo-member-{i}-{}",
                                                m.user_id
                                            )))
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .p(px(6.))
                                            .rounded(px(8.))
                                            .hover(|s| s.bg(p.surface_muted))
                                            .child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "ginfo-open-{i}-{}",
                                                        m.user_id
                                                    )))
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(8.))
                                                    .cursor_pointer()
                                                    .child(avatar(m_img, 32., &p))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .flex()
                                                            .flex_col()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap(px(4.))
                                                                    .child(
                                                                        div()
                                                                            .truncate()
                                                                            .text_size(px(13.))
                                                                            .font_weight(
                                                                                FontWeight::MEDIUM,
                                                                            )
                                                                            .text_color(p.ink)
                                                                            .child(m.name),
                                                                    )
                                                                    .when(m.is_official, |d| {
                                                                        d.child(verified_badge(
                                                                            12., &p,
                                                                        ))
                                                                    }),
                                                            )
                                                            .when_some(m_role, |d, rl| {
                                                                d.child(
                                                                    div()
                                                                        .text_size(px(11.))
                                                                        .text_color(p.primary_ink)
                                                                        .child(rl),
                                                                )
                                                            }),
                                                    )
                                                    .on_click(move |_, window, cx| {
                                                        let target = click_member.clone();
                                                        let _ =
                                                            room_click.update(cx, |this, cx| {
                                                                this.open_profile_dialog(
                                                                    target, window, cx,
                                                                );
                                                            });
                                                    }),
                                            )
                                            .when(can_role, |d| {
                                                d.child(
                                                    chip_btn(
                                                        SharedString::from(format!(
                                                            "ginfo-role-{i}"
                                                        )),
                                                        role_btn_text,
                                                        false,
                                                    )
                                                    .on_click(move |_, _, cx| {
                                                        st_role.update(cx, |s, cx| {
                                                            s.update_participant_role(
                                                                cid_role.clone(),
                                                                uid_role.clone(),
                                                                next_role.to_string(),
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                                )
                                                .child(
                                                    chip_btn(
                                                        SharedString::from(format!(
                                                            "ginfo-owner-{i}"
                                                        )),
                                                        "โอนเจ้าของ",
                                                        false,
                                                    )
                                                    .on_click(move |_, _, cx| {
                                                        st_owner.update(cx, |s, cx| {
                                                            s.transfer_group_ownership(
                                                                cid_owner.clone(),
                                                                uid_owner.clone(),
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                                )
                                            })
                                            .when(can_kick, |d| {
                                                d.child(
                                                    chip_btn(
                                                        SharedString::from(format!(
                                                            "ginfo-kick-{i}"
                                                        )),
                                                        "ลบ",
                                                        true,
                                                    )
                                                    .on_click(move |_, _, cx| {
                                                        st_kick.update(cx, |s, cx| {
                                                            s.remove_group_participant(
                                                                cid_kick.clone(),
                                                                uid_kick.clone(),
                                                                false,
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                                )
                                            })
                                    })),
                            )
                            .vertical_scrollbar(&scroll),
                    )
                    .child(if is_owner {
                        div()
                            .text_size(px(12.))
                            .text_color(p.ink_subtitle)
                            .text_center()
                            .child("ต้องโอนสิทธิ์เจ้าของกลุ่มให้สมาชิกอื่นก่อนจึงจะออกจากกลุ่มได้")
                            .into_any_element()
                    } else {
                        div()
                            .id("group-leave-btn")
                            .w_full()
                            .h(px(34.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(p.error.opacity(0.35))
                            .bg(p.surface_muted)
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(13.))
                            .text_color(p.error_strong)
                            .hover(|s| s.bg(p.divider))
                            .child("ออกจากกลุ่ม")
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                store_leave.update(cx, |s, cx| {
                                    s.remove_group_participant(
                                        cid_leave.clone(),
                                        me_leave.clone(),
                                        true,
                                        cx,
                                    );
                                });
                            })
                            .into_any_element()
                    }),
            )
        });
    }

    fn open_members_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.members_dialog_scroll.set_offset(point(px(0.), px(0.)));
        let store = self.store.clone();
        let room = cx.entity().downgrade();
        let scroll = self.members_dialog_scroll.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let members = store
                .read(cx)
                .selected()
                .map(|c| c.participants())
                .unwrap_or_default();
            let title = if members.is_empty() {
                "สมาชิกกลุ่ม".to_string()
            } else {
                format!("สมาชิกกลุ่ม ({})", members.len())
            };
            let body: AnyElement = if members.is_empty() {
                div()
                    .py(px(36.))
                    .text_center()
                    .text_size(px(14.))
                    .text_color(p.ink_subtitle)
                    .child("ไม่พบข้อมูลสมาชิก")
                    .into_any_element()
            } else {
                div()
                    .relative()
                    .w_full()
                    .h(px(400.))
                    .child(
                        div()
                            .id("members-dialog-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .pr(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .children(members.into_iter().enumerate().map(|(i, m)| {
                                render_member_row(i, m, "dialog", &store, room.clone(), cx, &p)
                            })),
                    )
                    .vertical_scrollbar(&scroll)
                    .into_any_element()
            };
            dialog.title(title).w(px(380.)).footer(div()).child(body)
        });
    }

    fn open_profile_dialog(
        &mut self,
        member: ChatParticipant,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let room = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let img = member
                .avatar_url
                .as_deref()
                .and_then(|u| store.update(cx, |s, cx| s.image(u, cx)));
            let (is_self, blocked) = {
                let s = store.read(cx);
                (
                    s.user.as_ref().is_some_and(|u| u.user_id == member.user_id),
                    s.is_blocked(&member.user_id),
                )
            };
            let role_label = match member.role.as_str() {
                "owner" => "เจ้าของกลุ่ม",
                "admin" => "แอดมิน",
                _ => "สมาชิก",
            };
            let profile = UserProfile {
                user_id: member.user_id.clone(),
                username: member.username.clone(),
                name: member.name.clone(),
                profile_image: member.avatar_url.clone(),
                is_official: member.is_official,
            };
            let uid = member.user_id.clone();
            let store_msg = store.clone();
            let store_blk = store.clone();
            let room_msg = room.clone();

            dialog.w(px(340.)).footer(div()).child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.))
                    .pb(px(4.))
                    .child(avatar(img, 72., &p))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.ink)
                                    .child(member.name.clone()),
                            )
                            .when(member.is_official, |d| d.child(verified_badge(16., &p))),
                    )
                    .when(!member.username.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.ink_subtitle)
                                .child(format!("@{}", member.username)),
                        )
                    })
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded(px(8.))
                            .bg(p.primary_tint)
                            .text_size(px(12.))
                            .text_color(p.primary_ink)
                            .child(role_label),
                    )
                    .when(!is_self, |d| {
                        d.child(
                            div()
                                .w_full()
                                .pt(px(8.))
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .id("profile-send-msg")
                                        .w_full()
                                        .h(px(36.))
                                        .rounded(px(10.))
                                        .bg(p.primary)
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(6.))
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(ON_BUBBLE))
                                        .hover(|s| s.bg(p.primary_hover))
                                        .child(icon(
                                            IconName::MessageCircle,
                                            16.,
                                            rgb(ON_BUBBLE).into(),
                                        ))
                                        .child("ส่งข้อความ")
                                        .on_click(move |_, window, cx| {
                                            window.close_dialog(cx);
                                            let _ = room_msg.update(cx, |r, cx| {
                                                r.panel = None;
                                                cx.notify();
                                            });
                                            store_msg.update(cx, |s, cx| {
                                                s.open_direct(&profile, cx);
                                            });
                                        }),
                                )
                                .child(
                                    div()
                                        .id("profile-toggle-block")
                                        .w_full()
                                        .h(px(34.))
                                        .rounded(px(10.))
                                        .border_1()
                                        .border_color(p.hairline)
                                        .bg(p.surface_muted)
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(6.))
                                        .text_size(px(13.))
                                        .text_color(if blocked { p.ink } else { p.error_strong })
                                        .hover(|s| s.bg(p.divider))
                                        .child(icon(
                                            if blocked {
                                                IconName::ShieldCheck
                                            } else {
                                                IconName::Ban
                                            },
                                            15.,
                                            if blocked { p.ink } else { p.error_strong },
                                        ))
                                        .child(if blocked {
                                            "ปลดบล็อกผู้ใช้"
                                        } else {
                                            "บล็อกผู้ใช้"
                                        })
                                        .on_click(move |_, _, cx| {
                                            let id = uid.clone();
                                            store_blk.update(cx, |s, cx| {
                                                if blocked {
                                                    s.unblock_user(id, cx);
                                                } else {
                                                    s.block_user(id, cx);
                                                }
                                            });
                                        }),
                                ),
                        )
                    }),
            )
        });
    }

    fn open_image_dialog(
        &mut self,
        url: String,
        title: Option<String>,
        badge: Option<&'static str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let img_data = store.update(cx, |s, cx| s.image(&url, cx));
            let open_url = url.clone();
            let heading = title.clone().unwrap_or_else(|| "รูปภาพ".into());
            dialog.title(heading).w(px(640.)).footer(div()).child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(match img_data {
                        Some(data) => div()
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img(data)
                                    .max_w(px(590.))
                                    .max_h(px(460.))
                                    .object_fit(ObjectFit::Contain)
                                    .rounded(px(10.)),
                            )
                            .when_some(badge, |d, label| {
                                d.child(
                                    div()
                                        .absolute()
                                        .top(px(8.))
                                        .left(px(8.))
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(6.))
                                        .bg(black().opacity(0.55))
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(white())
                                        .child(label),
                                )
                            })
                            .into_any_element(),
                        None => div()
                            .w(px(360.))
                            .h(px(240.))
                            .rounded(px(10.))
                            .bg(p.surface_muted)
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(8.))
                            .child(icon(IconName::Image, 28., p.ink_disabled))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.ink_subtitle)
                                    .child("กำลังโหลดรูปภาพ..."),
                            )
                            .into_any_element(),
                    })
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("media-open-browser")
                                    .px(px(14.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(p.hairline)
                                    .bg(p.surface_muted)
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .hover(|s| s.bg(p.divider))
                                    .child(icon(IconName::Download, 14., p.ink_secondary))
                                    .child("เปิดในเบราว์เซอร์")
                                    .on_click(move |_, _, cx| cx.open_url(&open_url)),
                            )
                            .child(
                                div()
                                    .id("media-dialog-close")
                                    .px(px(14.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .bg(p.primary)
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(ON_BUBBLE))
                                    .hover(|s| s.bg(p.primary_hover))
                                    .child("ปิด")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            ),
                    ),
            )
        });
    }

    fn open_video_dialog(
        &mut self,
        url: String,
        name: String,
        size: Option<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _, cx| {
            let p = cx.palette();
            let play_url = url.clone();
            let btn_url = url.clone();
            let title = name.clone();
            dialog.title("วิดีโอ").w(px(460.)).footer(div()).child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .id("video-preview-play")
                            .w_full()
                            .h(px(220.))
                            .rounded(px(12.))
                            .bg(p.surface_muted)
                            .border_1()
                            .border_color(p.hairline)
                            .cursor_pointer()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(10.))
                            .hover(|s| s.bg(p.divider))
                            .child(
                                div()
                                    .size(px(56.))
                                    .rounded_full()
                                    .bg(p.primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(20.))
                                    .text_color(rgb(ON_BUBBLE))
                                    .child("▶"),
                            )
                            .child(
                                div()
                                    .px(px(16.))
                                    .truncate()
                                    .max_w_full()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(title.clone()),
                            )
                            .when_some(size, |d, sz| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.ink_subtitle)
                                        .child(format::file_size(sz)),
                                )
                            })
                            .on_click(move |_, _, cx| cx.open_url(&play_url)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("video-open-external")
                                    .px(px(14.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .bg(p.primary)
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(ON_BUBBLE))
                                    .hover(|s| s.bg(p.primary_hover))
                                    .child("เปิดเล่นวิดีโอ")
                                    .on_click(move |_, _, cx| cx.open_url(&btn_url)),
                            )
                            .child(
                                div()
                                    .id("video-dialog-close")
                                    .px(px(14.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(p.hairline)
                                    .bg(p.surface_muted)
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .hover(|s| s.bg(p.divider))
                                    .child("ปิด")
                                    .on_click(|_, window, cx| window.close_dialog(cx)),
                            ),
                    ),
            )
        });
    }

    fn render_composer(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let has_text = !self.composer.read(cx).value().trim().is_empty();
        let room_tab = cx.entity().downgrade();
        let room_pick = cx.entity().downgrade();
        let room_clear = cx.entity().downgrade();
        let room_tone = cx.entity().downgrade();
        let room_clear_search = cx.entity().downgrade();
        div()
            .flex_none()
            .relative()
            .px(px(24.))
            .pb(px(16.))
            .pt(px(8.))
            .when(self.emoji_open, |d| {
                d.child(
                    div()
                        .absolute()
                        .bottom(px(64.))
                        .left(px(24.))
                        .child(render_emoji_panel(
                            self.emoji_tab,
                            &self.emoji_prefs,
                            self.emoji_search.read(cx),
                            &self.emoji_search,
                            p,
                            EmojiPanelCallbacks {
                                on_tab: move |cat: EmojiCategory, _: &mut Window, cx: &mut App| {
                                    if let Some(v) = room_tab.upgrade() {
                                        v.update(cx, |this, cx| {
                                            this.emoji_tab = cat;
                                            cx.notify();
                                        });
                                    }
                                },
                                on_pick: move |emoji: String, window: &mut Window, cx: &mut App| {
                                    if let Some(v) = room_pick.upgrade() {
                                        v.update(cx, |this, cx| {
                                            this.insert_emoji(&emoji, window, cx);
                                        });
                                    }
                                },
                                on_clear_recent: move |_: &mut Window, cx: &mut App| {
                                    if let Some(v) = room_clear.upgrade() {
                                        v.update(cx, |this, cx| {
                                            this.emoji_prefs.recent.clear();
                                            if !(cfg!(debug_assertions)
                                                && std::env::var("PINTO_PREVIEW").is_ok())
                                            {
                                                this.emoji_prefs.save();
                                            }
                                            cx.notify();
                                        });
                                    }
                                },
                                on_skin_tone:
                                    move |tone: super::emoji::SkinTone,
                                          _: &mut Window,
                                          cx: &mut App| {
                                        if let Some(v) = room_tone.upgrade() {
                                            v.update(cx, |this, cx| {
                                                this.emoji_prefs.skin_tone = tone;
                                                if !(cfg!(debug_assertions)
                                                    && std::env::var("PINTO_PREVIEW").is_ok())
                                                {
                                                    this.emoji_prefs.save();
                                                }
                                                cx.notify();
                                            });
                                        }
                                    },
                                on_clear_search: move |window: &mut Window, cx: &mut App| {
                                    if let Some(v) = room_clear_search.upgrade() {
                                        v.update(cx, |this, cx| {
                                            this.emoji_search
                                                .update(cx, |s, cx| s.set_value("", window, cx));
                                            cx.notify();
                                        });
                                    }
                                },
                            },
                        ))
                        .with_animation(
                            "emoji-panel-enter",
                            Animation::new(Duration::from_millis(140))
                                .with_easing(ease_out_quint()),
                            |el, delta| el.opacity(delta).mb(px(6. * delta)),
                        ),
                )
            })
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.))
            .when_some(self.reply.clone(), |d, r| {
                d.child(
                    div()
                        .w_full()
                        .max_w(px(768.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(10.))
                        .bg(p.surface)
                        .border_1()
                        .border_color(p.hairline)
                        .child(div().w(px(2.)).h(px(32.)).rounded_full().bg(p.primary))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.primary_ink)
                                        .child(format!("ตอบกลับ {}", r.sender_name)),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .text_color(p.ink_subtitle)
                                        .child(r.content),
                                ),
                        )
                        .child(
                            icon_button("cancel-reply", IconName::X, 28., p.ink_subtitle, p)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.reply = None;
                                    cx.notify();
                                })),
                        ),
                )
            })
            .child(
                div()
                    .w_full()
                    .max_w(px(768.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .pl(px(8.))
                    .pr(px(8.))
                    .py(px(6.))
                    .rounded(px(24.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.divider)
                    .child(
                        icon_button("attach", IconName::Paperclip, 32., p.ink_secondary, p)
                            .rounded_full()
                            .on_click(cx.listener(|this, _, _, cx| this.attach(cx))),
                    )
                    .child({
                        let hover = p.surface_muted;
                        div()
                            .id("emoji-toggle")
                            .size(px(32.))
                            .flex_none()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(17.))
                            .when(self.emoji_open, |d| d.bg(p.primary_tint))
                            .when(!self.emoji_open, |d| d.hover(move |s| s.bg(hover)))
                            .child("😊")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.emoji_open = !this.emoji_open;
                                cx.notify();
                            }))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.composer).appearance(false)),
                    )
                    .child(
                        div()
                            .id("send")
                            .size(px(32.))
                            .flex_none()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(has_text, |d| {
                                let hover = p.primary_hover;
                                d.bg(p.primary).cursor_pointer().hover(move |s| s.bg(hover))
                            })
                            .child(icon(
                                IconName::SendHorizontal,
                                ICON,
                                if has_text {
                                    rgb(ON_BUBBLE).into()
                                } else {
                                    p.ink_disabled
                                },
                            ))
                            .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
                    ),
            )
    }
}

impl Render for RoomView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let narrow = window.viewport_size().width < px(super::shell::NARROW_W);
        if self.store.read(cx).selected_chat.is_none() {
            return self.render_empty(&p).into_any_element();
        }
        let typing_name = {
            let store = self.store.read(cx);
            store
                .selected()
                .filter(|c| store.is_typing(&c.chat_id))
                .map(|c| c.display_name().to_string())
        };
        let far = self.distance_from_bottom() > px(200.);
        let hover_bg = p.surface_muted;
        let pattern = self.store.read(cx).settings.wallpaper == Wallpaper::Pattern;
        let messages = self.render_messages(window, cx, &p);
        let pinned = self.render_pinned(cx, &p);
        let panel = self.panel.map(|t| self.render_panel(t, cx, &p));

        let conversation = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .when(pattern, |d| {
                        d.child(
                            img("images/chat-pattern.webp")
                                .absolute()
                                .inset_0()
                                .size_full()
                                .object_fit(ObjectFit::Cover)
                                .opacity(if p.dark { 0.03 } else { 0.05 }),
                        )
                    })
                    .child(div().size_full().child(messages).with_animation(
                        ElementId::Name(format!("room-enter-{}", self.room_epoch).into()),
                        Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                        |d, delta| d.opacity(0.35 + 0.65 * delta),
                    ))
                    .vertical_scrollbar(&self.scroll)
                    .when(far, |d| {
                        d.child(
                            div()
                                .absolute()
                                .right(px(28.))
                                .bottom(px(12.))
                                .child(
                                    div()
                                        .id("to-bottom")
                                        .size(px(36.))
                                        .rounded_full()
                                        .bg(p.surface)
                                        .border_1()
                                        .border_color(p.divider)
                                        .shadow(vec![BoxShadow {
                                            color: p.shadow,
                                            offset: point(px(0.), px(2.)),
                                            blur_radius: px(6.),
                                            spread_radius: px(0.),
                                            inset: false,
                                        }])
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(hover_bg))
                                        .child(icon(IconName::ArrowDown, ICON, p.ink_secondary))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.scroll_to_bottom(window, cx)
                                        })),
                                )
                                .with_animation(
                                    "to-bottom-enter",
                                    Animation::new(Duration::from_millis(150))
                                        .with_easing(ease_out_quint()),
                                    |d, delta| d.opacity(delta),
                                ),
                        )
                    }),
            )
            .when_some(typing_name, |d, name| {
                d.child(
                    div()
                        .px(px(24.))
                        .pt(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().flex().gap(px(3.)).children(
                            (0..3).map(|_| div().size(px(5.)).rounded_full().bg(p.primary)),
                        ))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.ink_subtitle)
                                .child(format!("{name} กำลังพิมพ์")),
                        ),
                )
            })
            .child(self.render_composer(cx, &p));

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(p.background)
            .child(self.render_header(narrow, cx, &p))
            .children(pinned)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(conversation)
                    .children(panel),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{animated_badge, is_video_message};
    use crate::models::{Message, MessageType};

    #[test]
    fn detects_animated_gif_and_webp_badges() {
        let mut msg = Message::temp("c1", "u1", "", MessageType::File, None);
        msg.file_name = Some("party.GIF".into());
        assert_eq!(animated_badge(&msg), Some("GIF"));

        msg.file_name = None;
        msg.media_url = Some("https://cdn.example.com/sticker.webp?v=2".into());
        assert_eq!(animated_badge(&msg), Some("WEBP"));

        msg.media_url = Some("https://cdn.example.com/photo.png".into());
        assert_eq!(animated_badge(&msg), None);
    }

    #[test]
    fn detects_video_messages() {
        let mut msg = Message::temp("c1", "u1", "", MessageType::File, None);
        msg.file_name = Some("clip.MP4".into());
        assert!(is_video_message(&msg));

        msg.file_name = None;
        msg.media_url = Some("https://cdn.example.com/demo.webm?token=123#t=1".into());
        assert!(is_video_message(&msg));

        msg.media_url = Some("https://cdn.example.com/report.pdf".into());
        assert!(!is_video_message(&msg));
    }
}
