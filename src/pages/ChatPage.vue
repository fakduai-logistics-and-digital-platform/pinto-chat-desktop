<script setup lang="ts">
import { watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AppShell from '@/layouts/AppShell.vue'
import { useChatsStore } from '@/features/chats/chats.store'

const route = useRoute()
const router = useRouter()
const chatsStore = useChatsStore()

watch(
  () => route.params.id as string | undefined,
  (id) => {
    if (id && id !== chatsStore.selectedChatId) {
      chatsStore.selectChat(id)
    } else if (!id && chatsStore.selectedChatId) {
      chatsStore.selectChat(null)
    }
  },
  { immediate: true },
)

watch(
  () => chatsStore.selectedChatId,
  (chatId) => {
    const routeId = route.params.id as string | undefined
    if (chatId && chatId !== routeId) {
      router.push({ name: 'chat-room', params: { id: chatId } })
    } else if (!chatId && routeId) {
      router.push({ name: 'chat' })
    }
  },
)
</script>

<template>
  <AppShell />
</template>
