//! Login screen, ported from `pages/LoginPage.vue`: QR code first, email and
//! password as the alternative.

use std::time::Duration;

use futures::StreamExt;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use qrcode::{EcLevel, QrCode};

use crate::{
    env::QR_MAX_LIFETIME_SECS,
    format,
    realtime::{self, QrEvent, SocketHandle},
    store::Store,
    theme::{Palette, PaletteExt},
    ui::widgets::{drag_region, icon, tnum},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Qr,
    Email,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QrStatus {
    Connecting,
    Pending,
    Confirmed,
    Expired,
    Error,
}

pub struct LoginView {
    store: Entity<Store>,
    mode: Mode,
    qr_status: QrStatus,
    qr_error: Option<String>,
    qr_modules: Option<(usize, Vec<bool>)>,
    countdown: u64,
    countdown_epoch: u64,
    socket: Option<SocketHandle>,
    email: Entity<InputState>,
    password: Entity<InputState>,
    email_error: Option<&'static str>,
    password_error: Option<&'static str>,
}

impl LoginView {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("รหัสผ่าน")
                .masked(true)
        });
        cx.subscribe_in(&password, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.submit(window, cx);
            }
        })
        .detach();
        cx.subscribe_in(&email, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.password.update(cx, |s, cx| s.focus(window, cx));
            }
        })
        .detach();
        cx.observe(&store, |_, _, cx| cx.notify()).detach();

        let mut this = Self {
            store,
            mode: Mode::Qr,
            qr_status: QrStatus::Connecting,
            qr_error: None,
            qr_modules: None,
            countdown: 0,
            countdown_epoch: 0,
            socket: None,
            email,
            password,
            email_error: None,
            password_error: None,
        };
        this.start_qr(cx);
        this
    }

    fn set_mode(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.email_error = None;
        self.password_error = None;
        match mode {
            Mode::Qr => self.start_qr(cx),
            Mode::Email => {
                self.stop_qr();
                self.email.update(cx, |s, cx| s.focus(window, cx));
            }
        }
        cx.notify();
    }

    fn stop_qr(&mut self) {
        self.socket = None;
        self.countdown_epoch += 1;
        self.countdown = 0;
    }

    fn start_qr(&mut self, cx: &mut Context<Self>) {
        self.stop_qr();
        self.qr_status = QrStatus::Connecting;
        self.qr_error = None;
        self.qr_modules = None;
        let url = self.store.read(cx).env.qr_auth_ws_url.clone();
        let (handle, mut rx) = realtime::spawn_qr_socket(url);
        self.socket = Some(handle);
        cx.spawn(async move |this, cx| {
            while let Some(event) = rx.next().await {
                if this.update(cx, |this, cx| this.on_qr(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn on_qr(&mut self, event: QrEvent, cx: &mut Context<Self>) {
        if self.socket.is_none() {
            return;
        }
        match event {
            QrEvent::Connecting => {
                if self.qr_modules.is_none() {
                    self.qr_status = QrStatus::Connecting;
                }
            }
            QrEvent::Open => {}
            QrEvent::SessionReady {
                session_id,
                expires_in,
            } => {
                self.qr_error = None;
                self.qr_status = QrStatus::Pending;
                self.qr_modules = QrCode::with_error_correction_level(
                    realtime::qr_payload(&session_id),
                    EcLevel::M,
                )
                .ok()
                .map(|code| {
                    let w = code.width();
                    (
                        w,
                        code.to_colors()
                            .into_iter()
                            .map(|c| c == qrcode::Color::Dark)
                            .collect(),
                    )
                });
                if self.qr_modules.is_none() {
                    self.qr_error = Some("สร้าง QR Code ไม่สำเร็จ".into());
                }
                self.start_countdown(expires_in.min(QR_MAX_LIFETIME_SECS), cx);
            }
            QrEvent::LoginSuccess {
                access_token,
                refresh_token,
                user,
            } => {
                self.qr_status = QrStatus::Confirmed;
                self.stop_qr();
                self.store.update(cx, |s, cx| {
                    s.login_with_tokens(access_token, refresh_token, user, cx)
                });
            }
            QrEvent::Expired => {
                self.qr_status = QrStatus::Expired;
                self.countdown_epoch += 1;
            }
            QrEvent::Error(msg) => {
                self.qr_status = QrStatus::Error;
                self.qr_error = Some(msg);
            }
        }
        cx.notify();
    }

    fn start_countdown(&mut self, secs: u64, cx: &mut Context<Self>) {
        self.countdown_epoch += 1;
        let epoch = self.countdown_epoch;
        self.countdown = secs;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let keep = this
                    .update(cx, |this, cx| {
                        if this.countdown_epoch != epoch {
                            return false;
                        }
                        this.countdown = this.countdown.saturating_sub(1);
                        if this.countdown == 0 {
                            this.qr_status = QrStatus::Expired;
                        }
                        cx.notify();
                        this.countdown > 0
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
    }

    fn submit(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let email = self.email.read(cx).value().trim().to_string();
        let password = self.password.read(cx).value().to_string();
        self.email_error = email.is_empty().then_some("กรอกอีเมลหรือชื่อผู้ใช้");
        self.password_error = password.is_empty().then_some("กรอกรหัสผ่าน");
        cx.notify();
        if self.email_error.is_some() || self.password_error.is_some() {
            return;
        }
        self.store.update(cx, |s, cx| s.login(email, password, cx));
    }

    // ---- render ---------------------------------------------------------

    fn render_brand(&self, p: &Palette) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(img("images/logo.png").size(px(32.)).rounded(px(8.)))
            .child(
                div()
                    .flex()
                    .text_size(px(20.))
                    .line_height(px(28.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(div().text_color(p.primary_ink).child("Pinto"))
                    .child(div().text_color(p.ink).child("\u{00a0}Chat")),
            )
    }

    fn render_qr_code(&self, p: &Palette) -> AnyElement {
        let failed = matches!(self.qr_status, QrStatus::Expired | QrStatus::Error)
            || self.qr_error.is_some();
        let frame = div()
            .size(px(208.))
            .rounded(px(16.))
            .bg(p.surface)
            .border_1()
            .border_color(if failed { p.error_container } else { p.primary })
            .p(px(16.))
            .flex()
            .items_center()
            .justify_center();

        if failed {
            let title = if self.qr_status == QrStatus::Expired {
                "QR Code หมดอายุแล้ว"
            } else {
                "เชื่อมต่อไม่สำเร็จ"
            };
            return frame
                .child(
                    div()
                        .size_full()
                        .rounded(px(10.))
                        .bg(p.error_container)
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(12.))
                        .px(px(16.))
                        .child(
                            div()
                                .size(px(44.))
                                .rounded_full()
                                .bg(p.surface)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(IconName::TriangleAlert, 22., p.error_strong)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(4.))
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
                                        .text_color(p.ink_secondary)
                                        .child("สร้างรหัสใหม่เพื่อสแกนอีกครั้ง"),
                                ),
                        ),
                )
                .into_any_element();
        }

        if self.qr_status == QrStatus::Confirmed {
            return frame
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(48.))
                                .rounded_full()
                                .bg(p.primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(IconName::Check, 24., p.on_primary)),
                        )
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.primary_ink)
                                .child("ยืนยันแล้ว"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.ink_secondary)
                                .child("กำลังเปิดแชท"),
                        ),
                )
                .into_any_element();
        }

        let Some((width, modules)) = &self.qr_modules else {
            // Connecting: a quiet placeholder of the right size instead of a spinner.
            return frame
                .child(
                    div()
                        .size_full()
                        .rounded(px(8.))
                        .bg(p.surface_muted)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(p.ink_subtitle)
                        .child(if self.qr_status == QrStatus::Connecting {
                            "กำลังเชื่อมต่อ"
                        } else {
                            "กำลังสร้าง QR Code"
                        }),
                )
                .into_any_element();
        };

        let cell = 176. / *width as f32;
        let ink = if p.dark {
            gpui_kit::rgb(0x433D4D).into()
        } else {
            p.ink
        };
        frame
            .bg(white())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .children(modules.chunks(*width).map(|row| {
                        div().flex().children(
                            row.iter()
                                .map(|dark| div().size(px(cell)).when(*dark, |d| d.bg(ink))),
                        )
                    })),
            )
            .into_any_element()
    }

    fn render_qr(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let failed = matches!(self.qr_status, QrStatus::Expired | QrStatus::Error)
            || self.qr_error.is_some();
        let live = self.countdown > 0 && self.qr_status == QrStatus::Pending && !failed;

        div()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .mb(px(32.))
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink)
                            .child("สแกน QR Code ด้วยแอป Pinto"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(p.ink_secondary)
                            .child("เปิดแอป Pinto ในมือถือแล้วสแกนรหัสนี้เพื่อเข้าสู่ระบบ"),
                    ),
            )
            .child(self.render_qr_code(p))
            .child(
                div()
                    .h(px(64.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(live, |d| {
                        d.gap(px(8.))
                            .text_size(px(14.))
                            .child(icon(IconName::Timer, 16., p.ink_subtitle))
                            .child(div().text_color(p.ink_secondary).child("หมดอายุใน"))
                            .child(
                                div()
                                    .w(px(36.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.ink)
                                    .font_features(tnum())
                                    .child(format::countdown(self.countdown)),
                            )
                    })
                    .when(failed, |d| {
                        d.child(
                            Button::new("refresh-qr")
                                .icon(IconName::RefreshCw)
                                .label("สร้าง QR Code ใหม่")
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| this.start_qr(cx))),
                        )
                    }),
            )
            .when_some(
                self.qr_error
                    .clone()
                    .filter(|_| self.qr_status == QrStatus::Error),
                |d, e| {
                    d.child(
                        div()
                            .mb(px(8.))
                            .text_size(px(12.))
                            .text_color(p.ink_subtitle)
                            .max_w(px(320.))
                            .text_center()
                            .child(e),
                    )
                },
            )
            .child(
                div()
                    .id("to-email")
                    .text_size(px(14.))
                    .text_color(p.primary_ink)
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|s| s.underline())
                    .on_click(
                        cx.listener(|this, _, window, cx| this.set_mode(Mode::Email, window, cx)),
                    )
                    .child("เข้าสู่ระบบด้วยอีเมลแทน"),
            )
    }

    fn field(
        &self,
        label: &'static str,
        input: &Entity<InputState>,
        error: Option<&'static str>,
        masked: bool,
        busy: bool,
        p: &Palette,
    ) -> impl IntoElement {
        let mut field = Input::new(input).disabled(busy).large();
        if masked {
            field = field.mask_toggle();
        }
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if error.is_some() {
                        p.error_strong
                    } else {
                        p.ink
                    })
                    .child(label),
            )
            .child(field)
            .when_some(error, |d, e| {
                d.child(
                    div()
                        .mt(px(-4.))
                        .text_size(px(12.))
                        .text_color(p.error_strong)
                        .child(e),
                )
            })
    }

    fn render_email(&self, cx: &mut Context<Self>, p: &Palette) -> impl IntoElement {
        let store = self.store.read(cx);
        let busy = store.auth_busy;
        let error = store.auth_error.clone();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(18.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink)
                            .child("เข้าสู่ระบบ"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(p.ink_secondary)
                            .child("ใช้อีเมลหรือชื่อผู้ใช้ของบัญชี Pinto"),
                    ),
            )
            .when_some(error, |d, e| {
                d.child(
                    div()
                        .p(px(12.))
                        .rounded(px(8.))
                        .bg(p.error_container)
                        .text_size(px(14.))
                        .text_color(p.error_strong)
                        .child(e),
                )
            })
            .child(self.field(
                "อีเมลหรือชื่อผู้ใช้",
                &self.email,
                self.email_error,
                false,
                busy,
                p,
            ))
            .child(self.field("รหัสผ่าน", &self.password, self.password_error, true, busy, p))
            .child(
                Button::new("login")
                    .primary()
                    .large()
                    .w_full()
                    .label("เข้าสู่ระบบ")
                    .loading(busy)
                    .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
            )
            .child(
                div().flex().justify_center().child(
                    div()
                        .id("to-qr")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(14.))
                        .text_color(p.primary_ink)
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(|s| s.underline())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.set_mode(Mode::Qr, window, cx)),
                        )
                        .child(icon(IconName::Smartphone, 16., p.primary_ink))
                        .child("สแกน QR Code แทน"),
                ),
            )
    }
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let body = match self.mode {
            Mode::Qr => self.render_qr(cx, &p).into_any_element(),
            Mode::Email => self.render_email(cx, &p).into_any_element(),
        };
        div()
            .size_full()
            .bg(p.background)
            .flex()
            .flex_col()
            .child(drag_region(div().h(px(52.)).flex_none())) // room for the window controls
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(px(16.))
                    .child(
                        div()
                            .w(px(400.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(32.))
                            .child(self.render_brand(&p))
                            .child(
                                div()
                                    .w_full()
                                    .bg(p.surface)
                                    .rounded(px(16.))
                                    .border_1()
                                    .border_color(p.hairline)
                                    .px(px(32.))
                                    .py(px(32.))
                                    .child(body),
                            ),
                    ),
            )
            .child(div().h(px(52.)).flex_none())
    }
}
