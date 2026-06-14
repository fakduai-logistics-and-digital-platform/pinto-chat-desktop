# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Quick Start

```bash
npm install
cp .env.example .env
npm run tauri dev
```

## Development Commands

**Development mode** (Vite dev server + Tauri app window):
```bash
npm run tauri dev
```

**Type check only**:
```bash
npx vue-tsc --noEmit
```

**Build frontend only**:
```bash
npx vite build
```

**Build full production app**:
```bash
npm run tauri build
```

## Project Architecture

**Pinto Chat Desktop** is a local-first, realtime chat application built with Tauri v2 (Rust backend) + Vue 3 (TypeScript frontend) + SQLite (local data store) + WebSocket (realtime updates).

### Stack Overview
- **Frontend**: Vue 3 + Vue Router + Pinia (UI state management)
- **Build**: Vite + vue-tsc type checking
- **Styling**: Tailwind CSS
- **Desktop**: Tauri v2 (Rust runtime, provides OS-level APIs)
- **Database**: SQLite (local-first, not cloud-backed)
- **Realtime**: WebSocket (`/v1/ws/chat`)
- **Auth**: JWT tokens (email/password or QR-code mobile confirmation)

### Architectural Patterns

**Local-first source of truth**: All data (chats, messages, users) lives in SQLite. The API is for sync/realtime, not primary storage. Messages are fetched from the database, not the API.

**State separation**:
- **Pinia stores**: UI-only state (selected chat, connection status, loading flags, drafts)
- **SQLite**: Persistent domain data (messages, chats, participants, outbox)
- Never duplicate domain data in Pinia

**Realtime flow**: WebSocket pushes events → update SQLite → Pinia store reacts to DB changes. Never mutate Pinia from WS events directly.

**Offline outbox**: Failed sends are queued in SQLite's `outbox` table and retried on reconnect. Check `outbox.service.ts` for the pattern.

## Directory Structure

```
src/
├── app/
│   ├── router.ts           # Vue Router configuration
│   ├── env.ts              # Runtime environment variables (source of truth for API URLs)
│   └── constants.ts        # App-wide constants
├── features/               # Feature modules (auth, chats, messages, etc.)
│   ├── auth/
│   │   ├── auth.store.ts           # Pinia store (login state, current user)
│   │   ├── auth.api.ts             # API calls (login, logout, refresh)
│   │   ├── token-storage.ts        # Token persistence (localStorage/Tauri)
│   │   ├── qr-login.service.ts     # QR login WebSocket flow
│   │   └── auth.types.ts           # TypeScript types
│   ├── chats/
│   │   ├── chats.store.ts          # Pinia store (selected chat, chat list UI state)
│   │   ├── chats.api.ts            # Chat API calls
│   │   ├── chats.db.ts             # Chat database queries
│   │   └── chats.types.ts
│   ├── messages/
│   │   ├── messages.store.ts       # Pinia store (message list UI state)
│   │   ├── messages.api.ts         # Message API calls
│   │   ├── messages.db.ts          # Message database queries
│   │   ├── message-normalizer.ts   # Normalize/denormalize message data
│   │   ├── outbox.service.ts       # Offline queue and retry logic
│   │   └── messages.types.ts
│   ├── realtime/
│   │   ├── pinto-ws.client.ts      # WebSocket client (connect, listen, send)
│   │   └── ws-events.ts            # Event type definitions
│   ├── local-db/
│   │   ├── db.client.ts            # SQLite initialization and migrations
│   │   └── db.repositories.ts      # Database query layer
│   ├── users/
│   │   ├── users.api.ts
│   │   └── users.types.ts
│   ├── notifications/              # OS-level notifications (Tauri plugin)
│   └── settings/
│       └── settings.store.ts       # App settings (theme, notification prefs, etc.)
├── layouts/
│   ├── AuthLayout.vue      # Login page layout
│   └── AppShell.vue        # Main app shell with sidebar, chat view
├── pages/
│   ├── LoginPage.vue       # QR code + email/password login
│   ├── ChatPage.vue        # Main chat interface
│   └── SettingsPage.vue    # Settings UI
├── shared/
│   ├── api/
│   │   └── api-client.ts   # HTTP client (with JWT auth, error handling)
│   ├── components/         # Reusable Vue components (MessageBubble, ChatList, etc.)
│   └── utils/              # Utility functions (date formatting, text parsing, etc.)
├── App.vue                 # Root component (router view + global setup)
├── main.ts                 # Vue app entry point
└── style.css               # Global styles
```

## Key Implementation Details

### Environment Configuration
All API URLs come from `src/app/env.ts`, not `.env` directly. Modules import the `env` object from there.
- `VITE_PINTO_API_BASE_URL` — REST API base
- `VITE_PINTO_WS_URL` — WebSocket base for chat realtime
- `VITE_PINTO_QR_AUTH_WS_URL` — QR login WebSocket (auto-derived if not set)

### Authentication Flow

**Email/Password**: POST to `/api/auth/login` → store JWT token in localStorage/Tauri → fetch profile.

**QR Code**: 
1. Desktop opens WS to `/ws/auth/qr`
2. Backend sends `init` with session_id + QR expires
3. Desktop renders QR with `{type_mode: 'scan_login', session_id}`
4. Mobile (already logged in) scans and POSTs `/api/auth/qr/confirm` with its token
5. Backend validates mobile token, issues desktop tokens, pushes `login_success` to WS
6. Desktop stores tokens and fetches profile

See `src/features/auth/qr-login.service.ts` for implementation.

### Database Schema
Tauri SQL plugin manages SQLite. Migrations run on app init in `src/features/local-db/db.client.ts`:
- `users` — user profiles
- `chats` — conversations (DMs and groups)
- `messages` — message history
- `outbox` — unsent/failed messages
- `app_meta` — app metadata (last sync timestamp, etc.)

All queries go through `src/features/local-db/db.repositories.ts`.

### Message Flow
1. User types → update Pinia draft
2. Click send → create `outbox` row + POST to API
3. API succeeds → mark in `messages` table
4. API fails → leave in `outbox` + retry on reconnect
5. WebSocket pushes new message → insert into `messages` + update `chats.last_message_at`
6. Pinia store reacts to DB changes (queries are re-run)

See `src/features/messages/outbox.service.ts` for retry logic.

### WebSocket Connection
Connected in `App.vue` after auth succeeds. Reconnects on network restore. See `src/features/realtime/pinto-ws.client.ts`:
- `connect()` — open connection
- `on(eventType, handler)` — register event listener
- `send(message)` — push event to server
- Auto-reconnect on drop

### Store Pattern
All Pinia stores use the Composition API pattern (`defineStore('name', () => { ... })`). Stores are **UI state only**:
- Selected chat ID
- Loading/error flags
- Draft text
- Connection status

They do not duplicate domain data from SQLite. Instead, they subscribe to database changes (via `db.repositories`) and compute derived state via `computed()`.

## Design System

**Read DESIGN.md before editing UI.** All color, typography, and spacing decisions come from there.

- Primary color: `#2ECC71` (green)
- Primary tint: `#EAFBF1`
- Typography: IBM Plex Sans Thai (Display, Headline, Title scales)
- Design philosophy: Bright, paper-flat, warm-ink typography

When editing Vue templates or CSS:
- Design review happens automatically via `.claude/settings.json` hook
- Skills invoked: `/design-taste-frontend-v1` (editorial), `/industrial-brutalist-ui` (mechanical), `/brandkit` (brand)
- Focus on color consistency, typography hierarchy, and spacing alignment

Tailwind is configured with custom Pinto colors in `tailwind.config.js`.

## Testing & Verification

No test suite exists yet. To verify a change:
1. Run `npm run tauri dev`
2. Test the feature manually in the Tauri window
3. Check browser DevTools (right-click → Inspect Element)
4. Check SQLite directly if needed (Tauri doesn't expose a DB viewer yet)

## Common Tasks

**Add a new API endpoint**:
1. Create a function in `src/features/[feature]/[feature].api.ts`
2. Call `api()` from `src/shared/api/api-client.ts`
3. Handle errors with a `.ok` / `.error` pattern
4. Call from Pinia store actions

**Add a new database table**:
1. Add migration in `src/features/local-db/db.client.ts` (in `runMigrations()`)
2. Add repository functions in `src/features/local-db/db.repositories.ts`
3. Call from feature stores/services

**Add a new page/route**:
1. Create `.vue` component in `src/pages/`
2. Add route in `src/app/router.ts`
3. Link from `AppShell.vue` or another page

**Debug WebSocket events**:
- Open browser DevTools console
- WebSocket messages are logged in `src/features/realtime/pinto-ws.client.ts`
- Check `pinto-ws.client.ts` for event handling logic

## Performance Notes

- Large message lists: use virtualization (not yet implemented, but `messages.store.ts` pagination pattern exists)
- SQLite queries: keep them in `db.repositories.ts` for centralization and debugging
- Pinia selectors: use `computed()` to avoid re-rendering on every dependency change
- WebSocket: connection lifecycle is global, initialized once in `App.vue`

## Known Limitations / TODOs

- No offline-first sync conflict resolution yet (simple last-write-wins)
- No encryption at rest
- Message search not implemented
- File/image uploads in progress
