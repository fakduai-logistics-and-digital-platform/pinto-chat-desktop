//! Pinto Chat Desktop, native build on GPUI Kit.

// Release builds on Windows open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod api;
mod cache;
mod db;
mod deeplink;
mod env;
mod format;
mod logging;
mod models;
mod persist;
mod realtime;
mod single_instance;
mod store;
mod theme;
mod ui;

use std::borrow::Cow;

use futures::StreamExt;
use gpui_kit::*;

/// Bundled images plus the complete Lucide icon catalog.
struct AppAssets;

const OWN_ASSETS: &[(&str, &[u8])] = &[
    (
        "images/logo.png",
        include_bytes!("../assets/images/logo-128.png"),
    ),
    (
        "images/chat-pattern.webp",
        include_bytes!("../assets/images/chat-pattern-transparent.webp"),
    ),
];

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = OWN_ASSETS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        gpui_kit::assets::AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out: Vec<SharedString> = OWN_ASSETS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| (*p).into())
            .collect();
        out.extend(gpui_kit::assets::AllAssets.list(path)?);
        Ok(out)
    }
}

const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/IBMPlexSansThai-Regular.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSansThai-Medium.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSansThai-SemiBold.ttf"),
];

actions!(pinto, [Quit]);

fn main() {
    let is_preview = cfg!(debug_assertions) && std::env::var("PINTO_PREVIEW").is_ok();
    let data_dir = env::data_dir();
    logging::init(&data_dir, !is_preview);

    let first_arg = std::env::args().nth(1);
    let instance_state = if !is_preview {
        match single_instance::acquire(&data_dir, first_arg.as_deref()) {
            Ok(single_instance::AcquireResult::SecondaryNotified) => {
                log::info!("Another instance is already running; sent activation command.");
                return;
            }
            Ok(single_instance::AcquireResult::Primary {
                _guard,
                activations,
            }) => Some((_guard, activations)),
            Err(err) => {
                log::warn!("Single-instance guard unavailable: {err:#}");
                None
            }
        }
    } else {
        None
    };

    let config = env::Env::load();
    log::info!("API {}", config.api_base_url);

    let (_instance_guard, activations) = match instance_state {
        Some((g, rx)) => (Some(g), Some(rx)),
        None => (None, None),
    };

    gpui_kit::application()
        .with_assets(AppAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            if let Err(e) = cx
                .text_system()
                .add_fonts(FONTS.iter().map(|f| Cow::Borrowed(*f)).collect())
            {
                log::warn!("load bundled fonts: {e:#}");
            }
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.set_menus(vec![Menu {
                name: env::APP_NAME.into(),
                items: vec![MenuItem::action("Quit", Quit)],
                disabled: false,
            }]);

            let store = cx.new(|cx| store::Store::new(config.clone(), cx));
            #[cfg(debug_assertions)]
            if std::env::var("PINTO_PREVIEW").is_ok() {
                store.update(cx, |s, cx| s.seed_preview(cx));
            }
            let settings_theme = store.read(cx).settings.theme;
            theme::apply(theme::resolve_dark(settings_theme, None), None, cx);

            let narrow = cfg!(debug_assertions)
                && std::env::var("PINTO_PREVIEW").is_ok_and(|v| v.contains("narrow"));
            let bounds = if narrow {
                Bounds::centered(None, size(px(470.), px(490.)), cx)
            } else if !is_preview && let Some(ws) = persist::WindowState::load() {
                Bounds::new(point(px(ws.x), px(ws.y)), size(px(ws.width), px(ws.height)))
            } else {
                Bounds::centered(None, size(px(1200.), px(800.)), cx)
            };
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(env::APP_NAME.into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(9.), px(9.))),
                }),
                window_min_size: Some(size(px(460.), px(480.))),
                app_id: Some(env::APP_IDENTIFIER.into()),
                ..Default::default()
            };
            let window_handle = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ui::PintoApp::new(store.clone(), window, cx))
            })
            .expect("open main window");
            cx.activate(true);

            if let Some(deeplink::DeepLink::OpenChat(chat_id)) =
                first_arg.as_deref().and_then(deeplink::parse)
            {
                store.update(cx, |s, cx| s.select_chat(Some(chat_id), cx));
            }

            if let Some(mut rx) = activations {
                let store = store.clone();
                cx.spawn(async move |cx| {
                    while let Some(arg) = rx.next().await {
                        log::info!("Activation requested from secondary instance: {arg:?}");
                        cx.update(|cx| {
                            if let Some(deeplink::DeepLink::OpenChat(chat_id)) =
                                arg.as_deref().and_then(deeplink::parse)
                            {
                                store.update(cx, |s, cx| s.select_chat(Some(chat_id), cx));
                            }
                            cx.activate(true);
                            let _ = window_handle.0.update(cx, |_, window, _| {
                                window.activate_window();
                            });
                        });
                    }
                })
                .detach();
            }
        });
}
