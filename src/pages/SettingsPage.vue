<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/features/auth/auth.store'
import { useSettingsStore, type ThemeMode } from '@/features/settings/settings.store'
import AppButton from '@/shared/components/AppButton.vue'

const router = useRouter()
const auth = useAuthStore()
const settings = useSettingsStore()

async function handleLogout() {
  await auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="h-screen flex flex-col bg-background">
    <header class="h-14 px-6 border-b border-divider flex items-center gap-4 bg-surface shrink-0">
      <button class="text-ink-secondary hover:text-pinto transition-colors text-body-sm font-medium" @click="router.push('/')">← กลับ</button>
      <h1 class="text-headline-md text-ink">ตั้งค่า</h1>
    </header>
    <div class="flex-1 overflow-y-auto p-6 max-w-2xl mx-auto w-full space-y-6">

      <section class="bg-surface rounded-sm p-5 border border-hairline shadow-card">
        <h2 class="text-title-lg text-ink mb-4">บัญชี</h2>
        <div class="flex items-center gap-4 mb-5">
          <div class="w-12 h-12 rounded-full bg-pinto flex items-center justify-center text-white text-lg font-bold">
            {{ auth.currentUser?.name?.charAt(0) ?? '?' }}
          </div>
          <div>
            <div class="text-title-md text-ink font-semibold">{{ auth.currentUser?.name ?? 'Unknown' }}</div>
            <div class="text-body-sm text-ink-secondary">@{{ auth.currentUser?.username ?? '' }}</div>
          </div>
        </div>
        <AppButton variant="danger" size="sm" @click="handleLogout">ออกจากระบบ</AppButton>
      </section>

      <section class="bg-surface rounded-sm p-5 border border-hairline shadow-card">
        <h2 class="text-title-lg text-ink mb-4">รูปลักษณ์</h2>
        <div class="space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-body-md text-ink">ธีม</span>
            <select
              :value="settings.theme"
              class="text-body-sm bg-surface-muted border border-hairline rounded-sm px-3 py-1.5 text-ink focus:border-pinto transition-colors"
              @change="settings.setTheme(($event.target as HTMLSelectElement).value as ThemeMode)"
            >
              <option value="light">สว่าง</option>
              <option value="dark">มืด</option>
              <option value="system">ตามระบบ</option>
            </select>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-body-md text-ink">โหมดกะทัดรัด</span>
            <button
              :class="['w-10 h-6 rounded-full transition-colors duration-150', settings.compactMode ? 'bg-pinto' : 'bg-ink-disabled']"
              @click="settings.setCompactMode(!settings.compactMode)"
            >
              <span :class="['block w-5 h-5 bg-white rounded-full shadow-card transition-transform duration-150', settings.compactMode ? 'translate-x-4' : 'translate-x-0.5']" />
            </button>
          </div>
        </div>
      </section>

      <section class="bg-surface rounded-sm p-5 border border-hairline shadow-card">
        <h2 class="text-title-lg text-ink mb-4">การแจ้งเตือน</h2>
        <div class="flex items-center justify-between">
          <span class="text-body-md text-ink">เปิดการแจ้งเตือน</span>
          <button
            :class="['w-10 h-6 rounded-full transition-colors duration-150', settings.notificationsEnabled ? 'bg-pinto' : 'bg-ink-disabled']"
            @click="settings.setNotificationsEnabled(!settings.notificationsEnabled)"
          >
            <span :class="['block w-5 h-5 bg-white rounded-full shadow-card transition-transform duration-150', settings.notificationsEnabled ? 'translate-x-4' : 'translate-x-0.5']" />
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
