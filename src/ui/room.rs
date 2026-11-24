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
    skeleton::Skeleton,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::sync::Arc;

use crate::{
    env::{MAX_MESSAGE_LENGTH, TYPING_DEBOUNCE_MS},
    format,
    models::{ChatType, Message, MessageStatus, MessageType, ReplyTarget},
    persist::Wallpaper,
    realtime::ConnectionState,
    store::{Store, StoreEvent},
    theme::{Palette, PaletteExt},
    ui::widgets::{ICON, avatar, drag_region, icon, icon_button, tnum},
};

/// Tabs of the side panel listing what a chat has shared.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaTab {
    Images,
    Files,
    Links,
}

impl MediaTab {
    fn label(self) -> &'static str {
        match self {
            Self::Images => "รูปภาพ",
            Self::Files => "ไฟล์",
            Self::Links => "ลิงก์",
        }
    }
}

/// Dark ink on the brand green keeps message text above 4.5:1.
const ON_BUBBLE: u32 = 0x10281A;

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
    drafts: std::collections::HashMap<String, (String, Option<ReplyTarget>)>,
    scroll_states: std::collections::HashMap<String, (bool, Point<Pixels>)>,
}

impl RoomView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("ข้อความ"));
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
            drafts: std::collections::HashMap::new(),
            scroll_states: std::collections::HashMap::new(),
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
                                .on_click(move |_, _, cx| cx.open_url(&url))
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
                                        .child(icon(IconName::File, 18., p.ink_secondary)),
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
                                                .child(
                                                    m.file_name
                                                        .clone()
                                                        .unwrap_or_else(|| "ไฟล์".into()),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.ink_subtitle)
                                                .font_features(tnum())
                                                .child(meta),
                                        ),
                                )
                                .on_click(move |_, _, cx| {
                                    if let Some(u) = &url {
                                        cx.open_url(u);
                                    }
                                })
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
                    .gap(px(20.))
                    .pl(px(16.))
                    .pr(px(8.))
                    .border_b_1()
                    .border_color(p.divider)
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
                    .id("panel-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(12.))
                    .child(body),
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
                    .child(img("images/logo.png").size(px(48.))),
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
        let image = chat
            .as_ref()
            .and_then(|c| c.avatar_url.clone())
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let muted = chat.as_ref().is_some_and(|c| c.is_muted);
        let name = chat
            .as_ref()
            .map(|c| c.display_name().to_string())
            .unwrap_or_default();
        let (sub, sub_color) = if connection != ConnectionState::Connected {
            ("ออฟไลน์ · ข้อความจะส่งเมื่อเชื่อมต่อได้".to_string(), p.ink_subtitle)
        } else if typing {
            ("กำลังพิมพ์…".to_string(), p.primary_ink)
        } else if chat
            .as_ref()
            .is_some_and(|c| c.chat_type == ChatType::Group)
        {
            ("กลุ่ม".to_string(), p.ink_subtitle)
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
            .when(cfg!(target_os = "windows"), |d| d.pr(px(124.)))
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
            .child(avatar(image, 40., p))
            .child(
                drag_region(div())
                    .flex_1()
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
                            .when(muted, |d| {
                                d.child(icon(IconName::BellOff, 14., p.ink_subtitle))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(connection != ConnectionState::Connected, |d| {
                                d.child(div().size(px(6.)).rounded_full().bg(p.ink_disabled))
                            })
                            .child(div().text_size(px(12.)).text_color(sub_color).child(sub)),
                    ),
            )
            .child(
                Button::new("room-menu")
                    .ghost()
                    .icon(icon(IconName::EllipsisVertical, ICON, p.ink_secondary))
                    .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                        let pal = cx.palette();
                        let Some(view) = room.upgrade() else {
                            return menu;
                        };
                        let (muted, wallpaper) = {
                            let s = view.read(cx).store.read(cx);
                            (
                                s.selected().is_some_and(|c| c.is_muted),
                                s.settings.wallpaper,
                            )
                        };
                        // Every row drawn the same way, so icons and labels share one column.
                        let row = move |label: &'static str, name: Option<IconName>| {
                            PopupMenuItem::element(move |_, _| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_color(pal.ink)
                                    .child(match name {
                                        Some(n) => {
                                            icon(n, 16., pal.ink_secondary).into_any_element()
                                        }
                                        None => div().size(px(16.)).into_any_element(),
                                    })
                                    .child(label)
                            })
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
                        .separator()
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
            bubble = bubble.child(match media {
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
                                .rounded(px(4.))
                                .bg(p.overlay.opacity(0.75))
                                .text_size(px(10.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.surface)
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
            });
        } else if m.message_type == MessageType::File {
            bubble = bubble.child(
                div()
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

    fn render_composer(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let has_text = !self.composer.read(cx).value().trim().is_empty();
        div()
            .flex_none()
            .px(px(24.))
            .pb(px(16.))
            .pt(px(8.))
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
