export type MessageType = 'text' | 'image' | 'file' | 'system'
export type MessageStatus = 'queued' | 'sending' | 'sent' | 'delivered' | 'read' | 'failed'

export type Message = {
  message_id: string
  chat_id: string
  sender_id: string | null
  content: string | null
  message_type: MessageType
  reply_to: string | null
  media_url: string | null
  local_media_path: string | null
  file_name: string | null
  file_size: number | null
  mime_type: string | null
  status: MessageStatus
  is_read: boolean
  is_deleted: boolean
  client_temp_id: string | null
  raw_json: string | null
  sent_at: string | null
  received_at: string | null
  created_at: string
  updated_at: string
}

export type PendingOutbox = {
  id: string
  chat_id: string
  client_temp_id: string
  content: string | null
  message_type: MessageType
  reply_to: string | null
  local_file_path: string | null
  status: MessageStatus
  retry_count: number
  last_error: string | null
  created_at: string
  updated_at: string
}

export type SendMessagePayload = {
  chat_id: string
  content?: string
  message_type?: MessageType
  reply_to?: string
  client_temp_id?: string
}
