import { ApiClient } from '@/shared/api/api-client'
import type { UserProfile } from './users.types'

let client: ApiClient | null = null

export function initUsersApi(getToken: () => string | null, baseUrl: string) {
  client = new ApiClient(baseUrl, getToken)
}

export async function getProfile(): Promise<{ ok: true; data: UserProfile } | { ok: false; error: string }> {
  if (!client) return { ok: false, error: 'API not initialized' }
  return client.get<UserProfile>('/v1/users/profile')
}
