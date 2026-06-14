# Pinto Chat Desktop — AI Agent Implementation Plan

> Stack ที่เลือก: **Tauri v2 + Vue 3 + TypeScript + Pinia + SQLite + WebSocket**  
> เป้าหมาย: ทำแอป Desktop Chat สำหรับ Pinto ที่ใช้งานได้บน **Windows / macOS / Linux** หน้าตาคล้าย LINE Desktop + Telegram Desktop แต่เบากว่า ไม่ค้าง และพร้อมต่อยอดเป็นแอปจริง

---

## 0. API ที่ใช้

```txt
API Base URL: https://api-dev.pinto-app.com
Swagger YAML: https://api-dev.pinto-app.com/swagger.yaml
Auth Type: JWT Bearer Token
Header: Authorization: Bearer <token>
Main Realtime Endpoint: /v1/ws/chat
```

### หมายเหตุสำคัญจาก API

ระบบ Chat ของ Pinto เป็นแนวทาง **local-first realtime chat**:

```txt
- Server ส่งข้อความใหม่ผ่าน WebSocket
- Client ต้องเก็บข้อความไว้ใน local database เอง
- ข้อความใหม่ไม่ได้พึ่ง MongoDB เป็น source หลัก
- Offline users จะได้รับ queued messages ตอน reconnect
- Message history endpoint ใช้เพื่อ migration / โหลดข้อมูลเก่าเท่านั้น
```

ดังนั้นแอป Desktop ต้องมี **SQLite** และห้ามทำเป็นแค่เว็บแชทธรรมดาที่โหลดข้อความจาก API ทุกครั้ง

---

## 1. เป้าหมายของโปรแกรม

ทำแอป Pinto Chat Desktop แบบ production-ready ที่รองรับ:

```txt
- Login / session management
- Chat list
- Direct chat
- Group chat
- Realtime messages
- Local message storage
- Offline outbox
- Retry failed messages
- File / image sending
- Reply message
- Typing indicator
- Online status
- Read receipt
- Native notification
- Tray icon
- Auto update
- Light / dark mode
- Windows / macOS / Linux build
```

---

## 2. หน้าตาแอป

หน้าหลักแบ่งเป็น 3 คอลัมน์:

```txt
┌──────────────────────┬──────────────────────────────────┬──────────────────────┐
│ Sidebar / Chat List  │ Chat Room                         │ Info Panel           │
│                      │                                  │                      │
│ Search               │ Chat Header                       │ Profile / Group Info │
│ Direct / Group Tabs  │ Message List                      │ Members              │
│ Chat Items           │ Typing Indicator                  │ Shared Files         │
│ Settings             │ Composer                          │ Actions              │
└──────────────────────┴──────────────────────────────────┴──────────────────────┘
```

### Mood & Style

```txt
- Clean
- Modern
- Professional
- ไม่รก
- ไม่ดู AI slop
- ใช้งานทุกวันได้จริง
- ไม่เหมือน admin dashboard จ๋าเกินไป
```

### Visual Direction

```txt
Background: off-white / dark slate
Primary: Pinto orange หรือ warm amber
Accent: soft blue สำหรับ online/status
Border: subtle gray
Radius: 12px - 18px
Shadow: very subtle
Font: system font
```

---

## 3. Responsive Layout

```txt
Desktop >= 1200px:
- แสดง 3 คอลัมน์: chat list + room + info panel

900px - 1199px:
- แสดง chat list + room
- info panel เป็น drawer

< 900px:
- แสดงทีละหน้า
- เลือก chat แล้วซ่อน sidebar
- มีปุ่ม back กลับ chat list
```

---

## 4. Tech Stack

```txt
Desktop Shell: Tauri v2
Frontend: Vue 3 + TypeScript + Vite
State: Pinia
Local Database: SQLite
Realtime: WebSocket
API Client: fetch หรือ axios
Virtual List: vue-virtual-scroller หรือ custom virtual list
Native Notification: Tauri notification plugin
Auto Update: Tauri updater plugin
Styling: Tailwind CSS หรือ UnoCSS
```

---

## 5. โครงสร้างโปรเจกต์

```txt
pinto-chat-desktop/
  package.json
  vite.config.ts
  tsconfig.json
  index.html

  src/
    main.ts
    App.vue

    app/
      router.ts
      plugins.ts
      constants.ts
      env.ts

    layouts/
      AuthLayout.vue
      AppShell.vue

    pages/
      LoginPage.vue
      ChatPage.vue
      SettingsPage.vue

    features/
      auth/
        auth.api.ts
        auth.store.ts
        auth.types.ts
        token-storage.ts

      users/
        users.api.ts
        users.types.ts

      chats/
        chats.api.ts
        chats.store.ts
        chats.types.ts
        components/
          ChatList.vue
          ChatListItem.vue
          NewChatDialog.vue
          ChatSearchInput.vue

      messages/
        messages.api.ts
        messages.db.ts
        messages.store.ts
        messages.types.ts
        message-normalizer.ts
        outbox.service.ts
        components/
          MessageVirtualList.vue
          MessageBubble.vue
          MessageComposer.vue
          AttachmentPreview.vue
          ReplyPreview.vue
          DateDivider.vue
          TypingIndicator.vue
          BotGeneratingIndicator.vue
          ScrollToBottomButton.vue

      realtime/
        pinto-ws.client.ts
        ws-events.ts
        ws-reconnect.ts
        ws-dispatcher.ts

      local-db/
        db.client.ts
        db.migrations.ts
        db.schema.ts
        db.repositories.ts

      notifications/
        notification.service.ts
        notification.store.ts

      settings/
        settings.store.ts
        settings.types.ts

    shared/
      api/
        api-client.ts
        api-error.ts
        api-response.ts
      components/
        AppButton.vue
        AppAvatar.vue
        AppIconButton.vue
        AppInput.vue
        AppDialog.vue
        AppEmptyState.vue
        AppSkeleton.vue
      utils/
        date.ts
        file.ts
        text.ts
        debounce.ts
        throttle.ts
        uuid.ts

  src-tauri/
    tauri.conf.json
    capabilities/
    src/
      main.rs
      lib.rs
```

---

## 6. API Endpoint Map

### 6.1 Authentication

| Method | Endpoint | ใช้ทำอะไร | Auth |
|---|---|---|---|
| POST | `/api/auth/login` | Login ด้วย email หรือ username + password | No |
| POST | `/api/auth/create-account` | สมัครบัญชี | No |
| POST | `/api/auth/refresh` | Refresh token | No / refresh token |
| POST | `/api/auth/logout` | Logout และ invalidate token | Bearer |
| POST | `/v1/auth/qr/confirm` | มือถือ confirm QR login session | Bearer |
| GET | `/v1/users/profile` | ดึง profile ของ user ปัจจุบัน | Bearer |

### 6.2 Chat

| Method | Endpoint | ใช้ทำอะไร | Auth |
|---|---|---|---|
| GET | `/v1/chats` | ดึง chat list | Bearer |
| POST | `/v1/chats` | สร้าง direct/group chat | Bearer |
| GET | `/v1/chats/{chat_id}` | ดึงรายละเอียด chat | Bearer |
| DELETE | `/v1/chats/{chat_id}` | ลบ chat ฝั่ง user ปัจจุบันแบบ soft delete | Bearer |
| PUT | `/v1/chats/{chat_id}/name` | เปลี่ยนชื่อ group chat | Bearer |
| PUT | `/v1/chats/{chat_id}/avatar` | เปลี่ยน avatar group chat | Bearer |

### 6.3 Messages

| Method | Endpoint | ใช้ทำอะไร | Auth |
|---|---|---|---|
| POST | `/v1/chats/{chat_id}/messages` | ส่งข้อความ text/image/file | Bearer |
| GET | `/v1/chats/{chat_id}/messages` | โหลด message history เก่า / migration เท่านั้น | Bearer |
| POST | `/v1/chats/{chat_id}/messages/read-all` | mark ข้อความทั้งหมดใน chat ว่าอ่านแล้ว | Bearer |
| POST | `/v1/chats/messages/{message_id}/read` | mark message เดี่ยวว่าอ่านแล้ว | Bearer |

### 6.4 Realtime / WebSocket

| Method | Endpoint | ใช้ทำอะไร | Auth |
|---|---|---|---|
| GET | `/v1/ws/chat` | realtime chat websocket | Bearer |
| POST | `/v1/chats/typing` | ส่ง typing indicator | Bearer |
| GET | `/v1/chats/online-status` | ดึง online status ของ users | Bearer |

### 6.5 Group Participants

| Method | Endpoint | ใช้ทำอะไร | Auth |
|---|---|---|---|
| POST | `/v1/chats/{chat_id}/participants` | เพิ่มสมาชิก group chat | Bearer |
| DELETE | `/v1/chats/{chat_id}/participants` | ลบสมาชิก group chat | Bearer |

---

## 7. API Contract ที่ Agent ต้องยึด

### 7.1 Standard Response

ทุก API ควร normalize เป็นรูปแบบนี้:

```ts
export type StandardResponse<T> = {
  ok: boolean;
  data: T | null;
  error: string | null;
};
```

### 7.2 API Client Rules

```txt
- ทุก request ที่ต้อง auth ต้องใส่ Authorization: Bearer <token>
- ถ้าเจอ 401 ให้ clear session แล้ว redirect ไป Login
- ถ้าเจอ 429 ให้ retry แบบ delay
- ถ้า network error ให้ return typed error
- ห้าม throw raw unknown error ไปที่ component
- component ต้องได้ error ที่พร้อมแสดงบน UI
```

### 7.3 API Client Example

```ts
export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: string; status?: number };

export class ApiClient {
  constructor(
    private readonly baseUrl: string,
    private readonly getToken: () => string | null,
  ) {}

  async request<T>(path: string, options: RequestInit = {}): Promise<ApiResult<T>> {
    const token = this.getToken();
    const headers = new Headers(options.headers);

    if (token) headers.set('Authorization', `Bearer ${token}`);
    if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
      headers.set('Content-Type', 'application/json');
    }

    try {
      const response = await fetch(`${this.baseUrl}${path}`, {
        ...options,
        headers,
      });

      const payload = await response.json().catch(() => null);

      if (!response.ok) {
        return {
          ok: false,
          status: response.status,
          error: payload?.error ?? `HTTP ${response.status}`,
        };
      }

      if (payload?.ok === false) {
        return {
          ok: false,
          status: response.status,
          error: payload.error ?? 'Unknown API error',
        };
      }

      return {
        ok: true,
        data: payload?.data ?? payload,
      };
    } catch (error) {
      return {
        ok: false,
        error: error instanceof Error ? error.message : 'Network error',
      };
    }
  }
}
```

---

## 8. Auth Feature

### 8.1 หน้าที่

```txt
- Login
- Register placeholder
- Token storage
- Refresh token
- Profile loading
- Logout
- Session guard
```

### 8.2 Login Page

UI ต้องมี:

```txt
- Pinto logo
- App name: Pinto Chat Desktop
- Email / username input
- Password input
- Login button
- Dev token login section
- QR login placeholder
- API environment selector
- Error alert
```

### 8.3 Auth Flow

```txt
1. เปิดแอป
2. โหลด token จาก secure storage/local storage
3. ถ้ามี token → GET /v1/users/profile
4. ถ้าสำเร็จ → เข้า ChatPage
5. ถ้า fail → clear token → LoginPage
6. Login สำเร็จ → save token + refresh token
7. Connect WebSocket หลัง login สำเร็จ
```

### 8.4 Auth Store State

```ts
export type AuthState = {
  token: string | null;
  refreshToken: string | null;
  currentUser: UserProfile | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
};
```

---

## 9. Chat List Feature

### 9.1 UI ที่ต้องมี

```txt
- Search input
- Filter tabs: All / Direct / Groups / Unread
- Pinned section
- Recent section
- Chat item
- Unread badge
- Online dot
- Last message preview
- Last message time
- Mute icon
- Draft indicator
- Connection status
```

### 9.2 Chat Item ต้องมี

```txt
- Avatar
- Chat name
- Username หรือ group subtitle
- Last message preview
- Time
- Unread count
- Online status
- Typing indicator
- Bot generating indicator
```

### 9.3 Chat List API Query

```txt
GET /v1/chats?page=1&limit=20&chat_type=direct&q=keyword
```

Query params:

```txt
page: number
limit: number, max 100
chat_type: direct | group
q: fuzzy search query
```

### 9.4 Behavior

```txt
- Search debounce 300ms
- Scroll ถึงล่างสุดแล้ว load next page
- ได้ new_message จาก WebSocket แล้วดัน chat นั้นขึ้นบนสุด
- ถ้า chat ถูกลบ ให้ซ่อนจาก list
- ถ้า typing event ให้แสดง "กำลังพิมพ์..."
- ถ้า bot_generating ให้แสดง "AI กำลังตอบ..."
```

---

## 10. Chat Room Feature

### 10.1 UI Components

```txt
- ChatHeader
- MessageVirtualList
- DateDivider
- MessageBubble
- TypingIndicator
- BotGeneratingIndicator
- ReplyPreview
- MessageComposer
- AttachmentPreview
- ScrollToBottomButton
- UnreadDivider
```

### 10.2 Chat Header

ต้องมี:

```txt
- Avatar
- Chat name
- Online status หรือ member count
- Search in chat button
- Call placeholder button
- More menu
- Info panel toggle
```

### 10.3 Message List

ต้องมี:

```txt
- Virtual scrolling
- Load older messages from SQLite
- Auto scroll เมื่อ user อยู่ใกล้ bottom
- ไม่ auto scroll ถ้า user กำลังอ่านข้อความเก่า
- Date divider: วันนี้ / เมื่อวาน / 13 มิ.ย. 2026
- Unread divider
- Skeleton loading
- Empty state
```

### 10.4 Message Bubble Types

รองรับ:

```txt
- text
- image
- file
- reply message
- sending state
- sent state
- failed state
- read state
- timestamp
- sender avatar ใน group
- sender name ใน group
```

---

## 11. Message Composer

### 11.1 UI

```txt
- Textarea auto-resize
- Send button
- Attachment button
- Emoji button placeholder
- Reply preview
- File preview
- Image preview
- Typing status
```

### 11.2 Behavior

```txt
Enter = send
Shift + Enter = new line
Paste image = attach image
Drag & drop file = attach file
Typing debounce = 800ms
Stop typing after idle 1500ms
Disable send ถ้าไม่มี content และไม่มี file
Limit content = 1000 characters
```

---

## 12. SQLite Schema

Database file:

```txt
pinto-chat.sqlite
```

### 12.1 Tables

```sql
CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_session (
  id TEXT PRIMARY KEY,
  access_token TEXT NOT NULL,
  refresh_token TEXT,
  user_id TEXT,
  expires_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  user_id TEXT PRIMARY KEY,
  username TEXT,
  name TEXT,
  profile_image TEXT,
  bio TEXT,
  is_online INTEGER DEFAULT 0,
  last_seen_at TEXT,
  raw_json TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chats (
  chat_id TEXT PRIMARY KEY,
  chat_type TEXT NOT NULL,
  name TEXT,
  avatar_url TEXT,
  last_message_id TEXT,
  last_message_preview TEXT,
  last_message_at TEXT,
  unread_count INTEGER DEFAULT 0,
  is_pinned INTEGER DEFAULT 0,
  is_muted INTEGER DEFAULT 0,
  is_archived INTEGER DEFAULT 0,
  is_deleted INTEGER DEFAULT 0,
  raw_json TEXT,
  created_at TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_participants (
  chat_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  role TEXT,
  joined_at TEXT,
  is_admin INTEGER DEFAULT 0,
  raw_json TEXT,
  PRIMARY KEY (chat_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages (
  message_id TEXT PRIMARY KEY,
  chat_id TEXT NOT NULL,
  sender_id TEXT,
  content TEXT,
  message_type TEXT NOT NULL,
  reply_to TEXT,
  media_url TEXT,
  local_media_path TEXT,
  file_name TEXT,
  file_size INTEGER,
  mime_type TEXT,
  status TEXT NOT NULL,
  is_read INTEGER DEFAULT 0,
  is_deleted INTEGER DEFAULT 0,
  client_temp_id TEXT,
  raw_json TEXT,
  sent_at TEXT,
  received_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS message_read_states (
  message_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  read_at TEXT NOT NULL,
  PRIMARY KEY (message_id, user_id)
);

CREATE TABLE IF NOT EXISTS pending_outbox (
  id TEXT PRIMARY KEY,
  chat_id TEXT NOT NULL,
  client_temp_id TEXT NOT NULL,
  content TEXT,
  message_type TEXT NOT NULL,
  reply_to TEXT,
  local_file_path TEXT,
  status TEXT NOT NULL,
  retry_count INTEGER DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS typing_states (
  chat_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  is_typing INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (chat_id, user_id)
);

CREATE TABLE IF NOT EXISTS bot_generating_states (
  chat_id TEXT NOT NULL,
  bot_id TEXT NOT NULL,
  is_generating INTEGER NOT NULL,
  generating_type TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (chat_id, bot_id)
);
```

### 12.2 Indexes

```sql
CREATE INDEX IF NOT EXISTS idx_messages_chat_sent_at
ON messages(chat_id, sent_at DESC);

CREATE INDEX IF NOT EXISTS idx_messages_chat_created_at
ON messages(chat_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_chats_last_message_at
ON chats(last_message_at DESC);

CREATE INDEX IF NOT EXISTS idx_pending_outbox_status
ON pending_outbox(status);
```

---

## 13. WebSocket

### 13.1 Endpoint

```txt
GET /v1/ws/chat
```

### 13.2 Event Format

```json
{
  "type": "new_message|typing|online_status|read_receipt",
  "data": {},
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### 13.3 Connection Lifecycle

```txt
App boot
→ Load token
→ Connect WebSocket
→ On open: set connected
→ On message: dispatch by type
→ On close: reconnect
→ On auth error: logout
```

### 13.4 Reconnect Strategy

```txt
Attempt 1: 1s
Attempt 2: 2s
Attempt 3: 5s
Attempt 4: 10s
Attempt 5+: 30s
```

### 13.5 Event Handling

```txt
new_message:
- normalize payload
- save message to SQLite
- update chat last_message
- update unread count ถ้าไม่ได้เปิด chat นั้น
- notify ถ้าแอปไม่ focus

typing:
- update typing_states
- auto clear after 3s

online_status:
- update users.is_online
- update last_seen_at

read_receipt:
- update message_read_states
- update message is_read

bot_generating:
- update bot_generating_states
- show indicator
```

---

## 14. Pinia State Rules

### Pinia ใช้เก็บ

```txt
- selectedChatId
- current user
- visible chat list
- connection status
- composer draft
- typing state
- UI settings
```

### SQLite ใช้เก็บ

```txt
- messages ทั้งหมด
- chats cache
- participants
- read receipts
- pending outbox
```

### ห้ามทำเด็ดขาด

```txt
- ห้ามเก็บ messages ทุกห้องไว้ใน Pinia
- ห้าม render messages ทั้งหมดใน DOM
- ห้าม load 10,000 messages ทีเดียว
- ห้ามให้ component ยิง API เองมั่ว ๆ โดยไม่ผ่าน service/store
```

---

## 15. Offline-first / Outbox

### 15.1 ตอน offline

```txt
- ส่งข้อความแล้ว insert pending_outbox
- insert message local ทันที
- message status = queued
- UI แสดง clock icon
- reconnect แล้วค่อยส่งใหม่
```

### 15.2 ตอนส่งสำเร็จ

```txt
- API response ได้ message_id
- replace temp message
- status = sent
- remove pending_outbox
```

### 15.3 ตอนส่งไม่สำเร็จ

```txt
- status = failed
- แสดงปุ่ม retry
- retry_count + 1
- เก็บ last_error
```

---

## 16. Native Notifications

### แจ้งเตือนเมื่อ

```txt
- ได้ new_message
- chat นั้นไม่ใช่ selectedChatId
- window ไม่ focused หรือ minimized
- chat ไม่ muted
```

### ไม่แจ้งเตือนเมื่อ

```txt
- ผู้ใช้อยู่ในห้องแชทนั้น
- ข้อความมาจากตัวเอง
- chat muted
- app focused และอยู่ที่ bottom
```

### Notification Content

```txt
Title: chat name
Body: sender name + message preview
Click: focus app + open chat
```

---

## 17. Settings Page

ต้องมีหมวดนี้:

```txt
Account
Appearance
Notifications
Storage
Network
Updates
About
Developer
```

### Account

```txt
- Profile preview
- Logout
```

### Appearance

```txt
- Light / Dark / System
- Compact mode
- Font size
```

### Notifications

```txt
- Enable notifications
- Sound placeholder
- Show message preview
```

### Storage

```txt
- SQLite path
- Cache size
- Clear media cache
- Export debug logs
```

### Network

```txt
- API base URL
- WebSocket status
- Reconnect button
```

### Updates

```txt
- Current version
- Check for updates
- Release notes
```

### Developer

```txt
- Token viewer masked
- Reset local DB
- Import old messages
- Open logs folder
```

---

## 18. Milestones

### Milestone 1: Project Bootstrap

```txt
- สร้าง Tauri v2 + Vue 3 + TypeScript project
- ติดตั้ง Pinia
- ติดตั้ง Vue Router
- ติดตั้ง Tailwind หรือ UnoCSS
- ติดตั้ง Tauri SQL plugin
- ติดตั้ง Tauri Notification plugin
- เตรียม env config
```

Acceptance:

```txt
- npm run tauri dev รันได้
- มีหน้า Login เปล่า
- มีหน้า Chat เปล่า
```

### Milestone 2: API Client + Auth

```txt
- สร้าง api-client.ts
- รองรับ baseURL
- รองรับ bearer token
- รองรับ StandardResponse
- ทำ token storage
- ทำ LoginPage
- เรียก GET /v1/users/profile
- ทำ route guard
```

Acceptance:

```txt
- login ได้
- token ผิดขึ้น error
- reload app แล้วยัง login อยู่
- logout แล้วกลับ LoginPage
```

### Milestone 3: SQLite Foundation

```txt
- setup SQL plugin
- สร้าง migrations
- สร้าง tables ทั้งหมด
- ทำ db repositories
- ทำ seed mock data สำหรับ dev
```

Acceptance:

```txt
- เปิดแอปแล้ว DB ถูกสร้าง
- insert chat ได้
- insert message ได้
- query messages by chat_id ได้
```

### Milestone 4: Chat List

```txt
- ทำ GET /v1/chats
- cache chats ลง SQLite
- render sidebar
- search
- filter direct/group
- pagination
```

Acceptance:

```txt
- แสดง chat list ได้
- ค้นหาได้
- เลือก chat ได้
- chat ที่เลือก active ชัดเจน
```

### Milestone 5: Message Room

```txt
- load messages from SQLite
- ทำ MessageVirtualList
- ทำ MessageBubble
- ทำ DateDivider
- ทำ composer
```

Acceptance:

```txt
- เปิด chat แล้วแสดงข้อความ local ได้
- ข้อความเยอะไม่ค้าง
- scroll ย้อนหลังได้
```

### Milestone 6: Send Message

```txt
- POST /v1/chats/{chat_id}/messages
- optimistic insert
- pending_outbox
- retry failed message
- file/image multipart upload
```

Acceptance:

```txt
- ส่ง text ได้
- ส่งรูปได้
- ส่งไฟล์ได้
- ส่งไม่ผ่านแล้ว retry ได้
- ข้อความไม่หายแม้ปิดเปิดแอป
```

### Milestone 7: WebSocket Realtime

```txt
- connect /v1/ws/chat
- reconnect
- dispatch events
- handle new_message
- handle typing
- handle online_status
- handle read_receipt
- update SQLite
- update UI
```

Acceptance:

```txt
- ข้อความใหม่ขึ้นทันที
- typing indicator แสดง
- online status แสดง
- หลุดเน็ตแล้ว reconnect ได้
```

### Milestone 8: Native Desktop Polish

```txt
- native notification
- tray icon
- minimize to tray
- window focus handling
- keyboard shortcuts
- auto update check
```

Shortcuts:

```txt
Cmd/Ctrl + K = command palette
Cmd/Ctrl + F = search in chat
Cmd/Ctrl + N = new chat
Esc = close dialog
Enter = send
Shift + Enter = new line
```

### Milestone 9: Group Features

```txt
- group info panel
- rename group
- update avatar
- add participants
- remove participants
- delete chat
```

Acceptance:

```txt
- แก้ชื่อ group ได้
- เปลี่ยน avatar group ได้
- เพิ่ม/ลบสมาชิกได้
- delete chat แล้วหายจาก list
```

### Milestone 10: Release Build

```txt
- build Windows
- build macOS
- build Linux
- setup updater
- setup app icon
- setup installer metadata
- write README
```

---

## 19. Definition of Done

โปรเจกต์ถือว่าเสร็จรอบแรกเมื่อทำได้ครบนี้:

```txt
- เปิดแอป desktop ได้
- login ได้
- ดึง profile ได้
- เห็น chat list
- เปิด chat ได้
- โหลดข้อความจาก SQLite ได้
- ส่ง text message ได้
- ส่ง image/file ได้
- รับ WebSocket new_message ได้
- typing indicator ทำงาน
- online status ทำงาน
- read receipt ทำงาน
- offline แล้วข้อความไม่หาย
- reconnect แล้ว sync ต่อได้
- native notification ทำงาน
- app ไม่ค้างเมื่อ message เยอะ
- build Windows/macOS/Linux ได้
```

---

## 20. Prompt สำหรับส่งให้ AI Coding Agent

```txt
Build a production-ready desktop chat application for Pinto using Tauri v2, Vue 3, TypeScript, Pinia, SQLite, and WebSocket.

The app must support Windows, macOS, and Linux.

Use this API:
- API Base URL: https://api-dev.pinto-app.com
- Swagger YAML: https://api-dev.pinto-app.com/swagger.yaml

The API uses JWT bearer authentication. Use POST /api/auth/login for email or username login, POST /api/auth/refresh for token refresh, POST /api/auth/logout for logout, and GET /v1/users/profile to validate the current user.

For chat, use GET /v1/chats for chat list, POST /v1/chats for creating chats, GET /v1/chats/{chat_id} for chat details, DELETE /v1/chats/{chat_id} for deleting chats, PUT /v1/chats/{chat_id}/name for renaming group chats, PUT /v1/chats/{chat_id}/avatar for group avatar upload, POST /v1/chats/{chat_id}/messages for sending messages, POST /v1/chats/typing for typing indicator, GET /v1/chats/online-status for online status, and GET /v1/ws/chat for realtime WebSocket.

Important architecture requirement: messages must be stored locally in SQLite. The message history endpoint is only for migration of old messages. The UI must load messages from SQLite. All WebSocket new_message events must be saved to SQLite before updating UI state.

Create a desktop UI similar to Telegram Desktop and LINE Desktop:
- left sidebar with chat list and search
- center chat room with virtualized messages
- right info panel for profile or group info
- clean professional design
- light/dark mode
- native notifications
- offline outbox
- automatic reconnect
- retry failed messages
- file/image sending
- reply support
- typing indicator
- online status
- read receipt
- bot generating indicator

Do not store all messages in Pinia. Pinia should only hold selected chat, UI state, visible chat list, connection status, current user, and composer draft. SQLite is the source of truth for messages.

Implement the project in milestones:
1. Bootstrap Tauri v2 + Vue 3 + TypeScript
2. API client and auth
3. SQLite migrations and repositories
4. Chat list
5. Message room with virtual scrolling
6. Send message with optimistic UI and outbox
7. WebSocket realtime
8. Native notifications and tray
9. Group chat management
10. Release build and updater

Create clean, modular files under src/features. Use strict TypeScript. Add error handling. Add loading states. Add empty states. Add retry states. Make the app usable with real API data.
```

---

## 21. สรุปสั้นสุด

แอปนี้ต้องถูกออกแบบเป็น:

```txt
Local-first realtime desktop chat client
```

ไม่ใช่:

```txt
Web chat ที่ครอบด้วย Tauri เฉย ๆ
```

จุดที่ห้ามพลาด:

```txt
SQLite + WebSocket + Virtual List + Outbox Retry
```

ถ้า 4 อย่างนี้แข็ง แอปจะลื่นและดูเป็นแอปจริงทันที
