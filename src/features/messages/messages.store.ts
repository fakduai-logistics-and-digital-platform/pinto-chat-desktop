import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Message } from './messages.types'
import { messagesRepo } from './messages.db'
import { initMessagesApi, sendTextMessage, sendFileMessage, getMessageHistory, markRead as apiMarkRead } from './messages.api'
import { outboxRepo } from './outbox.service'
import { useAuthStore } from '@/features/auth/auth.store'
import { useChatsStore } from '@/features/chats/chats.store'
import { normalizeMessage, createTempMessage, updateMessageStatus } from './message-normalizer'
import { v4 as uuid } from '@/shared/utils/uuid'
import { MESSAGE_PAGE_SIZE } from '@/app/constants'
import { getStoredToken } from '@/features/auth/token-storage'
import { env } from '@/app/env'

initMessagesApi(getStoredToken, env.apiBaseUrl)

export const useMessagesStore = defineStore('messages', () => {
  const messagesByChat = ref<Map<string, Message[]>>(new Map())
  const isLoadingMessages = ref(false)
  const isLoadingMore = ref(false)
  const hasMoreMessages = ref(true)
  const sendingMessages = ref<Map<string, Message>>(new Map())

  function getMessages(chatId: string): Message[] {
    return messagesByChat.value.get(chatId) ?? []
  }

  async function loadMessages(chatId: string, reset = false) {
    if (isLoadingMessages.value || isLoadingMore.value) return
    isLoadingMessages.value = true

    if (reset) {
      hasMoreMessages.value = true
    }

    const localMessages = await messagesRepo.getByChatId(chatId, MESSAGE_PAGE_SIZE)
    if (localMessages.length > 0) {
      const reversed = [...localMessages].reverse()
      messagesByChat.value.set(chatId, reversed)
      hasMoreMessages.value = localMessages.length === MESSAGE_PAGE_SIZE
    } else {
      const result = await getMessageHistory(chatId, { limit: MESSAGE_PAGE_SIZE })
      if (result.ok) {
        const normalized = result.data.messages.map(normalizeMessage)
        for (const msg of normalized) {
          await messagesRepo.upsert(msg)
        }
        messagesByChat.value.set(chatId, [...normalized].reverse())
        hasMoreMessages.value = normalized.length === MESSAGE_PAGE_SIZE
      }
    }
    isLoadingMessages.value = false
  }

  async function loadOlderMessages(chatId: string) {
    if (!hasMoreMessages.value || isLoadingMore.value) return
    isLoadingMore.value = true

    const current = messagesByChat.value.get(chatId) ?? []
    const oldest = current[0]
    if (oldest) {
      const older = await messagesRepo.getByChatId(chatId, MESSAGE_PAGE_SIZE, oldest.created_at)
      if (older.length > 0) {
        const reversed = [...older].reverse()
        messagesByChat.value.set(chatId, [...reversed, ...current])
        hasMoreMessages.value = older.length === MESSAGE_PAGE_SIZE
      } else {
        hasMoreMessages.value = false
      }
    }
    isLoadingMore.value = false
  }

  async function sendText(chatId: string, content: string, replyTo?: string) {
    const auth = useAuthStore()
    if (!auth.currentUser) return

    const tempMsg = createTempMessage(chatId, auth.currentUser.user_id, content, 'text', replyTo)
    const current = messagesByChat.value.get(chatId) ?? []
    messagesByChat.value.set(chatId, [...current, tempMsg])
    sendingMessages.value.set(tempMsg.message_id, tempMsg)

    const outboxId = uuid()
    await outboxRepo.insert({
      id: outboxId,
      chat_id: chatId,
      client_temp_id: tempMsg.client_temp_id ?? tempMsg.message_id,
      content,
      message_type: 'text',
      reply_to: replyTo ?? null,
      local_file_path: null,
      status: 'queued',
      retry_count: 0,
      last_error: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    })

    await messagesRepo.upsert(tempMsg)

    const result = await sendTextMessage(chatId, {
      content,
      message_type: 'text',
      reply_to: replyTo ?? undefined,
      client_temp_id: tempMsg.client_temp_id ?? undefined,
    })

    if (result.ok) {
      const serverMsg = normalizeMessage(result.data as unknown as Record<string, unknown>)
      const updated = updateMessageStatus(tempMsg, 'sent', serverMsg.message_id)
      await messagesRepo.updateStatusByTempId(tempMsg.message_id, serverMsg.message_id, 'sent')
      await outboxRepo.remove(outboxId)
      replaceTempMessage(chatId, tempMsg.message_id, updated)
      sendingMessages.value.delete(tempMsg.message_id)

      const chatsStore = useChatsStore()
      chatsStore.updateChatFromWs({
        chat_id: chatId,
        last_message_id: serverMsg.message_id,
        last_message_preview: content.slice(0, 100),
        last_message_at: serverMsg.created_at,
      })
    } else {
      const failed = updateMessageStatus(tempMsg, 'failed')
      await messagesRepo.updateStatus(tempMsg.message_id, 'failed')
      await outboxRepo.incrementRetry(outboxId, result.error)
      replaceTempMessage(chatId, tempMsg.message_id, failed)
      sendingMessages.value.delete(tempMsg.message_id)
    }
  }

  async function sendFile(chatId: string, file: File, replyTo?: string) {
    const auth = useAuthStore()
    if (!auth.currentUser) return

    const tempMsg = createTempMessage(chatId, auth.currentUser.user_id, file.name, file.type.startsWith('image/') ? 'image' : 'file', replyTo)
    const current = messagesByChat.value.get(chatId) ?? []
    messagesByChat.value.set(chatId, [...current, tempMsg])
    await messagesRepo.upsert(tempMsg)

    const result = await sendFileMessage(chatId, file, replyTo)
    if (result.ok) {
      const serverMsg = normalizeMessage(result.data as unknown as Record<string, unknown>)
      const updated = updateMessageStatus(tempMsg, 'sent', serverMsg.message_id)
      await messagesRepo.updateStatusByTempId(tempMsg.message_id, serverMsg.message_id, 'sent')
      replaceTempMessage(chatId, tempMsg.message_id, updated)
    } else {
      const failed = updateMessageStatus(tempMsg, 'failed')
      await messagesRepo.updateStatus(tempMsg.message_id, 'failed')
      replaceTempMessage(chatId, tempMsg.message_id, failed)
    }
  }

  function addMessageFromWs(msg: Message) {
    const current = messagesByChat.value.get(msg.chat_id) ?? []
    const exists = current.find(m => m.message_id === msg.message_id)
    if (exists) return
    messagesByChat.value.set(msg.chat_id, [...current, msg])
  }

  function replaceTempMessage(chatId: string, tempId: string, newMsg: Message) {
    const current = messagesByChat.value.get(chatId) ?? []
    const idx = current.findIndex(m => m.message_id === tempId || m.client_temp_id === tempId)
    if (idx >= 0) {
      const updated = [...current]
      updated[idx] = newMsg
      messagesByChat.value.set(chatId, updated)
    }
  }

  async function markChatAsRead(chatId: string) {
    await messagesRepo.markChatRead(chatId)
    await apiMarkRead(chatId)
  }

  async function retryMessage(chatId: string, messageId: string) {
    const current = messagesByChat.value.get(chatId) ?? []
    const msg = current.find(m => m.message_id === messageId)
    if (!msg || !msg.content) return
    const updated = updateMessageStatus(msg, 'sending')
    replaceTempMessage(chatId, messageId, updated)
    await sendText(chatId, msg.content, msg.reply_to ?? undefined)
  }

  async function deleteMessage(chatId: string, messageId: string) {
    await messagesRepo.markDeleted(messageId)
    const current = messagesByChat.value.get(chatId) ?? []
    messagesByChat.value.set(chatId, current.filter(m => m.message_id !== messageId))
  }

  return {
    messagesByChat,
    isLoadingMessages,
    isLoadingMore,
    hasMoreMessages,
    sendingMessages,
    getMessages,
    loadMessages,
    loadOlderMessages,
    sendText,
    sendFile,
    addMessageFromWs,
    markChatAsRead,
    retryMessage,
    deleteMessage,
  }
})
