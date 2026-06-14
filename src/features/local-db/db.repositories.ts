import { getDb } from './db.client'
import type { Chat } from '@/features/chats/chats.types'

export const chatsRepo = {
  async upsert(chat: Chat) {
    const db = getDb()
    await db.execute(
      `INSERT OR REPLACE INTO chats
        (chat_id, chat_type, name, avatar_url, last_message_id, last_message_preview, last_message_at, unread_count, is_pinned, is_muted, is_archived, is_deleted, raw_json, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        chat.chat_id, chat.chat_type, chat.name, chat.avatar_url,
        chat.last_message_id, chat.last_message_preview, chat.last_message_at,
        chat.unread_count, chat.is_pinned ? 1 : 0, chat.is_muted ? 1 : 0,
        chat.is_archived ? 1 : 0, chat.is_deleted ? 1 : 0,
        JSON.stringify(chat), chat.created_at, chat.updated_at,
      ]
    )
  },

  async upsertMany(chats: Chat[]) {
    for (const chat of chats) {
      await this.upsert(chat)
    }
  },

  async getAll(): Promise<Chat[]> {
    const db = getDb()
    return db.select<Chat[]>(
      `SELECT *, is_pinned as is_pinned, is_muted as is_muted, is_archived as is_archived, is_deleted as is_deleted FROM chats WHERE is_deleted = 0 ORDER BY is_pinned DESC, last_message_at DESC`
    )
  },

  async getById(chatId: string): Promise<Chat | null> {
    const db = getDb()
    const results = await db.select<Chat[]>(`SELECT * FROM chats WHERE chat_id = ?`, [chatId])
    return results[0] ?? null
  },

  async updateLastMessage(chatId: string, messageId: string, preview: string, at: string) {
    const db = getDb()
    await db.execute(
      `UPDATE chats SET last_message_id = ?, last_message_preview = ?, last_message_at = ?, updated_at = ? WHERE chat_id = ?`,
      [messageId, preview, at, new Date().toISOString(), chatId]
    )
  },

  async incrementUnread(chatId: string) {
    const db = getDb()
    await db.execute(`UPDATE chats SET unread_count = unread_count + 1, updated_at = ? WHERE chat_id = ?`, [new Date().toISOString(), chatId])
  },

  async clearUnread(chatId: string) {
    const db = getDb()
    await db.execute(`UPDATE chats SET unread_count = 0, updated_at = ? WHERE chat_id = ?`, [new Date().toISOString(), chatId])
  },

  async markDeleted(chatId: string) {
    const db = getDb()
    await db.execute(`UPDATE chats SET is_deleted = 1, updated_at = ? WHERE chat_id = ?`, [new Date().toISOString(), chatId])
  },
}
