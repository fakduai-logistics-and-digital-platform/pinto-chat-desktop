<script setup lang="ts">
import { onMounted } from 'vue'
import { useChatsStore } from '@/features/chats/chats.store'
import ChatListItem from './ChatListItem.vue'
import AppSkeleton from '@/shared/components/AppSkeleton.vue'

const chatsStore = useChatsStore()

function handleScroll(e: Event) {
  const el = e.target as HTMLDivElement
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 100) {
    chatsStore.loadMore()
  }
}

onMounted(() => {
  if (chatsStore.sortedChats.length === 0) {
    chatsStore.loadChats(true)
  }
})
</script>

<template>
  <div class="flex h-full flex-col bg-[#303236]">
    <div class="flex-1 overflow-y-auto" @scroll="handleScroll">
      <AppSkeleton v-if="chatsStore.isLoading" :count="8" />
      <div
        v-else-if="chatsStore.sortedChats.length === 0"
        class="flex h-full flex-col items-center justify-center px-6 text-center text-[#9a9ca0]"
      >
        <div class="mb-3 flex h-14 w-14 items-center justify-center rounded-full bg-[#3b3c40] text-[#77797d]">
          <svg class="h-8 w-8" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M4 5h16v11H7.5L4 19.5V5Z"/></svg>
        </div>
        <div class="text-body-sm font-semibold text-[#d7d7d7]">ยังไม่มีแชท</div>
        <div class="mt-1 text-caption">เริ่มต้นสนทนาเพื่อแสดงที่นี่</div>
      </div>
      <template v-else>
        <ChatListItem
          v-for="chat in chatsStore.sortedChats"
          :key="chat.chat_id"
          :chat="chat"
          :active="chatsStore.selectedChatId === chat.chat_id"
          @select="chatsStore.selectChat(chat.chat_id)"
        />
      </template>
      <div v-if="chatsStore.isLoadingMore" class="p-4 text-center text-body-sm text-[#9a9ca0]">กำลังโหลด...</div>
    </div>
  </div>
</template>
