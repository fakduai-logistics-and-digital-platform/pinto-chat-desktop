<script setup lang="ts">
import { ref } from 'vue'
import { useMessagesStore } from '../messages.store'
import { useAuthStore } from '@/features/auth/auth.store'
import { MAX_MESSAGE_LENGTH } from '@/app/constants'
import { debounce } from '@/shared/utils/debounce'
import { sendTyping } from '@/features/chats/chats.api'

const props = defineProps<{
  chatId: string
  replyTo?: { message_id: string; content: string; sender_name: string } | null
}>()

const emit = defineEmits<{
  sent: []
  cancelReply: []
}>()

const messagesStore = useMessagesStore()
const auth = useAuthStore()
const text = ref('')
const textareaRef = ref<HTMLTextAreaElement | null>(null)

function autoResize() {
  if (!textareaRef.value) return
  textareaRef.value.style.height = 'auto'
  textareaRef.value.style.height = Math.min(textareaRef.value.scrollHeight, 120) + 'px'
}

const debouncedTyping = debounce(() => {
  sendTyping(props.chatId).catch(() => {})
}, 800)

function onInput() {
  autoResize()
  debouncedTyping()
}

async function send() {
  const content = text.value.trim()
  if (!content || !auth.currentUser) return
  text.value = ''
  if (textareaRef.value) textareaRef.value.style.height = 'auto'
  await messagesStore.sendText(props.chatId, content, props.replyTo?.message_id)
  emit('sent')
  emit('cancelReply')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}
</script>

<template>
  <div class="relative z-10 shrink-0 px-6 py-3">
    <div v-if="replyTo" class="mx-auto mb-2 flex max-w-3xl items-center justify-between rounded-sm border border-hairline bg-surface-muted px-3 py-2">
      <div class="min-w-0">
        <div class="text-caption text-pinto font-semibold">ตอบ {{ replyTo.sender_name }}</div>
        <div class="text-caption text-ink-subtitle truncate">{{ replyTo.content }}</div>
      </div>
      <button class="text-ink-subtitle hover:text-ink ml-2 transition-colors" @click="$emit('cancelReply')">✕</button>
    </div>
    <div class="mx-auto flex max-w-3xl items-end gap-2 rounded-[24px] border border-divider bg-white px-3 py-2 shadow-[0_10px_30px_rgba(67,61,77,0.08)]">
      <button class="mb-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-ink-secondary hover:bg-surface-muted hover:text-pinto transition-colors" type="button" aria-label="แนบไฟล์">
        <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M21 11.5 12.2 20.3a6 6 0 0 1-8.5-8.5l9.5-9.5a4 4 0 0 1 5.7 5.7l-9.5 9.5a2 2 0 0 1-2.8-2.8l8.8-8.8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
      <button class="mb-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-ink-hint hover:bg-surface-muted hover:text-pinto transition-colors" type="button" aria-label="เครื่องมือช่วยเขียน">
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2l1.6 4.8L18 8.4l-4.4 1.6L12 15l-1.6-5L6 8.4l4.4-1.6L12 2Zm7 10 1 3 3 1-3 1-1 3-1-3-3-1 3-1 1-3ZM5 13l.8 2.2L8 16l-2.2.8L5 19l-.8-2.2L2 16l2.2-.8L5 13Z"/></svg>
      </button>
      <textarea
        ref="textareaRef"
        v-model="text"
        :maxlength="MAX_MESSAGE_LENGTH"
        placeholder="ข้อความ"
        rows="1"
        class="max-h-32 flex-1 resize-none border-0 bg-transparent px-1 py-2 text-body-sm text-content placeholder-ink-subtitle outline-none focus:ring-0"
        @input="onInput"
        @keydown="onKeydown"
      />
      <button
        :disabled="!text.trim()"
        :class="[
          'mb-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-full transition-all duration-150 select-none',
          text.trim()
            ? 'bg-pinto text-white hover:bg-pinto-600 active:bg-pinto-700 shadow-btn active:shadow-none active:scale-[0.98]'
            : 'text-ink-secondary cursor-not-allowed opacity-70',
        ]"
        @click="send"
      >
        <svg class="h-5 w-5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M4 4.5v15l16-7.5-16-7.5Zm2 3.1 9.2 4.4L6 16.4v-3.2l5.5-1.2L6 10.8V7.6Z"/></svg>
      </button>
    </div>
  </div>
</template>
