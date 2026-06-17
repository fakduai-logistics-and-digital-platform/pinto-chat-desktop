<script setup lang="ts">
import { ref, watch, nextTick, computed, onUnmounted } from 'vue'
import { useChatsStore } from '@/features/chats/chats.store'
import { useMessagesStore } from '@/features/messages/messages.store'
import { useAuthStore } from '@/features/auth/auth.store'
import AppAvatar from '@/shared/components/AppAvatar.vue'
import AppSkeleton from '@/shared/components/AppSkeleton.vue'
import AppDialog from '@/shared/components/AppDialog.vue'
import AppButton from '@/shared/components/AppButton.vue'
import MessageBubble from './MessageBubble.vue'
import MessageComposer from './MessageComposer.vue'
import DateDivider from './DateDivider.vue'
import TypingIndicator from './TypingIndicator.vue'
import { formatDateDivider, isSameDay } from '@/shared/utils/date'

const chatsStore = useChatsStore()
const messagesStore = useMessagesStore()
const auth = useAuthStore()

const messageListEl = ref<HTMLDivElement | null>(null)
const showScrollBtn = ref(false)
const replyMessage = ref<{ message_id: string; content: string; sender_name: string } | null>(null)
const pendingDeleteMessageId = ref<string | null>(null)
const toastMessage = ref('')
let toastTimer: ReturnType<typeof setTimeout> | null = null

const messages = computed(() => {
  if (!chatsStore.selectedChatId) return []
  return messagesStore.getMessages(chatsStore.selectedChatId)
})

const isTyping = computed(() => {
  if (!chatsStore.selectedChatId) return false
  return chatsStore.typingChats.has(chatsStore.selectedChatId)
})

const messageLookup = computed(() => {
  const lookup = new Map<string, typeof messages.value[number]>()
  for (const msg of messages.value) lookup.set(msg.message_id, msg)
  return lookup
})

function getSenderName(msg: typeof messages.value[number]) {
  if (!msg.raw_json) return 'Unknown'
  try {
    const raw = JSON.parse(msg.raw_json) as { sender?: { name?: string; display_name?: string; full_name?: string }; sender_name?: string }
    return raw.sender?.name ?? raw.sender?.display_name ?? raw.sender?.full_name ?? raw.sender_name ?? 'Unknown'
  } catch {
    return 'Unknown'
  }
}

function getReplyPreview(replyTo: string | null) {
  if (!replyTo) return null
  const replied = messageLookup.value.get(replyTo)
  if (!replied) return null
  return { sender_name: getSenderName(replied), content: replied.content }
}

function showToast(message: string) {
  toastMessage.value = message
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    toastMessage.value = ''
    toastTimer = null
  }, 2200)
}

function handleCopyMessage() {
  showToast('คัดลอกแล้ว')
}

function handleDeleteMessage(messageId: string) {
  pendingDeleteMessageId.value = messageId
}

async function confirmDeleteMessage() {
  if (!chatsStore.selectedChatId || !pendingDeleteMessageId.value) return
  await messagesStore.deleteMessage(chatsStore.selectedChatId, pendingDeleteMessageId.value)
  pendingDeleteMessageId.value = null
  showToast('ลบข้อความแล้ว')
}

function isCompactMessage(groupMessages: typeof messages.value, index: number) {
  if (index === 0) return false
  const current = groupMessages[index]
  const previous = groupMessages[index - 1]
  return current.sender_id === previous.sender_id
}

const groupedMessages = computed(() => {
  const groups: { date: string; messages: typeof messages.value }[] = []
  let currentDate = ''
  for (const msg of messages.value) {
    if (!currentDate || !isSameDay(msg.created_at, currentDate)) {
      currentDate = msg.created_at
      groups.push({ date: msg.created_at, messages: [msg] })
    } else {
      groups[groups.length - 1].messages.push(msg)
    }
  }
  return groups
})

function scrollToBottom(smooth = true) {
  nextTick(() => {
    if (messageListEl.value) {
      messageListEl.value.scrollTo({
        top: messageListEl.value.scrollHeight,
        behavior: smooth ? 'smooth' : 'instant',
      })
    }
  })
}

async function loadOlderMessages() {
  if (!chatsStore.selectedChatId) return
  const el = messageListEl.value
  if (!el) return
  const prevHeight = el.scrollHeight
  await messagesStore.loadOlderMessages(chatsStore.selectedChatId)
  await nextTick()
  el.scrollTop = el.scrollHeight - prevHeight
}

function handleScroll() {
  if (!messageListEl.value) return
  const { scrollTop, scrollHeight, clientHeight } = messageListEl.value
  showScrollBtn.value = scrollHeight - scrollTop - clientHeight > 200
  if (scrollTop < 80 && !messagesStore.isLoadingMore && messagesStore.hasMoreMessages) {
    loadOlderMessages()
  }
}

function handleReply(msg: { message_id: string; content: string; sender_name: string }) {
  replyMessage.value = msg
}

function clearReply() {
  replyMessage.value = null
}

watch(
  () => messages.value.length,
  () => {
    if (messageListEl.value) {
      const { scrollTop, scrollHeight, clientHeight } = messageListEl.value
      if (scrollHeight - scrollTop - clientHeight < 300) scrollToBottom()
    }
  },
)

watch(
  () => chatsStore.selectedChatId,
  async (chatId) => {
    if (!chatId) return
    await messagesStore.loadMessages(chatId, true)
    scrollToBottom(false)
  },
  { immediate: true },
)

onUnmounted(() => {
  if (toastTimer) clearTimeout(toastTimer)
})
</script>

<template>
  <div v-if="!chatsStore.selectedChatId" class="flex h-full items-center justify-center bg-background">
    <div class="flex flex-col items-center text-center text-ink-subtitle">
      <div class="mb-6 flex h-24 w-24 items-center justify-center rounded-full bg-pinto-50 text-pinto">
        <svg class="h-16 w-16" viewBox="0 0 120 96" fill="currentColor" aria-hidden="true">
          <path d="M60 8c29.8 0 54 17.7 54 39.5 0 15.6-12.3 29.1-30.2 35.5l-18.6 10.2 3.1-7.3c-2.7.3-5.5.5-8.3.5-29.8 0-54-17.7-54-39.5S30.2 8 60 8Z" />
        </svg>
      </div>
      <div class="text-title-md font-semibold text-ink">Pinto</div>
      <div class="mt-4 text-body-sm text-ink-subtitle">เริ่มการแชทใหม่!</div>
    </div>
  </div>

  <div v-else class="chat-panel h-full flex flex-col bg-surface relative overflow-hidden">
    <!-- Header -->
    <div class="relative z-20 h-16 px-4 flex items-center justify-between border-b border-divider shrink-0 bg-surface">
      <div class="flex items-center gap-3">
        <AppAvatar
          :name="chatsStore.selectedChat?.name ?? 'Unknown'"
          :src="chatsStore.selectedChat?.avatar_url"
          size="md"
        />
        <div>
          <div class="flex items-center gap-2 text-title-md text-ink font-semibold">
            <span>{{ chatsStore.selectedChat?.name ?? 'Unknown' }}</span>
          </div>
          <div class="text-caption text-ink-subtitle">
            {{ chatsStore.selectedChat?.chat_type === 'group' ? 'กลุ่ม' : 'แชทส่วนตัว' }}
          </div>
        </div>
      </div>
      <button class="h-10 w-10 rounded-full text-ink-secondary hover:bg-surface-muted hover:text-ink transition-colors" aria-label="ตัวเลือก">
        <svg class="mx-auto h-5 w-5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="12" cy="5" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="12" cy="19" r="1.8"/></svg>
      </button>
    </div>

    <!-- Messages -->
    <div
      ref="messageListEl"
      class="chat-wall relative z-10 flex-1 overflow-y-auto px-6 py-4"
      @scroll="handleScroll"
    >
      <AppSkeleton v-if="messagesStore.isLoadingMessages" :count="6" />

      <div v-else-if="messages.length === 0" class="flex h-full items-center justify-center">
        <div class="rounded-2xl bg-white/80 px-5 py-4 text-center shadow-[0_10px_30px_rgba(67,61,77,0.08)]">
          <div class="text-body-sm font-semibold text-ink">ยังไม่มีข้อความ</div>
          <div class="mt-1 text-caption text-ink-secondary">ส่งข้อความแรก!</div>
        </div>
      </div>

      <template v-else>
        <div v-if="messagesStore.isLoadingMore" class="text-center py-2 text-caption text-ink-hint">กำลังโหลดข้อความเก่า...</div>

        <template v-for="(group, gi) in groupedMessages" :key="gi">
          <DateDivider :date="formatDateDivider(group.date)" />
          <MessageBubble
            v-for="(msg, mi) in group.messages"
            :key="msg.message_id"
            :message="msg"
            :is-self="msg.sender_id === auth.currentUser?.user_id"
            :compact="isCompactMessage(group.messages, mi)"
            :show-sender="!isCompactMessage(group.messages, mi)"
            :reply-preview="getReplyPreview(msg.reply_to)"
            @reply="handleReply"
            @copied="handleCopyMessage"
            @delete="handleDeleteMessage"
            @retry="messagesStore.retryMessage(chatsStore.selectedChatId!, msg.message_id)"
          />
        </template>
      </template>
    </div>

    <!-- Typing indicator -->
    <TypingIndicator
      v-if="isTyping"
      class="relative z-10"
      :name="chatsStore.selectedChat?.name ?? ''"
    />

    <!-- Scroll to bottom -->
    <Transition name="fade">
      <button
        v-if="showScrollBtn"
        class="absolute bottom-24 right-6 z-10 w-10 h-10 bg-surface border border-divider rounded-full shadow-card flex items-center justify-center text-ink-secondary hover:text-pinto hover:border-pinto transition-all duration-150"
        @click="scrollToBottom()"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 3v10M4 9l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
    </Transition>

    <!-- Composer -->
    <MessageComposer
      :chat-id="chatsStore.selectedChatId!"
      :reply-to="replyMessage"
      @sent="scrollToBottom()"
      @cancel-reply="clearReply"
    />

    <Transition name="toast">
      <div
        v-if="toastMessage"
        class="absolute bottom-24 left-1/2 z-30 -translate-x-1/2 rounded-capsule bg-ink px-4 py-2 text-label-sm text-white shadow-toast"
      >
        {{ toastMessage }}
      </div>
    </Transition>

    <AppDialog
      :open="pendingDeleteMessageId !== null"
      title="ลบข้อความ?"
      max-width="max-w-sm"
      @close="pendingDeleteMessageId = null"
    >
      <p class="text-body-sm text-ink-secondary">ข้อความนี้จะถูกลบจากเครื่องนี้ก่อน ยังไม่ได้ลบจากเซิร์ฟเวอร์</p>
      <div class="mt-6 flex justify-end gap-2">
        <AppButton variant="ghost" label="ยกเลิก" @click="pendingDeleteMessageId = null" />
        <AppButton variant="danger" label="ลบ" @click="confirmDeleteMessage" />
      </div>
    </AppDialog>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.15s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.16s ease, transform 0.16s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translate(-50%, 6px) scale(0.98);
}

.chat-panel::before {
  position: absolute;
  z-index: 0;
  background-image: url("@/assets/chat-pattern-transparent.webp");
  background-position: center;
  background-repeat: repeat;
  background-size: auto 60%;
  content: "";
  inset-block: 64px 0;
  inset-inline: 0;
  opacity: 0.1;
  pointer-events: none;
}

.chat-wall {
  background-color: transparent;
}
</style>
