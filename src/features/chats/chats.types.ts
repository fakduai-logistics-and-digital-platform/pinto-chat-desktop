export type ChatType = 'direct' | 'group'

export type Chat = {
  chat_id: string
  chat_type: ChatType
  name: string | null
  avatar_url: string | null
  avatar?: string | null
  use_avatar?: { avatar_url?: string | null } | null
  last_message_id: string | null
  last_message_preview: string | null
  last_message_at: string | null
  last_message?: {
    message_id?: string
    content?: string
    created_at?: string
  } | null
  unread_count: number
  is_pinned: boolean
  is_muted: boolean
  is_archived: boolean
  is_deleted: boolean
  participants?: ChatParticipant[]
  created_at: string | null
  updated_at: string
}

export type ChatParticipant = {
  chat_id?: string
  user_id: string
  name?: string
  username?: string
  avatar?: string | null
  use_avatar?: { avatar_url?: string | null } | null
  role?: string | null
  joined_at: string | null
  is_admin: boolean
  is_bot?: boolean
  is_official?: boolean
}

export type ChatListParams = {
  page?: number
  limit?: number
  chat_type?: ChatType
  q?: string
}

export type ChatListResponse = {
  chats: Chat[]
  total?: number
  page?: number
  limit?: number
  pagination?: {
    total?: number
    page?: number
    limit?: number
  }
}
