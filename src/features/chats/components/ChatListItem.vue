<script setup lang="ts">
import type { Chat } from '../chats.types'
import AppAvatar from '@/shared/components/AppAvatar.vue'
import { formatChatTime } from '@/shared/utils/date'
import { truncate } from '@/shared/utils/text'

defineProps<{
  chat: Chat
  active: boolean
}>()

defineEmits(['select'])
</script>

<template>
  <div
    :class="[
      'flex items-center gap-3 px-4 py-3 cursor-pointer transition-colors duration-150',
      active ? 'bg-pinto-50' : 'hover:bg-surface-muted',
    ]"
    @click="$emit('select')"
  >
    <AppAvatar
      :name="chat.name ?? 'Unknown'"
      :src="chat.avatar_url"
      :online="chat.chat_type === 'direct'"
      size="lg"
    />
    <div class="flex-1 min-w-0">
      <div class="flex items-center justify-between gap-2">
        <div class="flex min-w-0 items-center gap-2">
          <span class="truncate text-body-sm font-semibold text-ink">
            {{ chat.name ?? 'Unknown' }}
          </span>
        </div>
        <span class="shrink-0 text-caption text-ink-subtitle">
          {{ formatChatTime(chat.last_message_at) }}
        </span>
      </div>
      <div class="mt-1 flex items-center justify-between gap-2">
        <span class="truncate text-caption text-ink-secondary">
          {{ truncate(chat.last_message_preview ?? 'ยังไม่มีข้อความ', 42) }}
        </span>
        <span
          v-if="chat.unread_count > 0"
          class="flex h-5 min-w-[20px] shrink-0 items-center justify-center rounded-full bg-error-strong px-1.5 text-caption font-semibold text-white"
        >
          {{ chat.unread_count > 99 ? '99+' : chat.unread_count }}
        </span>
      </div>
    </div>
  </div>
</template>
