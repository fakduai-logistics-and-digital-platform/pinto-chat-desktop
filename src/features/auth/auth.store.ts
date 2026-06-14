import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { UserProfile } from '@/features/users/users.types'
import type { QrWsUser } from '@/features/auth/qr-login.service'
import type { LoginPayload } from './auth.types'
import { login as apiLogin, logout as apiLogout } from './auth.api'
import { getProfile } from '@/features/users/users.api'
import { getStoredToken, storeTokens, clearTokens } from './token-storage'
import { env } from '@/app/env'
import { initAuthApi } from './auth.api'
import { initUsersApi } from '@/features/users/users.api'

initAuthApi(env.apiBaseUrl)
initUsersApi(() => useAuthStore().token, env.apiBaseUrl)

function wsUserToProfile(u: QrWsUser): UserProfile {
  return {
    user_id: u.user_id ?? u.id ?? '',
    username: u.username ?? '',
    name: u.name ?? u.username ?? '',
    profile_image: u.profile_image,
  }
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(getStoredToken())
  const refreshTokenVal = ref<string | null>(null)
  const currentUser = ref<UserProfile | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value && !!currentUser.value)

  async function login(payload: LoginPayload) {
    isLoading.value = true
    error.value = null
    const result = await apiLogin(payload)
    if (!result.ok) {
      error.value = result.error
      isLoading.value = false
      return false
    }
    applyTokens(result.data.access_token, result.data.refresh_token)
    await fetchProfile()
    isLoading.value = false
    return true
  }

  function applyTokens(accessToken: string, refreshToken?: string) {
    token.value = accessToken
    refreshTokenVal.value = refreshToken ?? null
    storeTokens(accessToken, refreshToken)
  }

  /**
   * QR login: store tokens issued by backend, then try fetching profile.
   * If profile endpoint fails, fall back to user data from the WS message.
   */
  async function loginWithQrTokens(accessToken: string, refreshToken?: string, wsUser?: QrWsUser) {
    isLoading.value = true
    error.value = null
    applyTokens(accessToken, refreshToken)
    await fetchProfile()
    // Fallback: if profile endpoint failed but WS provided user data
    if (!currentUser.value && wsUser) {
      currentUser.value = wsUserToProfile(wsUser)
    }
    isLoading.value = false
    return !!currentUser.value
  }

  async function fetchProfile() {
    const result = await getProfile()
    if (result.ok) {
      currentUser.value = result.data
    } else {
      currentUser.value = null
      token.value = null
      clearTokens()
    }
  }

  async function logout() {
    if (token.value) {
      await apiLogout(token.value)
    }
    token.value = null
    refreshTokenVal.value = null
    currentUser.value = null
    clearTokens()
  }

  async function initSession() {
    if (!token.value) return false
    isLoading.value = true
    await fetchProfile()
    isLoading.value = false
    return !!currentUser.value
  }

  return {
    token,
    refreshToken: refreshTokenVal,
    currentUser,
    isAuthenticated,
    isLoading,
    error,
    login,
    loginWithQrTokens,
    logout,
    fetchProfile,
    initSession,
  }
})
