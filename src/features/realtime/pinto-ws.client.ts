import { ref } from 'vue'
import { env } from '@/app/env'
import { WS_RECONNECT_DELAYS } from '@/app/constants'
import { getStoredToken } from '@/features/auth/token-storage'
import { useMessagesStore } from '@/features/messages/messages.store'
import { useChatsStore } from '@/features/chats/chats.store'
import { normalizeMessage } from '@/features/messages/message-normalizer'
import { messagesRepo } from '@/features/messages/messages.db'

export type WsConnectionState = 'disconnected' | 'connecting' | 'connected'

const connectionState = ref<WsConnectionState>('disconnected')
let ws: WebSocket | null = null
let reconnectAttempt = 0
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let intentionalClose = false

export function useWsClient() {
  function connect() {
    const token = getStoredToken()
    if (!token) return

    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return
    }

    connectionState.value = 'connecting'
    intentionalClose = false

    const wsUrl = `${env.apiBaseUrl.replace(/^http/, 'ws')}/v1/ws/chat`
    ws = new WebSocket(`${wsUrl}?token=${encodeURIComponent(token)}`)

    ws.onopen = () => {
      connectionState.value = 'connected'
      reconnectAttempt = 0
    }

    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data)
        handleEvent(payload)
      } catch {
        // ignore malformed messages
      }
    }

    ws.onclose = () => {
      connectionState.value = 'disconnected'
      if (!intentionalClose) {
        scheduleReconnect()
      }
    }

    ws.onerror = () => {
      ws?.close()
    }
  }

  function disconnect() {
    intentionalClose = true
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    ws?.close()
    ws = null
    connectionState.value = 'disconnected'
  }

  function scheduleReconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    const delay = WS_RECONNECT_DELAYS[Math.min(reconnectAttempt, WS_RECONNECT_DELAYS.length - 1)]
    reconnectTimer = setTimeout(() => {
      reconnectAttempt++
      connect()
    }, delay)
  }

  function handleEvent(payload: { type: string; data: Record<string, unknown> }) {
    const messagesStore = useMessagesStore()
    const chatsStore = useChatsStore()

    switch (payload.type) {
      case 'new_message': {
        const raw = payload.data as Record<string, unknown>
        const msg = normalizeMessage(raw)
        messagesStore.addMessageFromWs(msg)
        messagesRepo.upsert(msg)

        chatsStore.moveChatToTop(msg.chat_id)
        chatsStore.updateChatFromWs({
          chat_id: msg.chat_id,
          last_message_id: msg.message_id,
          last_message_preview: (msg.content ?? '').slice(0, 100),
          last_message_at: msg.created_at,
        })

        if (chatsStore.selectedChatId !== msg.chat_id) {
          chatsStore.updateUnread(msg.chat_id, (chatsStore.chats.get(msg.chat_id)?.unread_count ?? 0) + 1)
        }
        break
      }
      case 'typing': {
        const data = payload.data as { chat_id: string; user_id: string; is_typing: boolean }
        chatsStore.updateChatFromWs({ chat_id: data.chat_id } as any)
        break
      }
      case 'online_status': {
        break
      }
      case 'read_receipt': {
        const data = payload.data as { message_id: string; user_id: string }
        if (data.message_id) {
          messagesRepo.markRead(data.message_id)
        }
        break
      }
      case 'bot_generating': {
        break
      }
    }
  }

  function sendMessage(data: Record<string, unknown>) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(data))
    }
  }

  return {
    connectionState,
    connect,
    disconnect,
    sendMessage,
  }
}
