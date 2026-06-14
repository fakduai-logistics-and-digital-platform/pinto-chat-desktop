<script setup lang="ts">
import { computed } from 'vue'
import type { Message } from '../messages.types'
import { formatMessageTime } from '@/shared/utils/date'
import AppAvatar from '@/shared/components/AppAvatar.vue'

const props = defineProps<{
  message: Message
  isSelf: boolean
}>()

defineEmits<{
  reply: [msg: { message_id: string; content: string; sender_name: string }]
  retry: []
}>()

type RawSender = {
  name?: string
  display_name?: string
  full_name?: string
  avatar?: string | null
  avatar_url?: string | null
  profile_image?: string | null
}

const sender = computed<RawSender>(() => {
  if (!props.message.raw_json) return {}
  try {
    const raw = JSON.parse(props.message.raw_json) as {
      sender?: RawSender
      sender_name?: string
      sender_avatar_url?: string | null
      sender_avatar?: string | null
      sender_profile_image?: string | null
    }
    return {
      name: raw.sender?.name ?? raw.sender?.display_name ?? raw.sender?.full_name ?? raw.sender_name,
      avatar_url: raw.sender?.avatar_url ?? raw.sender?.avatar ?? raw.sender?.profile_image ?? raw.sender_avatar_url ?? raw.sender_avatar ?? raw.sender_profile_image ?? null,
    }
  } catch {
    return {}
  }
})

const senderName = computed(() => sender.value.name ?? 'Unknown')
const senderAvatarUrl = computed(() => sender.value.avatar_url ?? null)
</script>

<template>
  <div :class="['mb-4 flex', isSelf ? 'justify-end' : 'justify-start']">
    <AppAvatar
      v-if="!isSelf"
      class="mr-2 mt-1"
      :name="senderName"
      :src="senderAvatarUrl"
      size="sm"
    />
    <div :class="['group relative max-w-[66%]', isSelf ? 'text-right' : 'text-left']">
      <div
        :class="[
          'px-4 py-3 text-body-sm leading-relaxed break-words shadow-[0_10px_30px_rgba(67,61,77,0.08)]',
          isSelf
            ? 'rounded-2xl rounded-br-md bg-[#2ECC71] text-white'
            : 'rounded-2xl rounded-bl-md bg-white text-ink',
        ]"
      >
        <div v-if="!isSelf" class="mb-1 text-caption font-semibold text-[#2ECC71]">
          {{ senderName }}
        </div>

        <div v-if="message.reply_to" :class="['mb-1 pl-2 border-l-2 text-caption', isSelf ? 'border-white/40 opacity-75' : 'border-pinto/40 text-ink-secondary']">
          กำลังตอบข้อความ...
        </div>

        <div v-if="message.message_type === 'image' && message.media_url" class="mb-1">
          <img :src="message.media_url" class="max-w-full rounded-sm" loading="lazy" />
        </div>

        <div v-if="message.message_type === 'file'" :class="['flex items-center gap-2 p-2 rounded-sm', isSelf ? 'bg-white/15' : 'bg-surface-muted']">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M14 10v2a2 2 0 01-2 2H4a2 2 0 01-2-2V4a2 2 0 012-2h5l4 4v1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/><path d="M10 2v4h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          <span class="truncate text-body-sm">{{ message.file_name ?? 'File' }}</span>
        </div>

        <div v-if="message.content">{{ message.content }}</div>

        <div class="mt-2 flex items-center justify-end gap-1">
          <span :class="['text-[10px] font-medium leading-none', isSelf ? 'text-white/75' : 'text-ink-subtitle']">
            {{ formatMessageTime(message.created_at) }}
          </span>
          <span v-if="isSelf" class="text-[10px]">
            <template v-if="message.status === 'sending' || message.status === 'queued'">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="opacity-60"><circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1.2"/><path d="M6 3v3l2 1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
            </template>
            <template v-else-if="message.status === 'sent'">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="opacity-60"><path d="M2 6l3 3 5-5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </template>
            <template v-else-if="message.status === 'delivered' || message.status === 'read'">
              <svg width="16" height="12" viewBox="0 0 16 12" fill="none" class="opacity-60"><path d="M1 6l3 3 5-5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/><path d="M5 6l3 3 5-5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </template>
            <template v-else-if="message.status === 'failed'">
              <button class="text-error hover:text-error-strong" @click.stop="$emit('retry')">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1.2"/><path d="M6 4v2M6 8h.01" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
              </button>
            </template>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
