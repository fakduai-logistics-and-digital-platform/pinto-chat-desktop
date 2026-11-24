//! Settings page, ported from `pages/SettingsPage.vue`.

use gpui_kit::assets::IconName;
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::{
    persist::ThemeMode,
    store::Store,
    theme::{Palette, PaletteExt},
    ui::widgets::{avatar, drag_region, icon, icon_button, tnum},
};

pub enum SettingsEvent {
    Back,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

pub struct SettingsView {
    store: Entity<Store>,
    scroll: ScrollHandle,
}

impl SettingsView {
    pub fn new(store: Entity<Store>, cx: &mut Context<Self>) -> Self {
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        Self {
            store,
            scroll: ScrollHandle::new(),
        }
    }

    fn section(title: &'static str, p: &Palette) -> Div {
        div()
            .w_full()
            .p(px(20.))
            .rounded(px(12.))
            .bg(p.surface)
            .border_1()
            .border_color(p.hairline)
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(16.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(title),
            )
    }

    /// Segmented control: one surface, the chosen option lifted onto white.
    fn theme_control(
        &self,
        current: ThemeMode,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> impl IntoElement {
        let options = [
            (ThemeMode::Light, "สว่าง"),
            (ThemeMode::Dark, "มืด"),
            (ThemeMode::System, "ตามระบบ"),
        ];
        div()
            .flex()
            .p(px(2.))
            .gap(px(2.))
            .rounded(px(10.))
            .bg(p.surface_muted)
            .border_1()
            .border_color(p.hairline)
            .children(options.into_iter().map(|(mode, label)| {
                let active = mode == current;
                div()
                    .id(SharedString::from(format!("theme-{label}")))
                    .h(px(28.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .when(active, |d| {
                        d.bg(p.surface)
                            .border_1()
                            .border_color(p.divider)
                            .text_color(p.ink)
                            .font_weight(FontWeight::MEDIUM)
                    })
                    .when(!active, |d| {
                        d.text_color(p.ink_secondary).hover(|s| s.text_color(p.ink))
                    })
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.store
                            .update(cx, |s, cx| s.update_settings(|st| st.theme = mode, cx));
                    }))
            }))
    }

    fn row(
        label: &'static str,
        hint: Option<&'static str>,
        control: impl IntoElement,
        p: &Palette,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(14.)).text_color(p.ink).child(label))
                    .when_some(hint, |d, h| {
                        d.child(div().text_size(px(12.)).text_color(p.ink_subtitle).child(h))
                    }),
            )
            .child(control)
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let store = self.store.read(cx);
        let user = store.user.clone().unwrap_or_default();
        let settings = store.settings.clone();
        let image = user
            .profile_image
            .clone()
            .and_then(|u| self.store.update(cx, |s, cx| s.image(&u, cx)));

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            // Strip for the window controls; it also drags the window.
            .child(drag_region(div().h(px(32.)).w_full().flex_none()))
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .mx_auto()
                            .w_full()
                            .max_w(px(672.))
                            .px(px(24.))
                            .pt(px(8.))
                            .pb(px(32.))
                            .flex()
                            .flex_col()
                            .gap(px(24.))
                            // Page title on the card column's left edge, back arrow in front.
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .ml(px(-8.))
                                    .child(
                                        icon_button(
                                            "back",
                                            IconName::ChevronLeft,
                                            32.,
                                            p.ink_secondary,
                                            &p,
                                        )
                                        .on_click(
                                            cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Back)),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(24.))
                                            .line_height(px(32.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child("ตั้งค่า"),
                                    ),
                            )
                            .child(
                                Self::section("บัญชี", &p).child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(16.))
                                        .child(avatar(image, 48., &p))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .child(
                                                    div()
                                                        .text_size(px(16.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(p.ink)
                                                        .truncate()
                                                        .child(user.display_name().to_string()),
                                                )
                                                .when(!user.username.is_empty(), |d| {
                                                    d.child(
                                                        div()
                                                            .text_size(px(14.))
                                                            .text_color(p.ink_secondary)
                                                            .child(format!("@{}", user.username)),
                                                    )
                                                }),
                                        )
                                        // Destructive actions in a list are red text, per DESIGN.md.
                                        .child({
                                            let hover = p.error_container;
                                            div()
                                                .id("logout")
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .h(px(36.))
                                                .px(px(12.))
                                                .mr(px(-12.))
                                                .rounded(px(10.))
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(hover))
                                                .text_size(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.error_strong)
                                                .child(icon(IconName::LogOut, 16., p.error_strong))
                                                .child("ออกจากระบบ")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.store.update(cx, |s, cx| s.logout(cx))
                                                }))
                                        }),
                                ),
                            )
                            .child(
                                Self::section("รูปลักษณ์", &p)
                                    .child(Self::row(
                                        "ธีม",
                                        None,
                                        self.theme_control(settings.theme, cx, &p),
                                        &p,
                                    ))
                                    .child(Self::row(
                                        "โหมดกะทัดรัด",
                                        Some("แถวในรายการแชทเตี้ยลง เห็นได้มากขึ้น"),
                                        Switch::new("compact")
                                            .checked(settings.compact_mode)
                                            .on_click(cx.listener(
                                                |this, checked: &bool, _, cx| {
                                                    let v = *checked;
                                                    this.store.update(cx, |s, cx| {
                                                        s.update_settings(
                                                            |st| st.compact_mode = v,
                                                            cx,
                                                        )
                                                    });
                                                },
                                            )),
                                        &p,
                                    )),
                            )
                            .child(
                                Self::section("การแจ้งเตือน", &p)
                                    .child(Self::row(
                                        "แจ้งเตือนข้อความใหม่",
                                        Some("แสดงการแจ้งเตือนของระบบเมื่อมีข้อความในแชทที่ไม่ได้เปิดอยู่"),
                                        Switch::new("notify")
                                            .checked(settings.notifications_enabled)
                                            .on_click(cx.listener(
                                                |this, checked: &bool, _, cx| {
                                                    let v = *checked;
                                                    this.store.update(cx, |s, cx| {
                                                        s.update_settings(
                                                            |st| st.notifications_enabled = v,
                                                            cx,
                                                        )
                                                    });
                                                },
                                            )),
                                        &p,
                                    ))
                                    .child(Self::row(
                                        "แสดงตัวอย่างข้อความ",
                                        Some("ปิดไว้เพื่อซ่อนเนื้อหาในการแจ้งเตือน"),
                                        Switch::new("preview")
                                            .checked(settings.show_preview)
                                            .on_click(cx.listener(
                                                |this, checked: &bool, _, cx| {
                                                    let v = *checked;
                                                    this.store.update(cx, |s, cx| {
                                                        s.update_settings(
                                                            |st| st.show_preview = v,
                                                            cx,
                                                        )
                                                    });
                                                },
                                            )),
                                        &p,
                                    )),
                            )
                            .child(Self::section("พื้นที่จัดเก็บข้อมูลบนเครื่อง", &p).child({
                                let stats = crate::cache::inspect(&crate::env::data_dir());
                                let db_size = crate::cache::format_bytes(stats.db_bytes);
                                let log_size = crate::cache::format_bytes(stats.log_bytes);
                                Self::row(
                                    "ฐานข้อมูลและไฟล์ Log",
                                    Some("ฐานข้อมูลแชทออฟไลน์และบันทึกการทำงานบนเครื่อง"),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(12.))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.ink_secondary)
                                                .font_features(tnum())
                                                .child(format!("DB {db_size} · Log {log_size}")),
                                        )
                                        .child(
                                            div()
                                                .id("clear-logs")
                                                .h(px(28.))
                                                .px(px(12.))
                                                .flex()
                                                .items_center()
                                                .rounded(px(8.))
                                                .border_1()
                                                .border_color(p.hairline)
                                                .bg(p.surface_muted)
                                                .cursor_pointer()
                                                .text_size(px(12.))
                                                .text_color(p.ink)
                                                .hover(|s| s.bg(p.divider))
                                                .child("ล้าง Log เก่า")
                                                .on_click(cx.listener(|_, _, _, cx| {
                                                    crate::cache::clear_old_logs(
                                                        &crate::env::data_dir(),
                                                    );
                                                    cx.notify();
                                                })),
                                        ),
                                    &p,
                                )
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(12.))
                                    .text_color(p.ink_subtitle)
                                    .font_features(tnum())
                                    .child(format!(
                                        "{} {}",
                                        crate::env::APP_NAME,
                                        env!("CARGO_PKG_VERSION")
                                    )),
                            ),
                    ),
            )
    }
}
