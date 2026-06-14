import { ApiClient } from '@/shared/api/api-client'
import type { AuthTokens, LoginPayload } from './auth.types'

let client: ApiClient | null = null

export function initAuthApi(baseUrl: string) {
  client = new ApiClient(baseUrl, () => null)
}

export async function login(payload: LoginPayload): Promise<{ ok: true; data: AuthTokens } | { ok: false; error: string }> {
  if (!client) return { ok: false, error: 'API not initialized' }
  return client.post<AuthTokens>('/api/auth/login', payload)
}

export async function refreshToken(token: string): Promise<{ ok: true; data: AuthTokens } | { ok: false; error: string }> {
  if (!client) return { ok: false, error: 'API not initialized' }
  return client.post<AuthTokens>('/api/auth/refresh', { refresh_token: token })
}

export async function logout(token: string): Promise<{ ok: true; data: null } | { ok: false; error: string }> {
  if (!client) return { ok: false, error: 'API not initialized' }
  const authedClient = new ApiClient(client['baseUrl'], () => token)
  return authedClient.post<null>('/api/auth/logout')
}
