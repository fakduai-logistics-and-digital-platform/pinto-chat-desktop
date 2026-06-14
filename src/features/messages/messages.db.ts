import { getDb } from '@/features/local-db/db.client'
import type { Message } from '@/features/messages/messages.types'

export const messagesRepo = {
  async upsert(msg: Message) {
    const db = getDb()
    await db.execute(
      `INSERT OR REPLACE INTO messages
        (message_id, chat_id, sender_id, content, message_type, reply_to, media_url, local_media_path, file_name, file_size, mime_type, status, is_read, is_deleted, client_temp_id, raw_json, sent_at, received_at, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        msg.message_id, msg.chat_id, msg.sender_id, msg.content, msg.message_type,
        msg.reply_to, msg.media_url, msg.local_media_path, msg.file_name, msg.file_size,
        msg.mime_type, msg.status, msg.is_read ? 1 : 0, msg.is_deleted ? 1 : 0,
        msg.client_temp_id, msg.raw_json, msg.sent_at, msg.received_at, msg.created_at, msg.updated_at,
      ]
    )
  },

  async getByChatId(chatId: string, limit = 50, before?: string): Promise<Message[]> {
    const db = getDb()
    if (before) {
      return db.select<Message[]>(
        `SELECT * FROM messages WHERE chat_id = ? AND created_at < ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT ?`,
        [chatId, before, limit]
      )
    }
    return db.select<Message[]>(
      `SELECT * FROM messages WHERE chat_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT ?`,
      [chatId, limit]
    )
  },

  async updateStatus(messageId: string, status: string) {
    const db = getDb()
    await db.execute(`UPDATE messages SET status = ?, updated_at = ? WHERE message_id = ?`, [status, new Date().toISOString(), messageId])
  },

  async updateStatusByTempId(tempId: string, realId: string, status: string) {
    const db = getDb()
    await db.execute(
      `UPDATE messages SET message_id = ?, status = ?, updated_at = ? WHERE client_temp_id = ?`,
      [realId, status, new Date().toISOString(), tempId]
    )
  },

  async markRead(messageId: string) {
    const db = getDb()
    await db.execute(`UPDATE messages SET is_read = 1, updated_at = ? WHERE message_id = ?`, [new Date().toISOString(), messageId])
  },

  async markChatRead(chatId: string) {
    const db = getDb()
    await db.execute(`UPDATE messages SET is_read = 1, updated_at = ? WHERE chat_id = ? AND is_read = 0`, [new Date().toISOString(), chatId])
  },

  async markDeleted(messageId: string) {
    const db = getDb()
    await db.execute(`UPDATE messages SET is_deleted = 1, updated_at = ? WHERE message_id = ?`, [new Date().toISOString(), messageId])
  },

  async count(chatId: string): Promise<number> {
    const db = getDb()
    const result = await db.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM messages WHERE chat_id = ? AND is_deleted = 0`, [chatId])
    return result[0]?.count ?? 0
  },
}
