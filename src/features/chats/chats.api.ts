import { ApiClient } from '@/shared/api/api-client'
import type { Chat, ChatListParams, ChatListResponse } from './chats.types'

let client: ApiClient | null = null

export function initChatsApi(getToken: () => string | null, baseUrl: string) {
  client = new ApiClient(baseUrl, getToken)
}

export async function getChats(params: ChatListParams = {}) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  const searchParams = new URLSearchParams()
  if (params.page) searchParams.set('page', String(params.page))
  if (params.limit) searchParams.set('limit', String(params.limit))
  if (params.chat_type) searchParams.set('chat_type', params.chat_type)
  if (params.q) searchParams.set('q', params.q)
  const qs = searchParams.toString()
  return client.get<ChatListResponse>(`/v1/chats${qs ? `?${qs}` : ''}`)
}

export async function getChatDetail(chatId: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.get<Chat>(`/v1/chats/${chatId}`)
}

export async function createChat(payload: { chat_type: string; participant_ids: string[]; name?: string }) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.post<Chat>('/v1/chats', payload)
}

export async function deleteChat(chatId: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.delete<null>(`/v1/chats/${chatId}`)
}

export async function renameChat(chatId: string, name: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.put<Chat>(`/v1/chats/${chatId}/name`, { name })
}

export async function sendTyping(chatId: string) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  return client.post<null>('/v1/chats/typing', { chat_id: chatId })
}

export async function getOnlineStatus(userIds: string[]) {
  if (!client) return { ok: false as const, error: 'API not initialized' }
  const searchParams = new URLSearchParams()
  searchParams.set('user_ids', userIds.join(','))
  return client.get<Record<string, boolean>>(`/v1/chats/online-status?${searchParams.toString()}`)
}
