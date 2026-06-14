import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const notificationsEnabled = ref(true)

export function useNotifications() {
  async function requestPermission() {
    try {
      const result = await invoke<string>('plugin:notification|request_permission')
      return result === 'granted'
    } catch {
      return false
    }
  }

  async function sendNotification(title: string, body: string, _chatId?: string) {
    if (!notificationsEnabled.value) return
    try {
      await invoke('plugin:notification|notify', {
        options: {
          title,
          body,
          channelId: 'pinto-messages',
        },
      })
    } catch {
      // fallback: no-op
    }
  }

  function toggle(enabled: boolean) {
    notificationsEnabled.value = enabled
  }

  return {
    notificationsEnabled,
    requestPermission,
    sendNotification,
    toggle,
  }
}
