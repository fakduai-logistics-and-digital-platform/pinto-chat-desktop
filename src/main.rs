//! Pinto Chat Desktop, native build on GPUI Kit.

// Release builds on Windows open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod api;
mod db;
mod env;
mod format;
mod models;
mod persist;
mod realtime;
mod store;
mod theme;
mod ui;

use std::borrow::Cow;

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
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,naga=warn,wgpu=warn"),
    )
    .init();
    let config = env::Env::load();
    log::info!("API {}", config.api_base_url);

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
            let window_size = if narrow {
                size(px(470.), px(490.))
            } else {
                size(px(1200.), px(800.))
            };
            let bounds = Bounds::centered(None, window_size, cx);
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
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ui::PintoApp::new(store.clone(), window, cx))
            })
            .expect("open main window");
            cx.activate(true);
        });
}
