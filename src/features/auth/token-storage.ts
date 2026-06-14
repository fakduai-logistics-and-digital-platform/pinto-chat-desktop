const TOKEN_KEY = '_pt_token'
const TOKEN_KEY_LEGACY = 'pinto_access_token'
const REFRESH_KEY = '_pt_refresh_token'
const REFRESH_KEY_LEGACY = 'pinto_refresh_token'

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_KEY) ?? localStorage.getItem(TOKEN_KEY_LEGACY)
}

export function getStoredRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_KEY) ?? localStorage.getItem(REFRESH_KEY_LEGACY)
}

export function storeTokens(access: string, refresh?: string) {
  localStorage.setItem(TOKEN_KEY, access)
  localStorage.removeItem(TOKEN_KEY_LEGACY)
  if (refresh) {
    localStorage.setItem(REFRESH_KEY, refresh)
    localStorage.removeItem(REFRESH_KEY_LEGACY)
  }
}

export function clearTokens() {
  localStorage.removeItem(TOKEN_KEY)
  localStorage.removeItem(TOKEN_KEY_LEGACY)
  localStorage.removeItem(REFRESH_KEY)
  localStorage.removeItem(REFRESH_KEY_LEGACY)
}
