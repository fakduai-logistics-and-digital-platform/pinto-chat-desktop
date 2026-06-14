import { env } from '@/app/env'
import { getStoredToken } from '@/features/auth/token-storage'

let ws: WebSocket | null = null
let reconnectAttempt = 0
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let intentionalClose = false
let onMessageCallback: ((payload: { type: string; data: Record<string, unknown> }) => void) | null = null
let onStateChangeCallback: ((state: string) => void) | null = null

export function connectWs(onMessage: (payload: { type: string; data: Record<string, unknown> }) => void, onStateChange: (state: string) => void) {
  onMessageCallback = onMessage
  onStateChangeCallback = onStateChange
  doConnect()
}

export function disconnectWs() {
  intentionalClose = true
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  ws?.close()
  ws = null
  onStateChangeCallback?.('disconnected')
}

function doConnect() {
  const token = getStoredToken()
  if (!token) return

  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) return

  onStateChangeCallback?.('connecting')
  intentionalClose = false

  const wsUrl = env.apiBaseUrl.replace(/^http/, 'ws')
  ws = new WebSocket(`${wsUrl}/v1/ws/chat?token=${encodeURIComponent(token)}`)

  ws.onopen = () => {
    onStateChangeCallback?.('connected')
    reconnectAttempt = 0
  }

  ws.onmessage = (event) => {
    try {
      const payload = JSON.parse(event.data)
      onMessageCallback?.(payload)
    } catch { /* ignore */ }
  }

  ws.onclose = () => {
    onStateChangeCallback?.('disconnected')
    if (!intentionalClose) scheduleReconnect()
  }

  ws.onerror = () => {
    ws?.close()
  }
}

function scheduleReconnect() {
  if (reconnectTimer) clearTimeout(reconnectTimer)
  const delays = [1000, 2000, 5000, 10000, 30000]
  const delay = delays[Math.min(reconnectAttempt, delays.length - 1)]
  reconnectTimer = setTimeout(() => {
    reconnectAttempt++
    doConnect()
  }, delay)
}
