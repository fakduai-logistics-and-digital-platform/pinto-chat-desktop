# Pinto Chat Desktop

Desktop chat application built with **Tauri v2 + Vue 3 + TypeScript + Pinia + SQLite + WebSocket**.

Supports Windows, macOS, and Linux.

## Prerequisites

- **Node.js** >= 18
- **Rust** >= 1.70
- **Tauri CLI** (`cargo install tauri-cli --version "^2"`)

### Platform dependencies

**macOS:**
```bash
xcode-select --install
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Windows:**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## Setup

```bash
# Install npm dependencies
npm install

# Copy env file (defaults to dev API)
cp .env.example .env

# Run in development mode (starts Vite + Tauri)
npm run tauri dev

# Build for production
npm run tauri build
```

## Environment Variables

All API config is in `.env` (gitignored). Copy `.env.example` to `.env`.

| Variable | Default | Description |
|---|---|---|
| `VITE_PINTO_API_BASE_URL` | `https://api-dev.pinto-app.com` | REST API base URL |
| `VITE_PINTO_WS_URL` | `wss://api-dev.pinto-app.com` | WebSocket base URL (chat realtime) |
| `VITE_PINTO_QR_AUTH_WS_URL` | _(derived)_ | WebSocket URL for QR login. If empty, derived from `VITE_PINTO_API_BASE_URL` by replacing `http(s)` → `ws(s)` and appending `/ws/auth/qr` |

Source of truth: `src/app/env.ts` — all modules import from here, never from `.env` directly.

## Project Structure

```
pinto-chat-desktop/
├── src/                    # Vue 3 frontend
│   ├── app/                # Router, constants, env config
│   ├── features/           # Feature modules (auth, chats, messages, etc.)
│   ├── layouts/            # App shell and auth layouts
│   ├── pages/              # Route-level pages (Login, Chat, Settings)
│   └── shared/             # Reusable components, utils, API client
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/
│   ├── capabilities/       # Permission capabilities
│   └── tauri.conf.json     # Tauri configuration
├── tailwind.config.js
├── vite.config.ts
└── package.json
```

## Architecture

- **Local-first**: Messages stored in SQLite, not loaded from API each time
- **Realtime**: WebSocket for new messages, typing indicators, online status
- **Offline outbox**: Failed messages queued and retried on reconnect
- **Pinia**: UI state only (selected chat, connection status, drafts)
- **SQLite**: Source of truth for messages, chats, participants, outbox

## Login

Two login modes are supported:

- **Scan QR Code**: Binds a waiting desktop WebSocket session to an already-logged-in mobile user. The backend issues web/admin tokens after validating the mobile user's Bearer token.
- **Email / Password**: Traditional email or username + password login via `POST /api/auth/login`.

Both flows store tokens using the same path (`_pt_token` key with `pinto_access_token` fallback for migration).

### QR Login Flow (WebSocket)

QR login is **not autonomous auth** — it requires a mobile user already logged into Pinto.

1. Desktop opens WebSocket to `ws://<host>/ws/auth/qr`
2. Backend sends `{type:'init', session_id, expires_in}` — this is a waiting session, not a credential
3. Desktop renders QR with payload `{type_mode:'scan_login', session_id}`
4. Mobile user (already logged in) scans QR with Pinto app
5. Mobile sends `POST /api/auth/qr/confirm` with its own `Authorization: Bearer <mobile_token>` and body `{session_id}`
6. Backend validates mobile token + session, then issues web/admin tokens for the desktop
7. Backend pushes `{type:'login_success', access_token, refresh_token}` to the desktop's WS
8. Desktop stores tokens (same `_pt_token` key as email/password login) and fetches profile
9. On `expired`, desktop auto-reconnects and gets a fresh session

## API Endpoints

All requests go through `src/shared/api/api-client.ts` with JWT Bearer auth.

### Authentication

| Method | Endpoint | Purpose |
|---|---|---|
| POST | `/api/auth/login` | Login with email/username + password |
| POST | `/api/auth/create-account` | Register new account |
| POST | `/api/auth/refresh` | Refresh JWT token |
| POST | `/api/auth/logout` | Invalidate session |
| GET | `/v1/users/profile` | Get current user profile |

### QR Login

| Method | Endpoint | Auth | Body | Purpose |
|---|---|---|---|---|
| WS | `ws://<host>/ws/auth/qr` | — | — | Desktop waits for mobile confirmation. Backend sends `init`, `login_success`, `expired` |
| POST | `/api/auth/qr/confirm` | `Bearer <mobile_token>` | `{session_id}` | Mobile (logged-in user) confirms scan. Backend validates mobile token + session, issues web/admin tokens, pushes `login_success` to desktop WS |

### Chat

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/v1/chats` | List chats (paginated, filterable) |
| POST | `/v1/chats` | Create direct/group chat |
| GET | `/v1/chats/{id}` | Get chat details |
| DELETE | `/v1/chats/{id}` | Soft-delete chat |
| PUT | `/v1/chats/{id}/name` | Rename group chat |
| PUT | `/v1/chats/{id}/avatar` | Update group avatar |

### Messages

| Method | Endpoint | Purpose |
|---|---|---|
| POST | `/v1/chats/{id}/messages` | Send text/image/file message |
| GET | `/v1/chats/{id}/messages` | Load message history (migration only) |
| POST | `/v1/chats/{id}/messages/read-all` | Mark all messages read |
| POST | `/v1/chats/messages/{id}/read` | Mark single message read |

### Realtime

| Method | Endpoint | Purpose |
|---|---|---|
| GET | `/v1/ws/chat` | WebSocket connection |
| POST | `/v1/chats/typing` | Send typing indicator |
| GET | `/v1/chats/online-status` | Get user online status |

### Group Participants

| Method | Endpoint | Purpose |
|---|---|---|
| POST | `/v1/chats/{id}/participants` | Add member to group |
| DELETE | `/v1/chats/{id}/participants` | Remove member from group |

## Development

```bash
# Type check
npx vue-tsc --noEmit

# Build frontend only
npx vite build

# Build full Tauri app
npm run tauri build
```

## Quick Start

```bash
# Full setup from scratch
npm install
cp .env.example .env
npm run tauri dev

# Verify everything compiles
npx vue-tsc --noEmit && npx vite build
```

## License

Private - Pinto
