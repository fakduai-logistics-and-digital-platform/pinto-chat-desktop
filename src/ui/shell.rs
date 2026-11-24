//! Main window, ported from `layouts/AppShell.vue`, `ChatList.vue` and
//! `ChatListItem.vue`: a 64px rail, the 300px chat list and the room.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable,
    button::Button,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenuItem},
    scroll::ScrollableElement,
    skeleton::Skeleton,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::{
    env::SEARCH_DEBOUNCE_MS,
    format,
    models::{Chat, UserProfile},
    store::Store,
    theme::{Palette, PaletteExt},
    ui::{
        room::RoomView,
        widgets::{avatar, drag_region, icon, icon_button, tnum, unread_badge, verified_badge},
    },
};

/// Narrow rail; the window controls overhang into the sidebar's empty top strip.
const RAIL_W: f32 = 56.;

pub enum ShellEvent {
    OpenSettings,
}

impl EventEmitter<ShellEvent> for ShellView {}

/// What the sidebar shows, picked from the rail.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Friends,
    Chats,
    /// Find people by name or username and start a chat (bots too).
    People,
}

/// Below this width the window shows one column at a time, like LINE's
/// compact window: the list, or the open chat with a back button.
pub const NARROW_W: f32 = 720.;

pub struct ShellView {
    people_search: Entity<InputState>,
    people_focused: bool,
    people_epoch: u64,
    people_scroll: ScrollHandle,
    narrow: bool,
    friends_scroll: ScrollHandle,
    /// Which search field has focus, for its border.
    search_focused: bool,
    friend_focused: bool,
    section: Section,
    friend_search: Entity<InputState>,
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
            match ev {
                InputEvent::Focus => {
                    this.search_focused = true;
                    cx.notify();
                }
                InputEvent::Blur => {
                    this.search_focused = false;
                    cx.notify();
                }
                _ => {}
            }
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
        let friend_search = cx.new(|cx| InputState::new(window, cx).placeholder("ค้นหาเพื่อน"));
        cx.subscribe(&friend_search, |this, _, ev: &InputEvent, cx| {
            match ev {
                InputEvent::Focus => this.friend_focused = true,
                InputEvent::Blur => this.friend_focused = false,
                _ => {}
            }
            cx.notify();
        })
        .detach();
        let people_search = cx.new(|cx| InputState::new(window, cx).placeholder("ชื่อหรือชื่อผู้ใช้"));
        cx.subscribe(&people_search, |this, input, ev: &InputEvent, cx| {
            match ev {
                InputEvent::Focus => this.people_focused = true,
                InputEvent::Blur => this.people_focused = false,
                InputEvent::Change => {
                    // Debounced: search once typing pauses.
                    let q = input.read(cx).value().trim().to_string();
                    this.people_epoch += 1;
                    let epoch = this.people_epoch;
                    cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS + 100))
                            .await;
                        this.update(cx, |this, cx| {
                            if this.people_epoch == epoch {
                                this.store.update(cx, |s, cx| s.search_people(q, cx));
                            }
                        })
                        .ok();
                    })
                    .detach();
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        let preview = if cfg!(debug_assertions) {
            std::env::var("PINTO_PREVIEW").unwrap_or_default()
        } else {
            String::new()
        };
        let section = if preview.contains("friends") {
            Section::Friends
        } else if preview.contains("people") {
            Section::People
        } else {
            Section::Chats
        };
        Self {
            people_search,
            people_focused: false,
            people_epoch: 0,
            people_scroll: ScrollHandle::new(),
            narrow: false,
            friends_scroll: ScrollHandle::new(),
            search_focused: false,
            friend_focused: false,
            section,
            friend_search,
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
            .pb(px(12.))
            .gap(px(4.))
            // The window controls sit in this strip; it also drags the window.
            .child(drag_region(div().h(px(32.)).w_full().flex_none()))
            .child(self.rail_item("rail-friends", IconName::User, Section::Friends, 0, cx, p))
            .child(self.rail_item(
                "rail-chats",
                IconName::MessageCircle,
                Section::Chats,
                unread,
                cx,
                p,
            ))
            .child(self.rail_item("rail-people", IconName::UserPlus, Section::People, 0, cx, p))
            .child(div().flex_1())
            .child(
                icon_button("rail-settings", IconName::Settings, 36., muted, p)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(ShellEvent::OpenSettings))),
            )
            // Your own avatar opens the account menu.
            .child(
                div()
                    .id("rail-account")
                    .size(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .cursor_pointer()
                    .when(menu_open, |d| d.bg(active_bg))
                    .hover(move |s| s.bg(hover_bg))
                    .child(avatar(photo, 28., p))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.user_menu_open = !this.user_menu_open;
                        cx.notify();
                    })),
            )
    }

    fn rail_item(
        &self,
        id: &'static str,
        name: IconName,
        section: Section,
        badge: i64,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> impl IntoElement {
        let active = self.section == section;
        div()
            .relative()
            .child(
                icon_button(
                    id,
                    name,
                    36.,
                    if active {
                        p.primary_ink
                    } else {
                        p.ink_secondary
                    },
                    p,
                )
                .when(active, |d| d.bg(p.primary_tint))
                .on_click(cx.listener(move |this, _, _, cx| this.show(section, cx))),
            )
            .when(badge > 0, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(-4.))
                        .right(px(-6.))
                        .child(unread_badge(badge, p)),
                )
            })
    }

    fn show(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.user_menu_open = false;
        if section == Section::Friends {
            self.store.update(cx, |s, cx| s.load_friends(false, cx));
        }
        cx.notify();
    }

    // ---- friends --------------------------------------------------------

    fn render_friends(&self, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let q = self.friend_search.read(cx).value().trim().to_lowercase();
        let store = self.store.read(cx);
        let loading = store.friends_loading;
        let total = store.friends.len();
        let friends: Vec<UserProfile> = store
            .friends
            .iter()
            .filter(|u| {
                q.is_empty()
                    || u.display_name().to_lowercase().contains(&q)
                    || u.username.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        let hover = p.surface_muted;

        let header = div()
            .h(px(64.))
            .flex_none()
            .flex()
            .flex_col()
            .child(drag_region(div().flex_1()))
            .child(
                div()
                    .h(px(36.))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .border_b_1()
                    .border_color(p.divider)
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child("เพื่อน"),
                    )
                    .when(total > 0, |d| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.ink_subtitle)
                                .font_features(tnum())
                                .child(total.to_string()),
                        )
                    }),
            );

        let body: AnyElement = if loading && total == 0 {
            div()
                .flex()
                .flex_col()
                .children((0..8).map(|i| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(10.))
                        .child(Skeleton::new().size(px(40.)).rounded_full())
                        .child(
                            Skeleton::new()
                                .h(px(12.))
                                .w(px(if i % 2 == 0 { 140. } else { 100. }))
                                .rounded(px(2.)),
                        )
                }))
                .into_any_element()
        } else if friends.is_empty() {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(24.))
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(if q.is_empty() {
                            "ยังไม่มีเพื่อน"
                        } else {
                            "ไม่พบเพื่อน"
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .text_center()
                        .child(if q.is_empty() {
                            "คนที่คุณติดตามในแอป Pinto จะแสดงที่นี่".to_string()
                        } else {
                            format!("ไม่มีเพื่อนที่ตรงกับ “{q}”")
                        }),
                )
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = friends
                .into_iter()
                .map(|u| {
                    let image = u
                        .profile_image
                        .clone()
                        .and_then(|url| self.store.update(cx, |s, cx| s.image(&url, cx)));
                    let friend = u.clone();
                    div()
                        .id(SharedString::from(format!("friend-{}", u.user_id)))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(10.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover))
                        .child(avatar(image, 40., p))
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
                                                .line_height(px(20.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(u.display_name().to_string()),
                                        )
                                        .when(u.is_official, |d| d.child(verified_badge(14., p))),
                                )
                                .when(!u.username.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.ink_subtitle)
                                            .child(format!("@{}", u.username)),
                                    )
                                }),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.section = Section::Chats;
                            this.store.update(cx, |s, cx| s.open_direct(&friend, cx));
                            cx.notify();
                        }))
                        .into_any_element()
                })
                .collect();
            // The scrollbar sits on a fixed wrapper, not on the moving content.
            div()
                .relative()
                .size_full()
                .child(
                    div()
                        .id("friends-list")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.friends_scroll)
                        .children(rows),
                )
                .vertical_scrollbar(&self.friends_scroll)
                .into_any_element()
        };

        div()
            .when(self.narrow, |d| d.flex_1().min_w_0())
            .when(!self.narrow, |d| d.w(px(300.)).flex_none())
            .h_full()
            .flex()
            .flex_col()
            .bg(p.surface)
            .border_r_1()
            .border_color(p.divider)
            .child(header)
            .child(div().p(px(12.)).child(search_field(
                &self.friend_search,
                self.friend_focused,
                p,
            )))
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    // ---- find people ----------------------------------------------------

    fn render_people(&self, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let store = self.store.read(cx);
        let (query, loading) = (store.people_query.clone(), store.people_loading);
        let people: Vec<UserProfile> = store.people.clone();
        let hover = p.surface_muted;

        let body: AnyElement = if query.is_empty() {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(24.))
                .gap(px(4.))
                .child(
                    div()
                        .size(px(48.))
                        .mb(px(8.))
                        .rounded_full()
                        .bg(p.primary_tint)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(IconName::UserPlus, 22., p.primary_ink)),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child("ค้นหาคนหรือบอทในแอป Pinto"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .text_center()
                        .child("พิมพ์ชื่อหรือชื่อผู้ใช้ แล้วกดแชทเพื่อเริ่มคุยได้ทันที"),
                )
                .into_any_element()
        } else if loading && people.is_empty() {
            div()
                .flex()
                .flex_col()
                .children((0..6).map(|i| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(10.))
                        .child(Skeleton::new().size(px(40.)).rounded_full())
                        .child(
                            Skeleton::new()
                                .h(px(12.))
                                .w(px(if i % 2 == 0 { 140. } else { 100. }))
                                .rounded(px(2.)),
                        )
                }))
                .into_any_element()
        } else if people.is_empty() {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(24.))
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child("ไม่พบผู้ใช้"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .text_center()
                        .child(format!("ไม่มีใครตรงกับ “{query}”")),
                )
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = people
                .into_iter()
                .map(|u| {
                    let image = u
                        .profile_image
                        .clone()
                        .and_then(|url| self.store.update(cx, |s, cx| s.image(&url, cx)));
                    let chat_with = u.clone();
                    div()
                        .id(SharedString::from(format!("person-{}", u.user_id)))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(10.))
                        .hover(move |s| s.bg(hover))
                        .child(avatar(image, 40., p))
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
                                                .line_height(px(20.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(u.display_name().to_string()),
                                        )
                                        .when(u.is_official, |d| {
                                            d.child(icon(IconName::BadgeCheck, 14., p.primary_ink))
                                        }),
                                )
                                .when(!u.username.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.ink_subtitle)
                                            .child(format!("@{}", u.username)),
                                    )
                                }),
                        )
                        .child(
                            div().flex().flex_none().gap(px(4.)).child(
                                Button::new(SharedString::from(format!("chat-{}", u.user_id)))
                                    .outline()
                                    .small()
                                    .label("แชท")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.section = Section::Chats;
                                        this.store
                                            .update(cx, |s, cx| s.open_direct(&chat_with, cx));
                                        cx.notify();
                                    })),
                            ),
                        )
                        .into_any_element()
                })
                .collect();
            // The scrollbar sits on a fixed wrapper, not on the moving content.
            div()
                .relative()
                .size_full()
                .child(
                    div()
                        .id("people-list")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.people_scroll)
                        .children(rows),
                )
                .vertical_scrollbar(&self.people_scroll)
                .into_any_element()
        };

        div()
            .when(self.narrow, |d| d.flex_1().min_w_0())
            .when(!self.narrow, |d| d.w(px(300.)).flex_none())
            .h_full()
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
                    .child(
                        div()
                            .h(px(36.))
                            .px(px(16.))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(p.divider)
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child("เพิ่มเพื่อน"),
                            ),
                    ),
            )
            .child(div().p(px(12.)).child(search_field(
                &self.people_search,
                self.people_focused,
                p,
            )))
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
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
                   value: crate::store::ChatFilter,
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
            .child(tab("tab-all", "ทั้งหมด", crate::store::ChatFilter::All, cx))
            .child(tab(
                "tab-direct",
                "เพื่อน",
                crate::store::ChatFilter::Direct,
                cx,
            ))
            .child(tab("tab-group", "กลุ่ม", crate::store::ChatFilter::Group, cx))
            .child(tab("tab-bot", "บอท", crate::store::ChatFilter::Bot, cx))
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
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(14.))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(chat.display_name().to_string()),
                                    )
                                    .when(self.store.read(cx).is_chat_official(chat), |d| {
                                        d.child(verified_badge(14., p))
                                    }),
                            )
                            .when(chat.is_pinned, |d| {
                                d.child(icon(IconName::Pin, 12., p.primary_ink))
                            })
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
            .context_menu({
                let shell = cx.entity().downgrade();
                let chat_id = chat.chat_id.clone();
                let archived = chat.is_archived;
                let pinned = chat.is_pinned;
                let muted = chat.is_muted;
                let unread = chat.unread_count > 0;
                move |menu, _, cx| {
                    let pal = cx.palette();
                    let row = move |label: &'static str, name: IconName, danger: bool| {
                        let color = if danger { pal.error_strong } else { pal.ink };
                        let icon_color = if danger {
                            pal.error_strong
                        } else {
                            pal.ink_secondary
                        };
                        PopupMenuItem::element(move |_, _| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_color(color)
                                .child(icon(name, 15., icon_color))
                                .child(label)
                        })
                    };
                    let act = |f: fn(&mut Store, &str, &mut Context<Store>)| {
                        let shell = shell.clone();
                        let cid = chat_id.clone();
                        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                            let cid = cid.clone();
                            let _ = shell.update(cx, |v, cx| {
                                v.store.update(cx, |s, cx| f(s, &cid, cx));
                            });
                        }
                    };
                    menu.item(
                        row(
                            if archived {
                                "ย้ายออกจากที่เก็บถาวร"
                            } else {
                                "เก็บถาวร (Archive)"
                            },
                            IconName::Archive,
                            false,
                        )
                        .on_click(act(Store::toggle_archive_chat)),
                    )
                    .item(
                        row(
                            if pinned {
                                "เลิกปักหมุด"
                            } else {
                                "ปักหมุด"
                            },
                            IconName::Pin,
                            false,
                        )
                        .on_click(act(Store::toggle_pin_chat)),
                    )
                    .item(
                        row(
                            if muted {
                                "เปิดเสียงแจ้งเตือน"
                            } else {
                                "ปิดเสียงแจ้งเตือน"
                            },
                            if muted {
                                IconName::Bell
                            } else {
                                IconName::BellOff
                            },
                            false,
                        )
                        .on_click(act(Store::toggle_mute)),
                    )
                    .item(
                        row(
                            if unread {
                                "ทำเครื่องหมายว่าอ่านแล้ว"
                            } else {
                                "ทำเครื่องหมายว่ายังไม่อ่าน"
                            },
                            IconName::MessageCircle,
                            false,
                        )
                        .on_click(act(Store::toggle_unread_chat)),
                    )
                    .separator()
                    .item(
                        row("ล้างประวัติแชท", IconName::Eraser, false)
                            .on_click(act(Store::clear_chat_history)),
                    )
                    .item(row("ลบแชท", IconName::Trash, true).on_click(act(Store::delete_chat)))
                }
            })
    }

    fn render_list(&self, cx: &mut Context<Self>, p: &Palette) -> AnyElement {
        let store = self.store.read(cx);
        let chats = store.sorted_chats();
        let (archived_count, archived_names, archived_unread) = store.archived_summary();
        let show_archived = store.show_archived;
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

        if chats.is_empty() && archived_count == 0 {
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

        let list = div()
            .id("chat-list")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .when(archived_count > 0 || show_archived, |d| {
                let hover = p.surface_muted;
                let avatar_sz = if compact { 36. } else { 44. };
                d.child(
                    div()
                        .id("archived-toggle")
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .py(px(if compact { 6. } else { 10. }))
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover))
                        .child(
                            div()
                                .size(px(avatar_sz))
                                .flex_none()
                                .rounded_full()
                                .bg(p.surface_muted)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(
                                    if show_archived {
                                        IconName::ChevronLeft
                                    } else {
                                        IconName::Archive
                                    },
                                    18.,
                                    p.ink_secondary,
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(if show_archived {
                                            "กลับไปแชทหลัก"
                                        } else {
                                            "แชทที่เก็บถาวร"
                                        }),
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
                                                .text_color(p.ink_secondary)
                                                .child(if show_archived {
                                                    format!("{archived_count} แชทที่เก็บถาวร")
                                                } else {
                                                    archived_names
                                                }),
                                        )
                                        .when(!show_archived && archived_unread > 0, |d| {
                                            d.child(unread_badge(archived_unread, p))
                                        }),
                                ),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.store
                                .update(cx, |s, cx| s.set_show_archived(!show_archived, cx));
                        })),
                )
            })
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
            });
        // The scrollbar sits on a fixed wrapper, not on the moving content.
        div()
            .relative()
            .size_full()
            .child(list)
            .vertical_scrollbar(&self.list_scroll)
            .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        div()
            .when(self.narrow, |d| d.flex_1().min_w_0())
            .when(!self.narrow, |d| d.w(px(300.)).flex_none())
            .h_full()
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
                div()
                    .p(px(12.))
                    .child(search_field(&self.search, self.search_focused, p)),
            )
            .child(div().flex_1().min_h_0().child(self.render_list(cx, p)))
    }
}

/// Search box shared by the chat and friends lists: 36px, soft fill,
/// 10px radius, a hairline that turns green while it has focus.
fn search_field(state: &Entity<InputState>, focused: bool, p: &Palette) -> impl IntoElement {
    div()
        .h(px(36.))
        .px(px(12.))
        .flex()
        .items_center()
        .gap(px(2.))
        .rounded(px(10.))
        .bg(if focused { p.surface } else { p.surface_muted })
        .border_1()
        .border_color(if focused { p.primary } else { p.hairline })
        .child(icon(IconName::Search, 16., p.ink_subtitle))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(state).appearance(false).cleanable(true)),
        )
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        self.narrow = window.viewport_size().width < px(NARROW_W);
        let chat_open = self.store.read(cx).selected_chat.is_some();
        // Narrow: the list, or the open chat, never both.
        let show_list = !self.narrow || !chat_open || self.section != Section::Chats;
        let show_room = !self.narrow || (chat_open && self.section == Section::Chats);
        div()
            .size_full()
            .relative()
            .flex()
            .bg(p.background)
            .text_color(p.ink)
            .child(self.render_rail(cx, &p))
            .when(show_list, |d| {
                d.child(match self.section {
                    Section::Chats => self.render_sidebar(cx, &p).into_any_element(),
                    Section::Friends => self.render_friends(cx, &p),
                    Section::People => self.render_people(cx, &p),
                })
            })
            .when(show_room, |d| {
                d.child(div().flex_1().min_w_0().h_full().child(self.room.clone()))
            })
            .when(self.user_menu_open, |d| {
                d.child(self.render_user_menu(cx, &p))
            })
    }
}
