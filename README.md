# Pinto Chat Desktop (GPUI)

Native rewrite of the Tauri + Vue app on [GPUI Kit](https://gpui-kit.com)
(`gpui-kit` 0.10: GPUI, GPUI Base, GPUI Component). One Rust binary, no web view.

## Run

```bash
cargo run
```

Configuration is read from the environment after loading `.env` in the working
directory (copy `.env.example`). Both new and old names work:

| Variable | Old name | Default |
| --- | --- | --- |
| `PINTO_API_BASE_URL` | `VITE_PINTO_API_BASE_URL` | `https://api-dev.pinto-app.com` |
| `PINTO_WS_URL` | `VITE_PINTO_WS_URL` | derived from the API URL |
| `PINTO_QR_AUTH_WS_URL` | `VITE_PINTO_QR_AUTH_WS_URL` | `<api as ws>/ws/auth/qr` |

```bash
cargo test                  # db, realtime parsing, formatting
cargo build --release       # optimized binary in target/release/pinto-chat
```

### Preview mode (debug builds only)

Sample data, no network, no reads of the real session or database:

```bash
PINTO_PREVIEW=list cargo run        # chat list
PINTO_PREVIEW=room cargo run        # open conversation
PINTO_PREVIEW=room-dark cargo run   # same, dark theme
PINTO_PREVIEW=settings cargo run    # settings page
```

## Install a built copy

GitHub Actions builds every push to `main` (files under the run's Artifacts) and every `v*` tag (files on the Release page).

**macOS:** the app is not signed with an Apple Developer ID, so macOS blocks it the first time ("damaged" or "unidentified developer"). Unzip it, move `Pinto Chat.app` to Applications, then clear the quarantine flag once:

```bash
xattr -dr com.apple.quarantine "/Applications/Pinto Chat.app"
```

**Windows:** unzip and run `Pinto Chat.exe`. SmartScreen may warn about an unknown publisher; choose More info, then Run anyway.

## Layout

```
src/
├── main.rs        app start, bundled assets and fonts, window
├── env.rs         configuration and constants (was app/env.ts, constants.ts)
├── models.rs      Chat, Message, UserProfile + normalizers
├── api.rs         blocking HTTP client (ureq), call from background tasks
├── db.rs          SQLite (rusqlite), same schema as the Tauri build
├── persist.rs     session.json and settings.json (was localStorage)
├── realtime.rs    chat and QR WebSockets on background threads
├── store.rs       app state (was the Pinia stores)
├── format.rs      Thai date, time and size formatting
├── theme.rs       DESIGN.md palette, light and dark, mapped onto gpui-component
└── ui/
    ├── mod.rs     root: screen switching, theme, toasts, notifications
    ├── login.rs   QR login and email/password
    ├── shell.rs   rail, chat list, user menu
    ├── room.rs    header, messages, bubbles, menus, composer
    ├── settings.rs
    └── widgets.rs avatar, icons, badges, drag regions
```

Data lives in `~/Library/Application Support/com.pinto.chat-desktop/`, the
same folder the Tauri build used, so an existing `pinto-chat.sqlite` opens as is.

## Architecture

- **Local-first.** Chats and messages are written to SQLite first; the store's
  lists are a view of it. Opening a chat reads SQLite and only asks the API when
  the cache is empty.
- **Threads.** HTTP runs on GPUI's background executor. Each WebSocket has its own
  thread and reports over a channel that a GPUI task drains on the UI thread.
- **Outbox.** Sends are queued in `pending_outbox`, marked failed on error, and
  retried on sign-in and every reconnect. A failed bubble offers a retry.

## Known limitations

- Thai line breaking happens at spaces only (ICU4X ships no Thai segmenter in this
  build), so a very long Thai run without spaces wraps between characters.
- The composer is a single-line input; Enter sends.
- OS notifications need a bundled `.app`; `cargo run` only shows in-app state.
- Mute, pinned message, wallpaper, the media/files/links panel and chat export
  are stored on this device only and do not sync.
- Friends, add friend, open chats, forward, invite, poll, album, report and
  block are hidden until their API calls are wired.
