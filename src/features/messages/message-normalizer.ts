import { v4 as uuid } from '@/shared/utils/uuid'
import type { Message, MessageStatus, MessageType } from './messages.types'

export function normalizeMessage(raw: Record<string, unknown>): Message {
  return {
    message_id: (raw.message_id as string) ?? (raw.id as string) ?? uuid(),
    chat_id: raw.chat_id as string,
    sender_id: (raw.sender_id as string) ?? ((raw.sender as Record<string, unknown>)?.user_id as string) ?? null,
    content: (raw.content as string) ?? null,
    message_type: (raw.message_type as MessageType) ?? 'text',
    reply_to: (raw.reply_to as string) ?? null,
    media_url: (raw.media_url as string) ?? null,
    local_media_path: null,
    file_name: (raw.file_name as string) ?? null,
    file_size: (raw.file_size as number) ?? null,
    mime_type: (raw.mime_type as string) ?? null,
    status: 'sent',
    is_read: (raw.is_read as boolean) ?? false,
    is_deleted: false,
    client_temp_id: (raw.client_temp_id as string) ?? null,
    raw_json: JSON.stringify(raw),
    sent_at: (raw.sent_at as string) ?? null,
    received_at: new Date().toISOString(),
    created_at: (raw.created_at as string) ?? new Date().toISOString(),
    updated_at: new Date().toISOString(),
  }
}

export function createTempMessage(chatId: string, senderId: string, content: string, messageType: MessageType = 'text', replyTo?: string): Message {
  const tempId = uuid()
  return {
    message_id: tempId,
    chat_id: chatId,
    sender_id: senderId,
    content,
    message_type: messageType,
    reply_to: replyTo ?? null,
    media_url: null,
    local_media_path: null,
    file_name: null,
    file_size: null,
    mime_type: null,
    status: 'sending',
    is_read: false,
    is_deleted: false,
    client_temp_id: tempId,
    raw_json: null,
    sent_at: null,
    received_at: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  }
}

export function updateMessageStatus(msg: Message, status: MessageStatus, serverMessageId?: string): Message {
  return {
    ...msg,
    message_id: serverMessageId ?? msg.message_id,
    status,
    updated_at: new Date().toISOString(),
  }
}
