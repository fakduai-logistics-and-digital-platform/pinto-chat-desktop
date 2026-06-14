import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ThemeMode = 'light' | 'dark' | 'system'

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<ThemeMode>((localStorage.getItem('pinto_theme') as ThemeMode) ?? 'light')
  const fontSize = ref(Number(localStorage.getItem('pinto_font_size') ?? '14'))
  const compactMode = ref(localStorage.getItem('pinto_compact') === 'true')
  const notificationsEnabled = ref(localStorage.getItem('pinto_notifications') !== 'false')
  const showPreview = ref(localStorage.getItem('pinto_preview') !== 'false')

  function setTheme(t: ThemeMode) {
    theme.value = t
    localStorage.setItem('pinto_theme', t)
    applyTheme(t)
  }

  function applyTheme(t: ThemeMode) {
    const root = document.documentElement
    if (t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
      root.classList.add('dark')
    } else {
      root.classList.remove('dark')
    }
  }

  function setFontSize(s: number) {
    fontSize.value = s
    localStorage.setItem('pinto_font_size', String(s))
  }

  function setCompactMode(c: boolean) {
    compactMode.value = c
    localStorage.setItem('pinto_compact', String(c))
  }

  function setNotificationsEnabled(n: boolean) {
    notificationsEnabled.value = n
    localStorage.setItem('pinto_notifications', String(n))
  }

  function setShowPreview(p: boolean) {
    showPreview.value = p
    localStorage.setItem('pinto_preview', String(p))
  }

  function init() {
    applyTheme(theme.value)
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (theme.value === 'system') applyTheme('system')
    })
  }

  return {
    theme,
    fontSize,
    compactMode,
    notificationsEnabled,
    showPreview,
    setTheme,
    setFontSize,
    setCompactMode,
    setNotificationsEnabled,
    setShowPreview,
    init,
  }
})
