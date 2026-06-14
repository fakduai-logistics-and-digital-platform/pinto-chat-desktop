import Database from '@tauri-apps/plugin-sql'

let db: Database | null = null

export async function initDatabase(name: string = 'pinto-chat'): Promise<Database> {
  if (db) return db
  db = await Database.load(`sqlite:${name}.sqlite`)
  await runMigrations(db)
  return db
}

export function getDb(): Database {
  if (!db) throw new Error('Database not initialized')
  return db
}

async function runMigrations(database: Database) {
  await database.execute(`
    CREATE TABLE IF NOT EXISTS app_meta (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at TEXT NOT NULL
    )
  `)
  await database.execute(`
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
    )
  `)
  await database.execute(`
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
    )
  `)
  await database.execute(`
    CREATE TABLE IF NOT EXISTS chat_participants (
      chat_id TEXT NOT NULL,
      user_id TEXT NOT NULL,
      role TEXT,
      joined_at TEXT,
      is_admin INTEGER DEFAULT 0,
      raw_json TEXT,
      PRIMARY KEY (chat_id, user_id)
    )
  `)
  await database.execute(`
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
    )
  `)
  await database.execute(`
    CREATE TABLE IF NOT EXISTS message_read_states (
      message_id TEXT NOT NULL,
      user_id TEXT NOT NULL,
      read_at TEXT NOT NULL,
      PRIMARY KEY (message_id, user_id)
    )
  `)
  await database.execute(`
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
    )
  `)
  await database.execute(`
    CREATE TABLE IF NOT EXISTS typing_states (
      chat_id TEXT NOT NULL,
      user_id TEXT NOT NULL,
      is_typing INTEGER NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (chat_id, user_id)
    )
  `)
  await database.execute(`
    CREATE TABLE IF NOT EXISTS bot_generating_states (
      chat_id TEXT NOT NULL,
      bot_id TEXT NOT NULL,
      is_generating INTEGER NOT NULL,
      generating_type TEXT,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (chat_id, bot_id)
    )
  `)
  await database.execute(`CREATE INDEX IF NOT EXISTS idx_messages_chat_sent_at ON messages(chat_id, sent_at DESC)`)
  await database.execute(`CREATE INDEX IF NOT EXISTS idx_messages_chat_created_at ON messages(chat_id, created_at DESC)`)
  await database.execute(`CREATE INDEX IF NOT EXISTS idx_chats_last_message_at ON chats(last_message_at DESC)`)
  await database.execute(`CREATE INDEX IF NOT EXISTS idx_pending_outbox_status ON pending_outbox(status)`)
}
