import { getDb } from '@/features/local-db/db.client'
import type { PendingOutbox } from '@/features/messages/messages.types'

export const outboxRepo = {
  async insert(item: PendingOutbox) {
    const db = getDb()
    await db.execute(
      `INSERT OR REPLACE INTO pending_outbox
        (id, chat_id, client_temp_id, content, message_type, reply_to, local_file_path, status, retry_count, last_error, created_at, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        item.id, item.chat_id, item.client_temp_id, item.content, item.message_type,
        item.reply_to, item.local_file_path, item.status, item.retry_count, item.last_error,
        item.created_at, item.updated_at,
      ]
    )
  },

  async getPending(): Promise<PendingOutbox[]> {
    const db = getDb()
    return db.select<PendingOutbox[]>(`SELECT * FROM pending_outbox WHERE status IN ('queued', 'failed') ORDER BY created_at ASC`)
  },

  async updateStatus(id: string, status: string, error?: string) {
    const db = getDb()
    await db.execute(
      `UPDATE pending_outbox SET status = ?, last_error = ?, updated_at = ? WHERE id = ?`,
      [status, error ?? null, new Date().toISOString(), id]
    )
  },

  async incrementRetry(id: string, error?: string) {
    const db = getDb()
    await db.execute(
      `UPDATE pending_outbox SET retry_count = retry_count + 1, last_error = ?, status = 'failed', updated_at = ? WHERE id = ?`,
      [error ?? null, new Date().toISOString(), id]
    )
  },

  async remove(id: string) {
    const db = getDb()
    await db.execute(`DELETE FROM pending_outbox WHERE id = ?`, [id])
  },

  async count(): Promise<number> {
    const db = getDb()
    const result = await db.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM pending_outbox WHERE status IN ('queued', 'failed')`)
    return result[0]?.count ?? 0
  },
}
