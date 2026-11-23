# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

```bash
cp .env.example .env
cargo run
```

## Development Commands

```bash
cargo run                     # debug build + app window
cargo test                    # unit tests (db, realtime parsing, formatting)
cargo clippy                  # lints; keep at zero warnings
cargo fmt                     # format
cargo build --release         # optimized binary: target/release/pinto-chat
```

**Preview mode** (debug builds only): sample data, no network, never reads the real session or database. Use it to check UI without signing in.

```bash
PINTO_PREVIEW=list cargo run        # chat list
PINTO_PREVIEW=room cargo run        # open conversation
PINTO_PREVIEW=room-dark cargo run   # dark theme
PINTO_PREVIEW=settings cargo run    # settings page
```

## Project Architecture

**Pinto Chat Desktop** is a local-first, realtime chat app: one native Rust binary built on **GPUI Kit** (`gpui-kit` 0.10 = GPUI + GPUI Base + GPUI Component), SQLite for local data, WebSocket for realtime. It replaced an earlier Tauri + Vue build (still in git history before the move).

### Stack
- **UI**: `gpui-kit` (`use gpui_kit::*;` is GPUI; components under `gpui_kit::component`, Lucide icons via `gpui_kit::assets::IconName`)
- **HTTP**: `ureq` (blocking), always called inside `cx.background_spawn`
- **Realtime**: `tungstenite` on dedicated threads, events sent over `futures` channels
- **Database**: `rusqlite` (bundled SQLite)
- **Auth**: JWT (email/password or QR confirmation from the mobile app)

### Architectural rules

**Local-first source of truth**: chats and messages are written to SQLite first. `Store`'s in-memory lists are a view for rendering. Opening a chat reads SQLite; the API is only asked when the cache is empty.

**One state entity**: `store::Store` (an `Entity<Store>`) replaces the old Pinia stores. Views hold `Entity<Store>`, observe it, and call its methods. Views keep UI-only state (scroll handles, input entities, open menus, reply target).

**Threading**: never block the UI thread. Network calls go through `cx.background_spawn(...)` and results are applied with `this.update(cx, ...)`. WebSocket threads never touch UI state; a GPUI task drains their channel and calls into `Store`.

**Realtime flow**: WS frame → `Store::on_frame` → write SQLite → update in-memory view → `cx.notify()` / `StoreEvent`.

**Offline outbox**: text sends go to `pending_outbox` before the request; failures mark the row and message `failed`; `flush_outbox` retries on sign-in and every reconnect. Failed bubbles offer a retry.

## Directory Structure

```
src/
├── main.rs        app start, bundled assets and fonts, window options
├── env.rs         configuration + constants (source of truth for URLs)
├── models.rs      Chat, Message, UserProfile, normalizers from API JSON
├── api.rs         HTTP client and endpoints
├── db.rs          SQLite schema, migrations and queries (+ tests)
├── persist.rs     session.json and settings.json in the app data dir
├── realtime.rs    chat and QR WebSocket threads (+ tests)
├── store.rs       app state: auth, chats, messages, settings, images
├── format.rs      Thai date/time/size formatting (+ tests)
├── theme.rs       DESIGN.md palette (light + dark) mapped onto gpui-component
└── ui/
    ├── mod.rs     root view: screen switching, theme sync, toasts, notifications
    ├── login.rs   QR login and email/password
    ├── shell.rs   rail, chat list, user menu
    ├── room.rs    header, messages, bubbles, menus, composer
    ├── settings.rs
    └── widgets.rs avatar, icon, badges, drag regions
assets/
├── fonts/         IBM Plex Sans Thai (Regular, Medium, SemiBold), embedded
├── images/        logo, chat wallpaper pattern, embedded
└── icons/         app icons (icns/ico/png) for bundling
```

## Key Implementation Details

### Environment
`env::Env::load()` reads `.env` then the process env. Names: `PINTO_API_BASE_URL`, `PINTO_WS_URL`, `PINTO_QR_AUTH_WS_URL` (old `VITE_PINTO_*` names are still accepted). WS and QR URLs are derived from the API URL when unset. With nothing set, the API defaults to dev, or to `PINTO_BUILD_API_BASE_URL` read at compile time (CI sets it to `https://api.pinto-app.com`, so built apps use production).

### Data location
`~/Library/Application Support/com.pinto.chat-desktop/` (same identifier as the old Tauri build, so existing `pinto-chat.sqlite` opens as is): `pinto-chat.sqlite`, `session.json` (tokens), `settings.json`.

### Authentication
- **Email/password**: `POST /api/auth/login` → store tokens → `GET /v1/users/profile`.
- **QR**: `realtime::spawn_qr_socket` opens `/ws/auth/qr` → `init` gives `session_id` + expiry → render QR of `{type_mode:'scan_login', session_id}` → mobile confirms via `/api/auth/qr/confirm` → `login_success` delivers tokens → `Store::login_with_tokens`. Expired sessions reconnect after 3 s.

### Database schema
Created in `db::MIGRATIONS` (idempotent `CREATE ... IF NOT EXISTS`): `app_meta`, `users`, `chats`, `chat_participants`, `messages`, `message_read_states`, `pending_outbox`, `typing_states`, `bot_generating_states`. Add a table by appending to `MIGRATIONS` and adding query methods on `Db`, with a test.

### WebSocket
`/v1/ws/chat?token=...`, frames `{ type, data }`: `new_message`, `typing`, `stop_typing`, `read_receipt`. Reconnect delays 1/2/5/10/30 s.

### Images
Remote avatars and image messages are fetched by `Store::image(url, cx)` (bytes via `Api::fetch_bytes`, cached as `Arc<Image>`); it returns `None` until loaded.

## Design System

**Read DESIGN.md before editing UI.** Colors, type and spacing come from there; `theme.rs` holds them as `Palette` (read with `cx.palette()`).

- Primary `#2ECC71`, tint `#EAFBF1`; warm ink text (`#433D4D` ramp); paper-flat, hairline borders, no glows or gradients.
- Font: IBM Plex Sans Thai, weights 400/500/600.
- Spacing on a 4px grid; icons are Lucide at one size per context.
- Own message bubbles use dark ink on the green (white on `#2ECC71` fails contrast).
- After changing `theme.rs`, rebuild `theme.tokens` from `colors` (already done in `theme::apply`) or components keep stale colors.
- Verify UI changes with preview mode and a screenshot.

## Common Tasks

**Add an API endpoint**: method on `Api` in `api.rs` returning `ApiResult<T>`; call it from `Store` inside `cx.background_spawn`.

**Add a screen**: view in `src/ui/`, add a variant to `Screen` in `ui/mod.rs`, navigate via an event the root subscribes to.

**Debug realtime**: run with `RUST_LOG=debug cargo run`; socket errors log from `realtime.rs`.

## Known Limitations / TODOs

- Thai line breaking only at spaces (no ICU4X Thai segmenter in this build).
- Composer is single-line (Enter sends).
- OS notifications need an `.app` bundle; not set up yet.
- No sync conflict resolution (last write wins), no encryption at rest, no message search.
- Local-only (this device, no sync): chat mute, pinned message, wallpaper, media/files/links panel, chat export. The API has endpoints to sync mute and media later.
- Hidden until the API is wired: friends, add friend, open chats, forward, invite, poll, album, report, block.
- Outbox auto-resend covers messages under 24 h with fewer than 5 attempts, one at a time, stopping on 429; older ones wait for a manual retry.
