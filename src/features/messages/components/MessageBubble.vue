<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Message } from '../messages.types'
import { formatMessageTime } from '@/shared/utils/date'
import AppAvatar from '@/shared/components/AppAvatar.vue'

const props = defineProps<{
  message: Message
  isSelf: boolean
  replyPreview?: { sender_name: string; content: string | null } | null
}>()

const emit = defineEmits<{
  reply: [msg: { message_id: string; content: string; sender_name: string }]
  retry: []
  delete: [messageId: string]
}>()

const menuOpen = ref(false)

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
const messageText = computed(() => props.message.content ?? '')

function closeMenu() {
  menuOpen.value = false
}

function handleReply() {
  emit('reply', {
    message_id: props.message.message_id,
    content: messageText.value,
    sender_name: senderName.value,
  })
  closeMenu()
}

async function handleCopy() {
  if (!messageText.value) return
  await navigator.clipboard.writeText(messageText.value)
  closeMenu()
}

function handleDelete() {
  emit('delete', props.message.message_id)
  closeMenu()
}
</script>

<template>
  <div :class="['mb-4 flex', isSelf ? 'justify-end' : 'justify-start']" @contextmenu.prevent="menuOpen = true">
    <AppAvatar
      v-if="!isSelf"
      class="mr-2 mt-1"
      :name="senderName"
      :src="senderAvatarUrl"
      size="sm"
    />
    <div :class="['group relative max-w-[66%]', isSelf ? 'text-right' : 'text-left']">
      <button
        class="absolute top-1 z-20 hidden h-7 w-7 items-center justify-center rounded-full border border-divider bg-surface text-ink-secondary shadow-card transition-colors hover:text-pinto group-hover:flex"
        :class="isSelf ? '-left-9' : '-right-9'"
        type="button"
        aria-label="เมนูข้อความ"
        @click="menuOpen = !menuOpen"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/></svg>
      </button>

      <div
        v-if="menuOpen"
        class="absolute top-9 z-30 w-40 overflow-hidden rounded-xl border border-divider bg-surface py-1 text-left shadow-card"
        :class="isSelf ? 'right-0' : 'left-0'"
        @mouseleave="closeMenu"
      >
        <button class="flex w-full px-3 py-2 text-body-sm text-ink hover:bg-surface-muted" type="button" @click="handleReply">ตอบกลับ</button>
        <button
          class="flex w-full px-3 py-2 text-body-sm hover:bg-surface-muted"
          :class="messageText ? 'text-ink' : 'text-ink-disabled'"
          type="button"
          :disabled="!messageText"
          @click="handleCopy"
        >
          คัดลอก
        </button>
        <button class="flex w-full px-3 py-2 text-body-sm text-ink-disabled" type="button" disabled>ส่งต่อ</button>
        <button class="flex w-full px-3 py-2 text-body-sm text-ink-disabled" type="button" disabled>ปักหมุด</button>
        <button class="flex w-full px-3 py-2 text-body-sm text-error-strong hover:bg-error-container" type="button" @click="handleDelete">ลบ</button>
      </div>

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

        <div v-if="message.reply_to" :class="['mb-2 border-l-2 pl-2 text-caption', isSelf ? 'border-white/40 text-white/80' : 'border-pinto/40 text-ink-secondary']">
          <div class="font-semibold">{{ replyPreview?.sender_name ?? 'ข้อความที่ตอบกลับ' }}</div>
          <div class="truncate">{{ replyPreview?.content ?? 'กำลังตอบข้อความ...' }}</div>
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
