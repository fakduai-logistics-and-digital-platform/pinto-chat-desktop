//! Main window, ported from `layouts/AppShell.vue`, `ChatList.vue` and
//! `ChatListItem.vue`: a 64px rail, the 300px chat list and the room.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable,
    input::{Input, InputEvent, InputState},
    skeleton::Skeleton,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::{
    env::SEARCH_DEBOUNCE_MS,
    format,
    models::{Chat, ChatType},
    store::Store,
    theme::{Palette, PaletteExt},
    ui::{
        room::RoomView,
        widgets::{avatar, drag_region, icon, icon_button, tnum, unread_badge},
    },
};

const RAIL_W: f32 = 76.;

pub enum ShellEvent {
    OpenSettings,
}

impl EventEmitter<ShellEvent> for ShellView {}

pub struct ShellView {
    store: Entity<Store>,
    room: Entity<RoomView>,
    search: Entity<InputState>,
    search_epoch: u64,
    list_scroll: ScrollHandle,
    user_menu_open: bool,
}

impl ShellView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let room = cx.new(|cx| RoomView::new(store.clone(), window, cx));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("ค้นหา"));
        cx.subscribe_in(&search, window, |this, input, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let q = input.read(cx).value().trim().to_string();
                this.search_epoch += 1;
                let epoch = this.search_epoch;
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                        .await;
                    this.update(cx, |this, cx| {
                        if this.search_epoch == epoch {
                            this.store.update(cx, |s, cx| s.set_search(q, cx));
                        }
                    })
                    .ok();
                })
                .detach();
            }
        })
        .detach();
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        Self {
            store,
            room,
            search,
            search_epoch: 0,
            list_scroll: ScrollHandle::new(),
            user_menu_open: cfg!(debug_assertions)
                && std::env::var("PINTO_PREVIEW").is_ok_and(|v| v.contains("menu")),
        }
    }

    // ---- rail -----------------------------------------------------------

    fn render_rail(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let unread = self.store.read(cx).total_unread();
        let muted = p.ink_secondary;
        let user = self.store.read(cx).user.clone().unwrap_or_default();
        let photo = user
            .profile_image
            .clone()
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let menu_open = self.user_menu_open;
        let (active_bg, hover_bg) = (p.hairline, p.surface_muted);
        div()
            .w(px(RAIL_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .bg(p.surface)
            .border_r_1()
            .border_color(p.divider)
            .pb(px(16.))
            .gap(px(8.))
            // The window controls sit in this strip; it also drags the window.
            .child(drag_region(div().h(px(40.)).w_full().flex_none()))
            .child(
                img("images/logo.png")
                    .size(px(28.))
                    .rounded(px(6.))
                    .mb(px(12.)),
            )
            .child(
                div()
                    .relative()
                    .child(
                        icon_button("rail-chats", IconName::MessageCircle, 40., p.primary_ink, p)
                            .bg(p.primary_tint),
                    )
                    .when(unread > 0, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(-4.))
                                .right(px(-6.))
                                .child(unread_badge(unread, p)),
                        )
                    }),
            )
            // Friends and add-friend stay hidden until the API supports them.
            .child(div().flex_1())
            .child(
                icon_button("rail-settings", IconName::Settings, 40., muted, p)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShellEvent::OpenSettings))),
            )
            // Your own avatar opens the account menu.
            .child(
                div()
                    .id("rail-account")
                    .size(px(40.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .cursor_pointer()
                    .when(menu_open, |d| d.bg(active_bg))
                    .hover(move |s| s.bg(hover_bg))
                    .child(avatar(photo, 32., p))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.user_menu_open = !this.user_menu_open;
                        cx.notify();
                    })),
            )
    }

    fn render_user_menu(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let store = self.store.read(cx);
        let user = store.user.clone().unwrap_or_default();
        let photo = user.profile_image.clone();
        let image = photo.and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let hover = p.surface_muted;
        div()
            .absolute()
            .inset_0()
            .child(
                // Click outside closes.
                div()
                    .id("menu-backdrop")
                    .absolute()
                    .inset_0()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.user_menu_open = false;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("user-menu")
                    .occlude()
                    .absolute()
                    .left(px(RAIL_W + 8.))
                    .bottom(px(16.))
                    .w(px(232.))
                    .p(px(8.))
                    .rounded(px(16.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.divider)
                    .shadow(vec![BoxShadow {
                        color: p.shadow,
                        offset: point(px(0.), px(4.)),
                        blur_radius: px(16.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .p(px(8.))
                            .child(avatar(image, 36., p))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(user.display_name().to_string()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.ink_subtitle)
                                            .truncate()
                                            .child(if user.username.is_empty() {
                                                "บัญชี Pinto".to_string()
                                            } else {
                                                format!("@{}", user.username)
                                            }),
                                    ),
                            ),
                    )
                    .child(div().h(px(1.)).my(px(4.)).bg(p.hairline))
                    .child(
                        div()
                            .id("logout")
                            .flex()
                            .items_center()
                            // Same 36px icon column as the avatar above, so both labels start on one line.
                            .gap(px(12.))
                            .h(px(40.))
                            .px(px(8.))
                            .rounded(px(8.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover))
                            .text_size(px(14.))
                            .text_color(p.ink)
                            .child(div().w(px(36.)).flex().justify_center().child(icon(
                                IconName::LogOut,
                                18.,
                                p.ink_secondary,
                            )))
                            .child("ออกจากระบบ")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.user_menu_open = false;
                                this.store.update(cx, |s, cx| s.logout(cx));
                            })),
                    ),
            )
    }

    // ---- chat list ------------------------------------------------------

    fn render_tabs(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let filter = self.store.read(cx).filter;
        let tab = |id: &'static str,
                   label: &'static str,
                   value: Option<ChatType>,
                   cx: &mut Context<Self>| {
            let active = filter == value;
            div()
                .id(id)
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
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active {
                    p.primary_ink
                } else {
                    p.ink_secondary
                })
                .cursor_pointer()
                .hover(|s| s.text_color(p.ink))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| s.set_filter(value, cx));
                    this.list_scroll.set_offset(point(px(0.), px(0.)));
                }))
        };
        div()
            .flex()
            .items_end()
            .gap(px(20.))
            .px(px(16.))
            .border_b_1()
            .border_color(p.divider)
            .child(tab("tab-all", "ทั้งหมด", None, cx))
            .child(tab("tab-direct", "เพื่อน", Some(ChatType::Direct), cx))
            .child(tab("tab-group", "กลุ่ม", Some(ChatType::Group), cx))
    }

    fn render_row(
        &self,
        chat: &Chat,
        active: bool,
        compact: bool,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> impl IntoElement {
        let image = chat
            .avatar_url
            .clone()
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));
        let typing = self.store.read(cx).is_typing(&chat.chat_id);
        let preview = if typing {
            "กำลังพิมพ์…".to_string()
        } else {
            format::truncate(
                chat.last_message_preview
                    .as_deref()
                    .unwrap_or("ยังไม่มีข้อความ"),
                48,
            )
        };
        let id = chat.chat_id.clone();
        let hover = p.surface_muted;
        div()
            .id(SharedString::from(format!("chat-{}", chat.chat_id)))
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(16.))
            .py(px(if compact { 6. } else { 10. }))
            .cursor_pointer()
            .when(active, |d| d.bg(p.primary_tint))
            .when(!active, |d| d.hover(move |s| s.bg(hover)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.user_menu_open = false;
                this.store
                    .update(cx, |s, cx| s.select_chat(Some(id.clone()), cx));
            }))
            .child(avatar(image, if compact { 36. } else { 44. }, p))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(chat.display_name().to_string()),
                            )
                            .when(chat.is_muted, |d| {
                                d.child(icon(IconName::BellOff, 12., p.ink_subtitle))
                            })
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.ink_subtitle)
                                    .font_features(tnum())
                                    .child(format::chat_time(chat.last_message_at.as_deref())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(if typing {
                                        p.primary_ink
                                    } else {
                                        p.ink_secondary
                                    })
                                    .child(preview),
                            )
                            .when(chat.unread_count > 0, |d| {
                                d.child(unread_badge(chat.unread_count, p))
                            }),
                    ),
            )
    }

    fn render_list(&self, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let store = self.store.read(cx);
        let chats = store.sorted_chats();
        let selected = store.selected_chat.clone();
        let (loading, loading_more, has_more) = (
            store.chats_loading,
            store.chats_loading_more,
            store.chats_has_more,
        );
        let compact = store.settings.compact_mode;
        let searching = !store.search.is_empty();
        let search = store.search.clone();

        if loading && chats.is_empty() {
            return div()
                .flex()
                .flex_col()
                .children((0..8).map(|i| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(12.))
                        .child(Skeleton::new().size(px(48.)).rounded_full())
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(
                                    Skeleton::new()
                                        .h(px(12.))
                                        .w(px(if i % 2 == 0 { 120. } else { 96. }))
                                        .rounded(px(2.)),
                                )
                                .child(Skeleton::new().h(px(12.)).w(px(180.)).rounded(px(2.))),
                        )
                }))
                .into_any_element();
        }

        if chats.is_empty() {
            let (title, body) = if searching {
                ("ไม่พบแชท".to_string(), format!("ไม่มีแชทที่ตรงกับ “{search}”"))
            } else {
                (
                    "ยังไม่มีแชท".to_string(),
                    "แชทที่เริ่มจากแอป Pinto จะแสดงที่นี่".to_string(),
                )
            };
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(24.))
                .gap(px(4.))
                .child(
                    div()
                        .size(px(56.))
                        .mb(px(12.))
                        .rounded_full()
                        .bg(p.primary_tint)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(
                            if searching {
                                IconName::Search
                            } else {
                                IconName::MessageCircle
                            },
                            24.,
                            p.primary_ink,
                        )),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .text_center()
                        .child(body),
                )
                .into_any_element();
        }

        // Infinite scroll: ask for the next page once within 100px of the end.
        let offset = self.list_scroll.offset().y;
        let max = self.list_scroll.max_offset().y;
        if has_more && !loading_more && max > px(0.) && max + offset < px(100.) {
            let store = self.store.clone();
            cx.defer(move |cx| store.update(cx, |s, cx| s.load_chats(false, cx)));
        }

        div()
            .id("chat-list")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .children(
                chats
                    .iter()
                    .map(|c| {
                        self.render_row(c, selected.as_deref() == Some(&c.chat_id), compact, cx, p)
                            .into_any_element()
                    })
                    .collect::<Vec<_>>(),
            )
            .when(loading_more, |d| {
                d.child(
                    div()
                        .p(px(16.))
                        .text_center()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .child("กำลังโหลด"),
                )
            })
            .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        div()
            .w(px(300.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(p.surface)
            .border_r_1()
            .border_color(p.divider)
            .child(
                div()
                    .h(px(64.))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .child(drag_region(div().flex_1()))
                    .child(self.render_tabs(cx, p)),
            )
            .child(
                div().p(px(12.)).child(
                    Input::new(&self.search)
                        .prefix(icon(IconName::Search, 16., p.ink_subtitle))
                        .cleanable(true)
                        .small(),
                ),
            )
            .child(div().flex_1().min_h_0().child(self.render_list(cx, p)))
    }
}

impl Render for ShellView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        div()
            .size_full()
            .relative()
            .flex()
            .bg(p.background)
            .text_color(p.ink)
            .child(self.render_rail(cx, &p))
            .child(self.render_sidebar(cx, &p))
            .child(div().flex_1().min_w_0().h_full().child(self.room.clone()))
            .when(self.user_menu_open, |d| {
                d.child(self.render_user_menu(cx, &p))
            })
    }
}
