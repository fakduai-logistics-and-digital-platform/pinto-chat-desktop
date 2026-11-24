//! Root view: picks the screen (login, chat shell, settings), keeps the theme
//! in step with the setting, and shows toasts and system notifications.

mod emoji;
mod login;
mod room;
mod settings;
mod shell;
mod widgets;

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{WindowExt, notification::Notification};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::{
    store::{Phase, Store, StoreEvent},
    theme::{self, Palette, PaletteExt},
    ui::widgets::{drag_region, icon},
};
use login::LoginView;
use settings::{SettingsEvent, SettingsView};
use shell::{ShellEvent, ShellView};

enum Screen {
    Restoring,
    Login(Entity<LoginView>),
    Shell {
        shell: Entity<ShellView>,
        settings: Option<Entity<SettingsView>>,
    },
}

pub struct PintoApp {
    store: Entity<Store>,
    screen: Screen,
    dark: bool,
    toast: Option<SharedString>,
    toast_epoch: u64,
}

impl PintoApp {
    pub fn new(store: Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.subscribe_in(&store, window, Self::on_store_event)
            .detach();
        cx.observe_in(&store, window, |this, _, window, cx| {
            this.sync_screen(window, cx);
            this.sync_theme(window, cx);
        })
        .detach();
        cx.observe_window_appearance(window, |this, window, cx| this.sync_theme(window, cx))
            .detach();
        if !(cfg!(debug_assertions) && std::env::var("PINTO_PREVIEW").is_ok()) {
            cx.observe_window_bounds(window, |_, window, _| {
                let b = window.bounds();
                let state = crate::persist::WindowState {
                    x: f32::from(b.origin.x),
                    y: f32::from(b.origin.y),
                    width: f32::from(b.size.width),
                    height: f32::from(b.size.height),
                };
                if state.width >= 460.0 && state.height >= 480.0 {
                    state.save();
                }
            })
            .detach();
        }

        // Opening the window syncs gpui-component's theme to the system, which
        // drops our overrides; install the palette again now the window exists.
        let dark = theme::resolve_dark(store.read(cx).settings.theme, Some(window));
        theme::apply(dark, Some(window), cx);
        let mut this = Self {
            store,
            screen: Screen::Restoring,
            dark,
            toast: None,
            toast_epoch: 0,
        };
        this.sync_screen(window, cx);
        this
    }

    fn sync_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = self.store.read(cx).settings.theme;
        let dark = theme::resolve_dark(mode, Some(window));
        if dark != self.dark || cx.palette().dark != dark {
            self.dark = dark;
            theme::apply(dark, Some(window), cx);
            window.refresh();
        }
    }

    fn sync_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let phase = self.store.read(cx).phase;
        match (phase, &self.screen) {
            (Phase::Restoring, _) => {}
            (Phase::SignedOut, Screen::Login(_)) | (Phase::SignedIn, Screen::Shell { .. }) => {}
            (Phase::SignedOut, _) => {
                let store = self.store.clone();
                self.screen = Screen::Login(cx.new(|cx| LoginView::new(store, window, cx)));
            }
            (Phase::SignedIn, _) => {
                let store = self.store.clone();
                let shell = cx.new(|cx| ShellView::new(store, window, cx));
                cx.subscribe_in(&shell, window, |this, _, ev: &ShellEvent, _, cx| match ev {
                    ShellEvent::OpenSettings => this.open_settings(cx),
                })
                .detach();
                self.screen = Screen::Shell {
                    shell,
                    settings: None,
                };
                #[cfg(debug_assertions)]
                if std::env::var("PINTO_PREVIEW").is_ok_and(|v| v.contains("settings")) {
                    self.open_settings(cx);
                }
            }
        }
        cx.notify();
    }

    fn open_settings(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        if let Screen::Shell { settings, .. } = &mut self.screen {
            let view = cx.new(|cx| SettingsView::new(store, cx));
            cx.subscribe(&view, |this, _, ev: &SettingsEvent, cx| match ev {
                SettingsEvent::Back => {
                    if let Screen::Shell { settings, .. } = &mut this.screen {
                        *settings = None;
                    }
                    cx.notify();
                }
            })
            .detach();
            *settings = Some(view);
        }
        cx.notify();
    }

    fn on_store_event(
        &mut self,
        _: &Entity<Store>,
        ev: &StoreEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match ev {
            StoreEvent::Toast(text) => self.show_toast(text.clone(), cx),
            StoreEvent::Notify { title, body } => {
                // Only interrupt when the window is in the background; the unread
                // badge covers the rest.
                if !window.is_window_active() {
                    window.push_notification(
                        Notification::new()
                            .title(title.clone())
                            .message(body.clone())
                            .system(),
                        cx,
                    );
                }
            }
            StoreEvent::SignedOut => {
                if let Screen::Shell { settings, .. } = &mut self.screen {
                    *settings = None;
                }
            }
            StoreEvent::SignedIn | StoreEvent::MessagesAppended => {}
        }
    }

    fn show_toast(&mut self, text: SharedString, cx: &mut Context<Self>) {
        self.toast = Some(text);
        self.toast_epoch += 1;
        let epoch = self.toast_epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(2200))
                .await;
            this.update(cx, |this, cx| {
                if this.toast_epoch == epoch {
                    this.toast = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

impl Render for PintoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let body = match &self.screen {
            Screen::Restoring => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    img("images/logo.png")
                        .size(px(56.))
                        .rounded(px(14.))
                        .opacity(0.9),
                )
                .into_any_element(),
            Screen::Login(v) => v.clone().into_any_element(),
            Screen::Shell {
                settings: Some(s), ..
            } => s.clone().into_any_element(),
            Screen::Shell { shell, .. } => shell.clone().into_any_element(),
        };
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.ink)
            .font_family(theme::FONT_FAMILY)
            .text_size(px(14.))
            .when(cfg!(target_os = "windows"), |d| {
                d.child(windows_titlebar(window, &p))
            })
            .child(div().flex_1().min_h_0().w_full().child(body))
            .when_some(self.toast.clone(), |d, text| {
                d.child(
                    div()
                        .absolute()
                        .bottom(px(96.))
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px(px(16.))
                                .py(px(8.))
                                .rounded_full()
                                .bg(p.overlay)
                                .text_color(p.surface)
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(text),
                        )
                        .with_animation(
                            ElementId::Name(format!("toast-{}", self.toast_epoch).into()),
                            Animation::new(Duration::from_millis(160))
                                .with_easing(ease_out_quint()),
                            |d, delta| d.opacity(delta).mb(px(8. * delta)),
                        ),
                )
            })
    }
}

/// Dedicated thin Windows title bar above the app content so caption controls
/// sit at the very top edge and never collide with the room header's `...` menu.
fn windows_titlebar(window: &Window, p: &Palette) -> impl IntoElement {
    let hover_bg = p.surface_muted;
    let max_icon = if window.is_maximized() {
        IconName::Copy
    } else {
        IconName::Square
    };

    div()
        .h(px(28.))
        .w_full()
        .flex_none()
        .bg(p.surface)
        .border_b_1()
        .border_color(p.hairline)
        .flex()
        .items_center()
        .child(drag_region(div().flex_1().h_full()))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .id("win-min")
                        .w(px(36.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .window_control_area(WindowControlArea::Min)
                        .hover(move |s| s.bg(hover_bg))
                        .child(icon(IconName::Minus, 13., p.ink_secondary))
                        .on_click(|_, window, _| window.minimize_window()),
                )
                .child(
                    div()
                        .id("win-max")
                        .w(px(36.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .window_control_area(WindowControlArea::Max)
                        .hover(move |s| s.bg(hover_bg))
                        .child(icon(max_icon, 11., p.ink_secondary))
                        .on_click(|_, window, _| window.zoom_window()),
                )
                .child(
                    div()
                        .id("win-close")
                        .w(px(36.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .window_control_area(WindowControlArea::Close)
                        .hover(|s| s.bg(rgb(0xE81123)).text_color(white()))
                        .child(icon(IconName::X, 13., p.ink_secondary))
                        .on_click(|_, _, cx| cx.quit()),
                ),
        )
}
