import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Chat, ChatType } from './chats.types'
import { initChatsApi, getChats, createChat as apiCreateChat, deleteChat as apiDeleteChat } from './chats.api'
import { useAuthStore } from '@/features/auth/auth.store'
import { getStoredToken } from '@/features/auth/token-storage'
import { env } from '@/app/env'
import { CHAT_PAGE_SIZE, TYPING_AUTO_CLEAR_MS } from '@/app/constants'

initChatsApi(getStoredToken, env.apiBaseUrl)

function normalizeChat(chat: Chat): Chat {
  const participant = chat.participants?.[0]
  const avatarUrl = chat.avatar_url ?? chat.use_avatar?.avatar_url ?? chat.avatar ?? participant?.use_avatar?.avatar_url ?? participant?.avatar ?? null
  const name = chat.name ?? participant?.name ?? participant?.username ?? null
  return {
    ...chat,
    name,
    avatar_url: avatarUrl,
    last_message_id: chat.last_message_id ?? chat.last_message?.message_id ?? null,
    last_message_preview: chat.last_message_preview ?? chat.last_message?.content ?? null,
    last_message_at: chat.last_message_at ?? chat.last_message?.created_at ?? chat.updated_at,
    is_pinned: chat.is_pinned ?? false,
    is_muted: chat.is_muted ?? false,
    is_archived: chat.is_archived ?? false,
    is_deleted: chat.is_deleted ?? false,
    created_at: chat.created_at ?? null,
  }
}

export const useChatsStore = defineStore('chats', () => {
  const chats = ref<Map<string, Chat>>(new Map())
  const selectedChatId = ref<string | null>(null)
  const isLoading = ref(false)
  const isLoadingMore = ref(false)
  const filterType = ref<ChatType | null>(null)
  const searchQuery = ref('')
  const page = ref(1)
  const hasMore = ref(true)
  const typingChats = ref<Set<string>>(new Set())
  const typingTimers = new Map<string, ReturnType<typeof setTimeout>>()

  const sortedChats = computed(() => {
    const list = Array.from(chats.value.values())
    if (filterType.value) {
      return list.filter(c => c.chat_type === filterType.value)
    }
    return list.sort((a, b) => {
      if (a.is_pinned && !b.is_pinned) return -1
      if (!a.is_pinned && b.is_pinned) return 1
      const aTime = a.last_message_at ?? a.updated_at
      const bTime = b.last_message_at ?? b.updated_at
      return new Date(bTime).getTime() - new Date(aTime).getTime()
    })
  })

  const selectedChat = computed(() => {
    if (!selectedChatId.value) return null
    return chats.value.get(selectedChatId.value) ?? null
  })

  const totalUnread = computed(() => {
    let count = 0
    for (const chat of chats.value.values()) {
      count += chat.unread_count
    }
    return count
  })

  async function loadChats(reset = false) {
    const auth = useAuthStore()
    if (!auth.token) return

    if (reset) {
      page.value = 1
      hasMore.value = true
      chats.value.clear()
    }

    if (isLoading.value || isLoadingMore.value) return
    isLoading.value = reset || chats.value.size === 0
    isLoadingMore.value = !isLoading.value

    const result = await getChats({ page: page.value, limit: CHAT_PAGE_SIZE, q: searchQuery.value || undefined, chat_type: filterType.value ?? undefined })
    if (result.ok) {
      for (const chat of result.data.chats) {
        const normalized = normalizeChat(chat)
        chats.value.set(normalized.chat_id, normalized)
      }
      hasMore.value = result.data.chats.length === CHAT_PAGE_SIZE
      page.value++
    }
    isLoading.value = false
    isLoadingMore.value = false
  }

  async function loadMore() {
    if (!hasMore.value || isLoadingMore.value) return
    await loadChats(false)
  }

  function selectChat(chatId: string | null) {
    selectedChatId.value = chatId
  }

  function updateChatFromWs(chat: Partial<Chat> & { chat_id: string }) {
    const existing = chats.value.get(chat.chat_id)
    if (existing) {
      chats.value.set(chat.chat_id, normalizeChat({ ...existing, ...chat }))
    }
  }

  function updateUnread(chatId: string, count: number) {
    const existing = chats.value.get(chatId)
    if (existing) {
      chats.value.set(chatId, { ...existing, unread_count: count })
    }
  }

  function moveChatToTop(chatId: string) {
    const existing = chats.value.get(chatId)
    if (existing) {
      chats.value.set(chatId, { ...existing, last_message_at: new Date().toISOString() })
    }
  }

  async function createChat(payload: { chat_type: string; participant_ids: string[]; name?: string }) {
    const result = await apiCreateChat(payload)
    if (result.ok) {
      const normalized = normalizeChat(result.data)
      chats.value.set(normalized.chat_id, normalized)
      return normalized
    }
    return null
  }

  async function removeChat(chatId: string) {
    const result = await apiDeleteChat(chatId)
    if (result.ok) {
      chats.value.delete(chatId)
      if (selectedChatId.value === chatId) {
        selectedChatId.value = null
      }
    }
    return result.ok
  }

  function setTyping(chatId: string, isTyping: boolean) {
    if (isTyping) {
      typingChats.value.add(chatId)
      if (typingTimers.has(chatId)) clearTimeout(typingTimers.get(chatId)!)
      typingTimers.set(chatId, setTimeout(() => {
        typingChats.value.delete(chatId)
        typingTimers.delete(chatId)
      }, TYPING_AUTO_CLEAR_MS))
    } else {
      typingChats.value.delete(chatId)
      if (typingTimers.has(chatId)) {
        clearTimeout(typingTimers.get(chatId)!)
        typingTimers.delete(chatId)
      }
    }
  }

  function setFilter(type: ChatType | null) {
    filterType.value = type
    loadChats(true)
  }

  function setSearch(q: string) {
    searchQuery.value = q
    loadChats(true)
  }

  return {
    chats,
    sortedChats,
    selectedChatId,
    selectedChat,
    totalUnread,
    isLoading,
    isLoadingMore,
    filterType,
    searchQuery,
    hasMore,
    typingChats,
    loadChats,
    loadMore,
    selectChat,
    updateChatFromWs,
    updateUnread,
    moveChatToTop,
    createChat,
    removeChat,
    setFilter,
    setSearch,
    setTyping,
  }
})
