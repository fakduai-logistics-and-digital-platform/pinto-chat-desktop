import { ApiClient } from '@/shared/api/api-client'
import type { Message, SendMessagePayload } from './messages.types'

let client: ApiClient | null = null

export function initMessagesApi(getToken: () => string | null, baseUrl: string) {
  client = new ApiClient(baseUrl, getToken)
}

export async function sendTextMessage(chatId: string, payload: Omit<SendMessagePayload, 'chat_id'>) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.post<Message>(`/v1/chats/${chatId}/messages`, {
    content: payload.content,
    message_type: payload.message_type ?? 'text',
    reply_to: payload.reply_to,
    client_temp_id: payload.client_temp_id,
  })
}

export async function sendFileMessage(chatId: string, file: File, replyTo?: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  const formData = new FormData()
  formData.append('file', file)
  if (replyTo) formData.append('reply_to', replyTo)
  formData.append('message_type', file.type.startsWith('image/') ? 'image' : 'file')
  const result = await client.post<Message>(`/v1/chats/${chatId}/messages`, formData)
  return result
}

export async function getMessageHistory(chatId: string, params: { page?: number; limit?: number; before?: string } = {}) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  const searchParams = new URLSearchParams()
  if (params.page) searchParams.set('page', String(params.page))
  if (params.limit) searchParams.set('limit', String(params.limit))
  if (params.before) searchParams.set('before', params.before)
  const qs = searchParams.toString()
  return client.get<{ messages: Message[]; total: number }>(`/v1/chats/${chatId}/messages${qs ? `?${qs}` : ''}`)
}

export async function markRead(chatId: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.post<null>(`/v1/chats/${chatId}/messages/read-all`)
}

export async function markMessageRead(messageId: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.post<null>(`/v1/chats/messages/${messageId}/read`)
}
