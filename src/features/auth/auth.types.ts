import type { UserProfile } from '@/features/users/users.types'

export type AuthState = {
  token: string | null
  refreshToken: string | null
  currentUser: UserProfile | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
}

export type LoginPayload = {
  email?: string
  username?: string
  password: string
}

export type AuthTokens = {
  access_token: string
  refresh_token?: string
}
