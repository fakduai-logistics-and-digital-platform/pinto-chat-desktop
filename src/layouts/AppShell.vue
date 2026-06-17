<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { PhList, PhMagnifyingGlass, PhRobot, PhSignOut } from '@phosphor-icons/vue'
import { useChatsStore } from '@/features/chats/chats.store'
import { useAuthStore } from '@/features/auth/auth.store'
import { useSettingsStore } from '@/features/settings/settings.store'
import { connectWs, disconnectWs } from '@/features/realtime/ws-events'
import { useMessagesStore } from '@/features/messages/messages.store'
import { normalizeMessage } from '@/features/messages/message-normalizer'
import { messagesRepo } from '@/features/messages/messages.db'
import { initDatabase } from '@/features/local-db/db.client'
import ChatList from '@/features/chats/components/ChatList.vue'
import ChatRoom from '@/features/messages/components/ChatRoom.vue'
import AppAvatar from '@/shared/components/AppAvatar.vue'
import { useNotifications } from '@/features/notifications/notification.service'

const chatsStore = useChatsStore()
const messagesStore = useMessagesStore()
const auth = useAuthStore()
const settings = useSettingsStore()
const notifications = useNotifications()
const router = useRouter()
const connectionStatus = ref('disconnected')
const showUserMenu = ref(false)

function handleLogout() {
  showUserMenu.value = false
  auth.logout().then(() => router.push('/login'))
}

function handleSearchInput(event: Event) {
  chatsStore.setSearch((event.target as HTMLInputElement).value)
}

onMounted(async () => {
  settings.init()
  await initDatabase()
  await chatsStore.loadChats(true)

  connectWs(
    (payload) => {
      if (payload.type === 'new_message') {
        const msg = normalizeMessage(payload.data)
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
          notifications.sendNotification(
            chatsStore.chats.get(msg.chat_id)?.name ?? 'New message',
            msg.content ?? 'New message',
            msg.chat_id,
          )
        }
      } else if (payload.type === 'typing') {
        const data = payload.data as { chat_id: string; user_id: string; is_typing?: boolean }
        if (data.chat_id) chatsStore.setTyping(data.chat_id, data.is_typing ?? true)
      } else if (payload.type === 'stop_typing') {
        const data = payload.data as { chat_id: string }
        if (data.chat_id) chatsStore.setTyping(data.chat_id, false)
      } else if (payload.type === 'read_receipt') {
        const data = payload.data as { message_id: string }
        if (data.message_id) messagesRepo.markRead(data.message_id)
      }
    },
    (state) => { connectionStatus.value = state },
  )
})

onUnmounted(() => {
  disconnectWs()
})
</script>

<template>
  <div class="flex h-screen overflow-hidden bg-background text-ink">
    <aside class="flex w-16 shrink-0 flex-col border-r border-divider bg-surface">
      <div class="flex h-10 items-center gap-1 px-2">
        <span class="h-3 w-3 rounded-full bg-[#ff5f57]"></span>
        <span class="h-3 w-3 rounded-full bg-[#ffbd2e]"></span>
        <span class="h-3 w-3 rounded-full bg-[#28c840]"></span>
      </div>

      <div class="flex flex-1 flex-col items-center gap-5 py-3">
        <button
          class="relative flex h-10 w-10 items-center justify-center rounded-xl text-ink-secondary transition-colors hover:bg-pinto-50 hover:text-pinto"
          aria-label="เปิดเมนู"
          type="button"
          @click="showUserMenu = !showUserMenu"
        >
          <PhList class="h-6 w-6" weight="bold" />
        </button>

        <button class="relative flex h-10 w-10 items-center justify-center rounded-xl bg-pinto-50 text-pinto" type="button" aria-label="แชท" @click="router.push('/chat')">
          <PhRobot class="h-6 w-6" weight="bold" />
          <span v-if="chatsStore.totalUnread > 0" class="absolute right-0 top-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-[#ff5a6a] px-1 text-[10px] font-bold leading-none text-white">
            {{ chatsStore.totalUnread > 99 ? '99+' : chatsStore.totalUnread }}
          </span>
        </button>

        <button class="flex h-10 w-10 items-center justify-center rounded-xl text-ink-secondary transition-colors hover:bg-pinto-50 hover:text-pinto" type="button" aria-label="เพื่อน">
          <svg class="h-6 w-6" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-8 9a8 8 0 0 1 16 0H4Z"/></svg>
        </button>

        <button class="flex h-10 w-10 items-center justify-center rounded-xl text-ink-secondary transition-colors hover:bg-pinto-50 hover:text-pinto" type="button" aria-label="เพิ่มเพื่อน">
          <svg class="h-6 w-6" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M15 12a5 5 0 1 0-10 0 5 5 0 0 0 10 0Zm-12 9a7 7 0 0 1 14 0H3Zm15-11V7h-3V5h3V2h2v3h3v2h-3v3h-2Z"/></svg>
        </button>

        <div class="mt-auto flex flex-col items-center gap-4">
          <button class="flex h-10 w-10 items-center justify-center rounded-xl text-ink-secondary transition-colors hover:bg-pinto-50 hover:text-pinto" type="button" aria-label="ตั้งค่า" @click="router.push('/settings')">
            <svg class="h-6 w-6" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 7h16M7 12h10M10 17h4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          </button>
        </div>
      </div>

      <Transition name="menu-pop">
        <div
          v-if="showUserMenu"
          class="absolute left-14 top-12 z-30 w-[220px] overflow-hidden rounded-2xl border border-divider bg-surface p-2 shadow-card"
        >
          <div class="flex items-center gap-3 rounded-xl bg-pinto-50 px-3 py-3">
            <AppAvatar :name="auth.currentUser?.name ?? auth.currentUser?.username ?? 'User'" :src="auth.currentUser?.profile_image" size="sm" />
            <div class="min-w-0">
              <div class="truncate text-body-sm font-semibold text-ink">{{ auth.currentUser?.name ?? auth.currentUser?.username ?? 'User' }}</div>
              <div class="truncate text-caption text-ink-subtitle">บัญชี Pinto</div>
            </div>
          </div>
          <button
            class="mt-1 flex w-full items-center gap-3 rounded-xl px-3 py-3 text-left text-body-sm font-semibold text-ink transition-all hover:bg-surface-muted active:scale-[0.99]"
            type="button"
            @click="handleLogout"
          >
            <PhSignOut class="h-5 w-5 text-[#2ecc71]" weight="bold" />
            <span>ออกจากระบบ</span>
          </button>
        </div>
      </Transition>
    </aside>

    <aside class="flex w-[300px] shrink-0 flex-col border-r border-divider bg-surface">
      <div class="shrink-0 border-b border-divider px-3 pt-4">
        <div class="mb-4 flex items-center justify-between gap-3 text-[13px] font-semibold">
          <button :class="['border-b-2 pb-3', chatsStore.filterType === null ? 'border-pinto text-pinto' : 'border-transparent text-ink-secondary hover:text-pinto']" type="button" @click="chatsStore.setFilter(null)">ทั้งหมด</button>
          <button :class="['border-b-2 pb-3', chatsStore.filterType === 'direct' ? 'border-pinto text-pinto' : 'border-transparent text-ink-secondary hover:text-pinto']" type="button" @click="chatsStore.setFilter('direct')">เพื่อน</button>
          <button :class="['border-b-2 pb-3', chatsStore.filterType === 'group' ? 'border-pinto text-pinto' : 'border-transparent text-ink-secondary hover:text-pinto']" type="button" @click="chatsStore.setFilter('group')">กลุ่ม</button>
          <button class="pb-3 text-ink-disabled" type="button" disabled title="ยังไม่รองรับโอเพนแชท">โอเพนแชท</button>
          <button class="pb-3 text-ink-secondary hover:text-pinto" type="button" aria-label="ตัวกรอง">
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h2M10 17h10M8 5v4M16 15v4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
          </button>
        </div>
        <div class="mb-3 flex items-center gap-2">
          <div class="relative flex-1">
            <PhMagnifyingGlass class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-subtitle" weight="bold" />
            <input
              class="h-9 w-full rounded bg-surface-muted pl-9 pr-3 text-body-sm text-ink placeholder-ink-subtitle outline-none ring-1 ring-input-border focus:ring-pinto"
              placeholder="ค้นหาข้อความและห้องแชท"
              :value="chatsStore.searchQuery"
              @input="handleSearchInput"
            />
          </div>
          <button class="flex h-9 w-9 items-center justify-center rounded text-ink-secondary hover:bg-pinto-50 hover:text-pinto" type="button" aria-label="เรียงลำดับ">
            <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M8 5v14m0 0-3-3m3 3 3-3M16 19V5m0 0-3 3m3-3 3 3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
        </div>
      </div>
      <ChatList />
    </aside>

    <main class="min-w-0 flex-1 bg-background">
      <ChatRoom />
    </main>
  </div>
</template>

<style scoped>
.menu-pop-enter-active,
.menu-pop-leave-active {
  transition: opacity 0.16s cubic-bezier(0.16, 1, 0.3, 1), transform 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}

.menu-pop-enter-from,
.menu-pop-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}
</style>
